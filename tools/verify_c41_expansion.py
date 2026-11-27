"""Cross-language reference for the frozen C4.1 geometry and overlay hash.

The reference reuses C4.0's already checked Python oracle, not Rust outputs.
Checkpoint goldens are separately checked by Rust and executable replay.
"""
from pathlib import Path
from collections import deque
import argparse,csv,hashlib,json,struct,subprocess
import verify_c40_generator as west

ROOT=Path(__file__).resolve().parents[1]
PACK=json.loads((ROOT/"content/expansion/v1/pineford.json").read_text(encoding="utf-8"))
OVERLAY=hashlib.sha256(b"SH:C41:CONTENT:v1\0"+json.dumps(PACK,sort_keys=True,separators=(",",":"),ensure_ascii=False).encode()).hexdigest()
W,H=54,15
GEO,WATER,ECO,ROAD=(0x3194d2c81ba67f05,0x537dc4f9e6a21b08,0xb5149de803f06a73,0x4ca79b52e1f8d603)
TERRAIN_NAMES=("ground","wall","meadow","woodland","rock","river","road","bridge")
GROUND,WALL,MEADOW,WOOD,ROCK,RIVER,ROADS,BRIDGE=range(8)
GLYPHS=".#,;#~:%"

def distances(tiles,start):
    if tiles[start][0] in (WALL,ROCK,RIVER):return {}
    seen={start:0}
    q=deque([start])
    while q:
        x,y=q.popleft()
        for p in ((x,y-1),(x+1,y),(x,y+1),(x-1,y)):
            if p in tiles and p not in seen and tiles[p][0] not in (WALL,ROCK,RIVER):
                seen[p]=seen[x,y]+1;q.append(p)
    return seen

def reference(seed):
    original=west.reference(seed)
    river=[36+west.random_at(seed,WATER,0,0)%3]
    for y in range(1,H):
        river.append(max(36,min(38,river[-1]+west.random_at(seed,WATER,y,1)%3-1)))
    # The preserved core tile fields are independently derived from the
    # previously verified v1 formulas and the reference terrain glyphs.
    tiles={}
    for y in range(H):
        for x in range(W):
            if x<30:
                if y>=9:tiles[x,y]=[WALL,0,0,0];continue
                glyph=original["terrain_rows"][y][x]
                kind=(WALL if x<=11 or x>=21 else ROCK) if glyph=="#" else GLYPHS.index(glyph)
                elev=west.scalar(seed,west.GEO,x,y)
                near=100-10*min(abs(x-original["river_x_by_row"][y]),10)
                moisture=(3*west.scalar(seed,west.ECO^0x71f9,x,y)+near)//4
                qi=(3*west.scalar(seed,west.ECO,x,y)+moisture)//4
            else:
                elev=west.scalar(seed,GEO,x,y)
                near=100-10*min(abs(x-river[y]),10)
                moisture=(3*west.scalar(seed,ECO^0x7423,x,y)+near)//4
                qi=(3*west.scalar(seed,ECO,x,y)+moisture)//4
                kind=GROUND if x>=43 else RIVER if x==river[y] else ROCK if elev>=63 else WOOD if moisture>=56 else MEADOW
            tiles[x,y]=[kind,elev,moisture,qi]
    first=1+west.random_at(seed,ROAD,0,0)%13
    options=[y for y in range(1,14) if abs(y-first)>=5]
    rows=sorted((first,options[west.random_at(seed,ROAD,1,0)%len(options)]))
    bridges=[[river[y],y] for y in rows]
    for y in rows:
        for x in range(30,43):tiles[x,y][0]=BRIDGE if x==river[y] else ROADS
    for y in range(H):
        for x in (31,42):tiles[x,y][0]=ROADS
    for x in (30,31):tiles[x,4][0]=ROADS
    reachable=distances(tiles,(29,4))
    for p,t in tiles.items():
        if p[0]>=30 and t[0] not in (WALL,ROCK,RIVER) and p not in reachable:t[0]=ROCK
    grove=tuple(PACK["grove"])
    cadence=PACK["regrow_fastest"]+PACK["regrow_step"]*(min(100-tiles[grove][3],99)//25)
    distance=distances(tiles,(29,4))[43,7]
    base_hash=original["digests"]["world"]
    raw=(b"SH:C41:WORLD:v1\0"+struct.pack("<HI",1,seed)+west.CONTENT.encode()+OVERLAY.encode()+bytes.fromhex(base_hash)+struct.pack("<II",W,H)
        +bytes(v for y in range(H) for x in range(W) for v in tiles[x,y])
        +bytes(river)+b"".join(struct.pack("<II",*p) for p in bridges)+struct.pack("<QH",cadence,distance))
    digest=hashlib.sha256(raw).hexdigest()
    return {
        "base_world_sha256":base_hash,"world_sha256":digest,"overlay_sha256":OVERLAY,
        "river_x_by_row":river,"bridges":bridges,"gate_distance":distance,"grove_regrow_every":cadence,
        "terrain_rows":["".join(GLYPHS[tiles[x,y][0]] for x in range(W)) for y in range(H)],
        "tiles":[[dict(zip(("terrain","elevation","moisture","qi"),(TERRAIN_NAMES[t[0]],*t[1:]))) for x in range(W) for t in [tiles[x,y]]] for y in range(H)],
    }

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--exe",type=Path)
    args=parser.parse_args()
    with (ROOT/"tests/fixtures/living/c41_expansion_v1.csv").open(encoding="utf-8",newline="") as f:rows=list(csv.DictReader(f))
    for row in rows:
        seed=int(row["seed"]);ref=reference(seed)
        for key in ("base_world_sha256","world_sha256"):assert ref[key]==row[key],(seed,key)
        assert ref["gate_distance"]==int(row["gate_distance"])
        assert ref["grove_regrow_every"]==int(row["grove_regrow"])
    cases=0
    if args.exe:
        for seed in [int(r["seed"]) for r in rows]+list(range(100,132)):
            proc=subprocess.run([str(args.exe.resolve()),"--expansion-info",str(seed)],capture_output=True,check=True,timeout=15)
            assert not proc.stderr
            actual=json.loads(proc.stdout)
            for key,value in reference(seed).items():assert actual[key]==value,(seed,key)
            cases+=1
    print(json.dumps({"status":"PASS","fixture_seeds":len(rows),"world_digests":len(rows)*2,"overlay_sha256":OVERLAY,"executable_cases":cases}))
if __name__=="__main__":main()
