# Tri-game performance baseline

**Command root:** `research/tri_game`  
**Binary:** `target/release/tri_game.exe`  
**SHA256 of the final experiment binary:** `1cb90c731d77864eeb2edba515896ca05b4f7b13b271fde13a805fe5868cc708`  
**Kernel rows below** were captured on the immediately previous build of the same measurement code. The only later edit split the schedule timer; it does not change `Simulation::drive`.  
**Raw files:** `research/tri_game/evidence/`

## Clocks, kept separate

| Clock | This session |
|---|---|
| Simulation compute | Unpaced `drive` duration below. No sleep, no 2 Hz wait. |
| Configured tick rate | 2 Hz (500 ms budget), from the accepted C0.4/C0.5 client. Not changed. |
| Rendering frame time | **NOT RE-MEASURED.** The C0.5 record still stands: about 16.67 ms on 31 F12 snapshots, which is not a full frame distribution. |
| Total elapsed time | Process lifetime includes warmup, validation, and JSON. The tables below are drive-call time only, matching the C0.5 rule that output destruction stays outside the sample. |

Nearest rank is `ceil(p * N) - 1` over 300 samples (3 fresh worlds × 100 timed ticks after 10 warmup ticks). Seed 100. Empty command batches.

## Active NPC worlds

These are the same authored populations as `examples/c05_bench.rs`. The 20-actor bytes equal `apps/sealed_playable/assets/demo.c03`. 100 and 1000 are one shared world, not 50 or 5 copies of a process. Topology stays the small canonical map. This is not a crowd or a larger geography.

| World | Median | p95 | Mean | Unpaced drive | Decisions | Final checkpoint | Peak working set |
|---|---:|---:|---:|---:|---:|---:|---:|
| 20 active | 0.2013 ms | 0.3742 ms | 0.2080 ms | 4808.22 /s | 1500 | 110613 bytes | 5.90 MB |
| 100 active | 0.8304 ms | 1.6662 ms | 0.8909 ms | 1122.45 /s | 7500 | 550005 bytes | 9.17 MB |
| 1000 active | 14.5382 ms | 46.7970 ms | 17.7543 ms | 56.32 /s | 75000 | 5493165 bytes | 40.66 MB |
| C0.5 player aggregate, 20 NPCs | 0.4116 ms | 0.8106 ms | 0.4314 ms | 2317.80 /s | not exposed by `World::drive` | 110808 bytes | 5.69 MB |

The 20-actor final hash is `32d87f149099b17d8eef0494afc3e42ce11d38743d8818b788a3ae326d74bedf`, the same hash published in `C0_5_PERFORMANCE_BASELINE.md`. Checkpoint sizes for 20, 100, and 1000 also match that document (110613, 550005, 5493165).

C0.5 player populations of 100 and 1000 are **NOT TESTED**. The binary exits 3 with `status: unavailable`. No scaled player world was invented. Evidence: `evidence/player-100.txt`.

## What the tick time is made of

Dormant rows use the same populations with the first wake moved to tick 100000, so the timed window emits no decisions. That is the empty-tick scan, not a new scheduler.

| Actors | Dormant median | Dormant p95 | Dormant mean | Active mean minus dormant mean |
|---:|---:|---:|---:|---:|
| 20 | 1.000 µs | 1.300 µs | 1.053 µs | 0.2069 ms |
| 100 | 3.700 µs | 4.600 µs | 3.858 µs | 0.8871 ms |
| 1000 | 53.300 µs | 93.300 µs | 59.413 µs | 17.695 ms |

Decision counts are exactly `actors / 4` per tick (75000 / 300 = 250 at 1000 actors), which is the frozen wake interval. Resource events, all kinds, average 29 / 145 / 1450 per tick at 20 / 100 / 1000. Those events include the decision and the ledger traffic around it. The kernel does not publish a separate timer for pathfinding or for one transaction, so those two costs are **not** given a fake split inside `drive`. Experiment B times path search on its own maps. Experiment C times a real ledger haul.

Peak working set is `K32GetProcessMemoryInfo` for that short process. It includes startup and the final checkpoint. It is not a long-soak resident size and it is not the native client's 442 MB working set from C0.5.

## The 1.44 ticks/second observation

`C0_4_FINAL_INDEPENDENT_AUDIT.md` records 21 ticks in 14.558 seconds, about 1.44 ticks/second, on one foreground debug session whose nominal clock was 2 Hz. That wall clock includes focus, input, and capture.

This session's 20-NPC kernel compute is 0.208 ms per tick. The configured presentation budget is 500 ms. Compute for that fixture is a small fraction of one presentation tick. Nothing in these numbers says the kernel is limited to 1.44 ticks/second. Nothing here is claimed as a speedup over C0.5: the 1000-actor mean (17.75 ms) is lower than the C0.5 row (24.09 ms), and the player-20 mean is slightly higher (0.431 ms versus 0.396 ms). The kernel code is the same. The difference is the machine load of each run.

## Reproduce

From `research/tri_game`, after `cargo build --release --offline`:

```powershell
.\target\release\tri_game.exe baseline --actors 20 --ticks 100 --warmup 10 --iterations 3
.\target\release\tri_game.exe baseline --actors 100 --ticks 100 --warmup 10 --iterations 3
.\target\release\tri_game.exe baseline --actors 1000 --ticks 100 --warmup 10 --iterations 3
.\target\release\tri_game.exe baseline --actors 20 --dormant --ticks 100 --warmup 10 --iterations 3
.\target\release\tri_game.exe baseline --actors 100 --dormant --ticks 100 --warmup 10 --iterations 3
.\target\release\tri_game.exe baseline --actors 1000 --dormant --ticks 100 --warmup 10 --iterations 3
.\target\release\tri_game.exe baseline --player --actors 20 --ticks 100 --warmup 10 --iterations 3
.\target\release\tri_game.exe baseline --player --actors 100 --ticks 1 --warmup 0 --iterations 1
```

The last command is expected to exit 3.
