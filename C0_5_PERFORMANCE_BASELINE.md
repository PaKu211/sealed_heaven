# C0.5 reproducible performance tooling

**C05/scaled timing/native baseline: NOT YET RUN.** One standalone 20-NPC tooling
smoke measurement is recorded below, not a C05 acceptance result or hardware
scalability guarantee. Core/app work is concurrent; run the full matrix after it
is stable. No profiling dependency was added.

## Run

From the repository root (PowerShell):

```powershell
cargo test --locked --example c05_bench
cargo build --release --locked --example c05_bench
& .\target\release\examples\c05_bench.exe --mode npc --actors 20 --ticks 100 --warmup 10 --iterations 3
& .\target\release\examples\c05_bench.exe --mode npc --actors 100 --ticks 100 --warmup 10 --iterations 3
& .\target\release\examples\c05_bench.exe --mode npc --actors 1000 --ticks 100 --warmup 10 --iterations 3
& .\target\release\examples\c05_bench.exe --mode c05 --actors 20 --ticks 100 --warmup 10 --iterations 3
.\tools\c05_performance.ps1 -OutputDirectory "$env:LOCALAPPDATA\Temp\opencode\c05-perf"
```

The wrapper builds once in release, then waits synchronously for each child.
Use `-SkipBuild` to reuse a known build; `-BenchmarkExe <path>` supports a custom
target directory. Keep the machine otherwise idle and note foreground/background
state, power policy and competing workloads. Do not compare debug and release.
Wrapper defaults: both modes, 20/100/1000 actors, 100 timed ticks, 10 warmup,
3 independent iterations. `--help` lists the Rust CLI. Invalid input exits 2;
unsupported C05 sizes produce `status: unavailable` JSON and exit 3, not timings.
On this host, direct script execution was blocked by the default execution
policy. A single-process invocation works without changing persistent policy:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/c05_performance.ps1 -OutputDirectory "$env:LOCALAPPDATA\Temp\opencode\c05-perf"
```

## Genuine fixture and limits

`examples/c05_bench.rs` includes immutable
`apps/sealed_playable/assets/demo.c03` for the actual 20-NPC workload. Its offline
initial-state authoring helper transcribes `apps/sealed_playable/tools/pack_demo.py`
and `docs/tools/c03_vectors.py`. Before ANY measurement it requires its authored
20-NPC bytes to equal that asset exactly. It neither alters production bytes nor
implements task selection, movement, reservations, work or stock transactions.

The C03 format's content hash covers the frozen scenario block, not a fixed
actor count (`src/fixture3.rs::decode_body`); semantic validation accepts sorted,
referentially valid arbitrary actor IDs with a higher next-actor cursor
(`src/fixture3/validation.rs`). Thus 100/1000 are authored as ONE actual kernel
world, not repeated unrelated simulations or multiplication of a 20-NPC timing.

Scaled scenario: 5/50 cohorts of the same 20 initial actors. Actor IDs are 1..N,
enabled/active, first wake 1, roles/relationship 0, at site 1. Five equally sized
groups strongly prefer Eat, Rest, Work, Haul and ReturnHome (one matching need
channel = 1000). Home is site 2 for Rest/Haul/ReturnHome, otherwise site 1.
Each eater owns one food unit; each hauler owns one definition-7 unit. Shared
tool/station stocks at site 1 have capacity N/5 (four per cohort), IDs N+81/N+82.
Stock count is 2N/5+2; cursors are N+1, 1, 1, N+83. Topology, seed 100, recipe,
grid, wake interval 4 and all other initial fields remain canonical. Topology is
small/shared and NPCs may share cells as permitted by the frozen kernel: this is
NOT a larger map, richer AI, collision/crowd simulation or production population.
Food/haul stocks are finite and depleted normally; no replenishment is invented.

Preflight checks initial stock ownership-derived IDs/quantities, kernel restore
and exact canonical roundtrip. An untimed clone advances one real tick and must
emit exactly N decisions in actor order, with each actor's authored preferred
goal, score 1,000,000, wake 5 and analytically expected task ID. The first-tick
checkpoint must restore/roundtrip too. Every final measured checkpoint must also
restore/roundtrip. These checks establish the measured fixture's admission and
initial oracle, not independent certification of all future behavior.

Executed tooling verification: `cargo test --locked --example c05_bench` passed
all 5 tests (immutable fixture equality, all-size first-tick oracles, 100/1000
continuation through tick 12 plus equal tick-13 replay after restore, nearest-rank
statistics, invalid/zero argument rejection). Release build succeeded.
`python apps/sealed_playable/tools/pack_demo.py --check` confirmed the immutable
asset. PowerShell AST syntax, nearest-rank/empty summary and dotted/missing
property helper checks passed using in-memory test inputs (not native evidence).
The wrapper's C05/100 unavailable
path was exercised with `-Ticks 1 -Warmup 0 -Iterations 1`; it preserved the
child's exit 3/status unavailable without making a C05 drive call. Evidence:
`C:\Users\PC_4\AppData\Local\Temp\opencode\c05-perf-unavailable-tool-smoke`.

C05 uses only `World::settlement()`, checks its embedded NPC bytes equal the
immutable asset, and calls the full `World::drive(boundary, &[])`. Its internal
aggregate clone, embedded kernel, validation and any internal serialization are
included. It is a no-player-input baseline, not an active-command/registry-growth
stress test. The authored C05 settlement has NPCs 1–20 and player 21; no supported
scaled aggregate constructor exists. **C05 100/1000 are unavailable**, even though
standalone C03 NPC 100/1000 can be validated. No C05 checkpoint is rewritten to
pretend otherwise. No new fixture file or production edit is needed.

## Metrics and interpretation

- Each iteration starts at tick 0; warmup advances that SAME world. Exactly
  `ticks` further commits are timed; there is no pacing, sleeping or parallelism.
  Final tick is `warmup + ticks`. Iterations are separate fresh worlds, not an
  unsupported method of manufacturing a larger actor population.
- `drive_ns` preserves every raw `Instant` duration. p50/p95/p99 use nearest rank
  `ceil(p*N)-1`; mean/max and total are supplied per run and pooled. The public
  drive call alone is timed; returned output destruction, loop bookkeeping,
  external checkpoint snapshots, validation probes, restore and JSON output are
  outside that duration. Timer overhead is not subtracted.
- `unpaced_drive_tps = samples * 1e9 / total_drive_ns`, NOT configured cadence.
  `loop_wall_tps` also includes per-step harness bookkeeping/output destruction.
  Neither is renderer FPS or an end-user throughput guarantee.
- Initial/post-warmup/final checkpoint sizes, measured/total byte growth, initial
  and final hashes, embedded NPC hash/digest and measured NPC event count provide
  state/workload context. Checkpoint growth can increase clone/scan cost across
  the run; these measurements must not be extrapolated as constant long-run cost.
- `environment.json` records binary SHA256, toolchain, OS, CPU identifier, logical
  processors and Git status (including concurrent modifications). Process summary
  records wall seconds, CPU seconds, CPU % relative to one core AND the machine,
  and observed OS peak working set. Live memory queries use synchronous timed
  `WaitForExit` (20 ms by default, `-MemorySampleIntervalMs` configurable), not a
  background job. Entire-child CPU/memory include warmup, validation,
  restore, checkpoints and output; they are NOT drive-only resource attribution.
  If exit peak accounting is unavailable, `peak_working_set_complete` is false;
  the observed live peak is only a lower bound, with observation count/time and
  a note. If no observation exists, the field is null. Missing CPU accounting is
  likewise null, never fabricated. CPU >100% of one core is not an error.

## Actual standalone NPC20 tooling smoke (not C05 baseline)

2026-11-18, Windows 10.0.26200, Intel64 Family 6 Model 151 Stepping 2, 20 logical
processors, Rust 1.100.0 MSVC release. Concurrent C05 development/build work means
this is **not an idle, matched native comparison**. The wrapper ran successfully
with `-SkipBuild -Mode npc -Actors 20 -Ticks 100 -Warmup 10 -Iterations 1`.
Raw evidence (local temporary files, not checked into the repository):
`C:\Users\PC_4\AppData\Local\Temp\opencode\c05-perf-npc20-tool-smoke-v2`.

| Metric | Actual result |
|---|---:|
| Timed ticks / warmup / independent iterations | 100 / 10 / 1 |
| Drive mean / p50 | 0.211523 / 0.213200 ms |
| Drive p95 / p99 / max | 0.371500 / 0.382900 / 0.391100 ms |
| Total timed drive / unpaced drive TPS | 21.1523 ms / 4727.618273 |
| Final tick / measured NPC events | 110 / 2900 |
| Initial / post-warmup / final checkpoint | 3657 / 21413 / 110613 bytes |
| Measured checkpoint growth | 89200 bytes |
| Child wall / CPU time | 0.0389231 / 0.03125 seconds |
| Child CPU (% one core / % whole machine) | 80.2865 / 4.0143 |
| Observed working-set peak (one live observation) | 4751360 bytes, **lower bound** |

Exit working-set accounting was unavailable; the sample at wall 0.0198763 seconds
does not establish the complete lifetime peak. CPU timing is coarse for this
short child. Final checkpoint SHA256:
`32d87f149099b17d8eef0494afc3e42ce11d38743d8818b788a3ae326d74bedf`.
The initial fixture SHA256 is
`f6a4be4d54a82e6400c31fd0a4c241c638399a1e1961b23eeddbbe6cba0dddad`.
An earlier smoke in the sibling `c05-perf-npc20-tool-smoke` folder identified the
host's missing exit memory data; v2 explicitly handles and labels it. No native
JSON was supplied; `native-summary.json` correctly reports not measured.

## Native rendering evidence (separate)

Supply real native JSON observations (object/array, `{ "samples": [...] }`, or
JSONL) containing `frame_time_ms` and `entity_count`, or specify dotted property
paths matching the client's actual schema:

```powershell
.\tools\c05_performance.ps1 -SkipBuild -Mode c05 -Actors 20 -NativeJson C:\evidence\native-samples.jsonl -OutputDirectory "$env:LOCALAPPDATA\Temp\opencode\c05-native-perf"
# Example alternate schema: -FrameTimeProperty metrics.frame_time_ms -EntityCountProperty metrics.entity_count
```

`native-summary.json` aggregates supplied frame intervals/entity observations,
reports sample counts/mean/p50/p95/p99/max and source hashes. Without observations
it explicitly says not measured. Do not treat an occasional snapshot's latest
frame interval as a complete per-frame distribution or GPU time. Record native
build/profile, capture method, foreground state, duration, warmup, observation
cadence and input/capture overhead alongside evidence. Headless process resource
metrics do NOT stand in for native CPU/memory; measure the native process over an
identified interval separately. Native rendering performance is **not yet run**.

## C04 cadence comparison

`C0_4_FINAL_INDEPENDENT_AUDIT.md` reports 21 ticks / 14.558 seconds = approximately
1.44 observed TPS for one foreground **debug** session, including input/capture
overhead after an earlier unfocused catch-up. It was nominally configured at
2 Hz (500 ms/tick). The configured rate is host pacing, while observed wall TPS
also includes scheduling, rendering, input, captures and pauses/focus effects.
That sample cannot isolate frozen-kernel compute time or prove a kernel ceiling
of 1.44 TPS. Compare the new drive duration/unpaced TPS separately; then collect
a matched native sample before attributing any difference to C05 or claiming a
speedup/regression. This benchmark does not change the nominal 2 Hz policy.
