"""Reference implementation of the frozen C4.0 generator contract.

Uses Python integers and independent traversal logic, never imports Rust
outputs except the golden CSV / optionally an executable manifest to compare.
This is a cross-language oracle authored in the same development session,
not an independent human audit.
"""
from pathlib import Path
from collections import deque
import argparse
import csv
import hashlib
import json
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[1]
PACK = json.loads((ROOT / "content/v1/sealed_heaven.json").read_text(encoding="utf-8"))
CONTENT = "5d33f089bf1086ed4e9465f4ef3be8254b8090c6a35fcbdf9a2e9f9bb0451b80"
WIDTH, HEIGHT = 30, 9
GEO, HYDRO, ECO, ROADS = (0x2dd390e9a53911b1, 0x85d161372566bc2f, 0xa06477cf521e6a8b, 0x6bfa390e88443e91)
MASK = (1 << 64) - 1
GROUND, WALL, MEADOW, WOOD, ROCK, RIVER, ROAD, BRIDGE = range(8)
GLYPHS = ".#,;#~:%"

def random_at(seed, domain, x, y):
    z = domain ^ ((seed * 0x9e3779b97f4a7c15) & MASK) ^ (x << 32) ^ y
    z = ((z ^ (z >> 30)) * 0xbf58476d1ce4e5b9) & MASK
    z = ((z ^ (z >> 27)) * 0x94d049bb133111eb) & MASK
    return z ^ (z >> 31)

def scalar(seed, domain, x, y):
    gx, dx = divmod(x, 4)
    gy, dy = divmod(y, 3)
    return sum(
        (random_at(seed, domain, gx+i, gy+j) % 101) * wx * wy
        for i, wx in [(0, 4-dx), (1, dx)]
        for j, wy in [(0, 3-dy), (1, dy)]
    ) // 12

def digest(label, seed, raw):
    return hashlib.sha256(b"SH:C40:" + label + b"\0" + struct.pack("<HI", 1, seed) + raw).digest()

def path_distances(kind, start):
    if kind[start] in {WALL, ROCK, RIVER}:
        return {}
    out = {start: 0}
    queue = deque([start])
    while queue:
        x, y = queue.popleft()
        for xy in [(x+1,y),(x-1,y),(x,y+1),(x,y-1)]:
            if xy in kind and xy not in out and kind[xy] not in {WALL, ROCK, RIVER}:
                out[xy] = out[(x,y)] + 1
                queue.append(xy)
    return out

def reference(seed):
    cells = [(x,y) for y in range(HEIGHT) for x in range(WIDTH)]
    terrain = next(m["rows"] for m in PACK["maps"] if m["id"] == ("sealed_heaven:lower_map" if seed & 1 else "sealed_heaven:ridge_map"))
    height = {xy: scalar(seed,GEO,*xy) for xy in cells}
    river = [15 + random_at(seed,HYDRO,0,0)%3]
    for y in range(1,HEIGHT):
        river.append(max(15,min(17,river[-1]+random_at(seed,HYDRO,y,1)%3-1)))
    moisture = {xy:(scalar(seed,ECO^0x71f9,*xy)*3 + 100-10*min(abs(xy[0]-river[xy[1]]),10))//4 for xy in cells}
    qi = {xy:(scalar(seed,ECO,*xy)*3+moisture[xy])//4 for xy in cells}
    kind={}
    for x,y in cells:
        xy=(x,y)
        if x<=11 or x>=21:
            kind[xy]=GROUND if terrain[y][x]=="." else WALL
        elif x==river[y]:
            kind[xy]=RIVER
        elif height[xy]>=63:
            kind[xy]=ROCK
        else:
            kind[xy]=WOOD if moisture[xy]>=56 else MEADOW
    a=1+random_at(seed,ROADS,0,0)%7
    other=[y for y in range(1,8) if abs(y-a)>=3]
    rows=sorted([a,other[random_at(seed,ROADS,1,0)%len(other)]])
    bridges=[(river[y],y) for y in rows]
    for y in rows:
        for x in range(12,21):
            kind[x,y]=BRIDGE if x==river[y] else ROAD
    road_bytes=bytes(v for bridge in bridges for v in bridge)+b"".join(
        struct.pack("<HB",i,kind[xy]) for i,xy in enumerate(cells) if kind[xy] in {ROAD,BRIDGE}
    )
    connected=path_distances(kind,(0,4))
    for x,y in cells:
        if 11<x<21 and kind[x,y] not in {WALL,ROCK,RIVER} and (x,y) not in connected:
            kind[x,y]=ROCK
    west=8+4*(min(100-qi[9,2],99)//25)
    east=12+4*(min(100-qi[27,2],99)//25)
    gate=path_distances(kind,(11,4))[21,4]
    stages={
        "geography":digest(b"GEO",seed,bytes(height[xy] for xy in cells)),
        "hydrology":digest(b"HYDRO",seed,bytes(river)),
        "ecology":digest(b"ECO",seed,bytes(v for xy in cells for v in (moisture[xy],qi[xy]))),
        "roads":digest(b"ROADS",seed,road_bytes),
    }
    world=CONTENT.encode()+struct.pack("<II",WIDTH,HEIGHT)+bytes(
        v for xy in cells for v in (kind[xy],height[xy],moisture[xy],qi[xy])
    )+bytes(river)+road_bytes+struct.pack("<QQH",west,east,gate)+b"".join(stages.values())
    stages["world"]=digest(b"WORLD",seed,world)
    return {"digests":{name:value.hex() for name,value in stages.items()},
            "river_x_by_row":river,"bridges":[list(p) for p in bridges],
            "gate_distance":gate,"west_regrow_every":west,"garden_regrow_every":east,
            "terrain_rows":["".join(GLYPHS[kind[x,y]] for x in range(WIDTH)) for y in range(HEIGHT)]}

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--exe",type=Path,help="Optional frozen C4.0 binary to check")
    args=parser.parse_args()
    with (ROOT/"tests/fixtures/living/c40_generator_v1.csv").open(encoding="utf-8",newline="") as file:
        rows=list(csv.DictReader(file))
    for row in rows:
        seed=int(row["seed"])
        ref=reference(seed)
        for name,value in ref["digests"].items():
            assert value==row[name+"_sha256"],(seed,name)
        for field,key in [("gate_distance","gate_distance"),("west_regrow_every","west_regrow"),("garden_regrow_every","east_regrow")]:
            assert ref[field]==int(row[key]),(seed,field)
    cases=0
    if args.exe:
        for seed in [int(row["seed"]) for row in rows]+list(range(32,64)):
            result=subprocess.run([str(args.exe.resolve()),"--worldgen-info",str(seed)],capture_output=True,check=True,timeout=10)
            assert not result.stderr
            actual=json.loads(result.stdout)
            ref=reference(seed)
            for field,value in ref.items():
                assert actual[field]==value,(seed,field)
            cases+=1
    print(json.dumps({"status":"PASS","fixture_seeds":len(rows),"stage_digests":len(rows)*5,"executable_cases":cases}))

if __name__=="__main__":
    main()
