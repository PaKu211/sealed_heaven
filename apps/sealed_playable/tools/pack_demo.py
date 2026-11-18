"""Author only the frozen C0.3 S4 mixed-goal fixture's initial checkpoint.

No kernel or simulation output is called. Field order follows C03 spec section 3.
Run --check to compare bytes, or --write to initialize this C04-only asset.
"""
import importlib.util
import pathlib
import sys

sys.dont_write_bytecode = True

ROOT = pathlib.Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("vectors", ROOT / "docs/tools/c03_vectors.py")
v = importlib.util.module_from_spec(spec)
spec.loader.exec_module(v)


def demo():
    actors, stocks = [], []
    for index in range(1, 21):
        goal = (index - 1) % 5
        channels = [1000 if channel == goal else 0 for channel in range(5)]
        home = 2 if goal in (1, 3, 4) else 1
        actors.append(v.p("IBBQ", index, 1, 1, 1) + v.p("5I", *channels)
                      + v.p("6q", *([0] * 6)) + v.u64(home, 0)
                      + v.p("BQIIQQ", 1, 1, 0, 0, 0, 0))
        if goal in (0, 3):
            stocks.append(v.stock(index, 1 if goal == 0 else 7, 1, 1, 1, index))
    stocks += [v.stock(101, 4, 2, 4, 3, 1), v.stock(102, 5, 2, 4, 3, 1)]
    return v.checkpoint(4, 0, actors, (21, 1, 1, 103), stocks, [], [], [])[1]


if __name__ == "__main__":
    target = pathlib.Path(__file__).resolve().parents[1] / "assets/demo.c03"
    data = demo()
    if sys.argv[1:] == ["--write"]:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    elif sys.argv[1:] == ["--check"]:
        if target.read_bytes() != data:
            raise SystemExit("demo fixture mismatch")
    else:
        raise SystemExit("usage: pack_demo.py --check|--write")
    print(f"C0.3 S4 mixed fixture: {len(data)} bytes SHA256 {v.sha(data).hex()}")
