# M0 Acceptance Tests and Scenario Contract

Version 0.2-draft · 17 November 2026

This is the M0 evidence plan, with the upstream C0.2/C0.3/I0 fixtures below. C0.1 has its own [acceptance matrix](C0_1_ACCEPTANCE_TESTS.md). All implementation evidence remains pending.

## 1. M0 target

One region, two sites, two factions, 20 persistent actors, 30 simulated game days. Include minimal needs, jobs, storage, physical transactions, relationships, events, save/load and domain LOD. The M0 runner stays headless. M0-T04 and M0-T12 use separate I0 integration evidence for client behavior. No full worldgen, advanced cultivation, major war or commercial UI is required.

Hand-author a small fixture. Starting budget is about four occupations, 3–5 jobs, six resource definitions, 8–12 item definitions, several recipes, a limited road graph and one local dispute. The author pins positive `ticks_per_game_day` in the fixture, starts at tick zero and advances exactly 30 times that value using checked arithmetic. The split is 15 times that value. Run a small conversion test with parameters 3 and 7. The 30-day horizons are 90 and 210 and the split ticks are 45 and 105. Reject zero and overflowing horizons. The fixture's signal rules and mappings are reviewed before the first golden run.

## 2. Reproducibility envelope

Record build commit, Rust toolchain, Cargo.lock hash, registry hash, OS/architecture, seed, boundary-indexed admitted command log, time parameters, scenario version and output schema. R1 means the same build/platform/input. Cross-platform bitwise agreement is deferred. Run the 30-day determinism case with seed 100 and smoke with seed 101 as a distinct input. Approve expected activities, balances, causal effects and initial variation before comparing implementation hashes.

## 3. Acceptance matrix

| ID | Scenario/procedure | Required result |
| --- | --- | --- |
| M0-T01 | Run the reviewed 30-day headless scenario three times with seed 100 and identical inputs | Same normalized state bytes/hash and consequential sequence. §5 activity and causality hold on each run |
| M0-T02 | Compare continuous 30 days with 15 days, load, then 15 days. Inspect the split checkpoint before resuming | Match split and final canonical bytes/hash, RNG, tasks, schedule, time parameter, queued commands and event cursor/digest. Run the §4 prefix/suffix oracle |
| M0-T03 | Reload across active task, an in-flight reservation/cargo and a completed faction event | IDs, next actions, cargo and reservations preserved. Faction and earlier events are not re-emitted. Valid completion releases once |
| M0-T04 | Run real paused projection recovery in the separately collected I0-T01 | Zero duplicate projections; complete canonical hash/bytes stay unchanged and later pending inputs act identically |
| M0-T05 | Switch an actor local → scheduled → local while work is in flight, extending C02-T02 | Preserve identity and exact conserved accounting, reservations, delivery counts, eligible task progress and due events. Assert exact results by default. Any approved approximation identifies field, bound and independent comparison |
| M0-T06 | 20 NPCs compete for fewer beds, tools, food and workstations. Include the declared C03-T01 contention ordering case | Expected winner and typed fallback or rejection for each contended fixture. No double reservation or consumption |
| M0-T07 | Transfer actor → store → cargo → site, execute declared recipes, then attempt a recipe with an invalid later leg | Trace each valid debit/credit and transformation. Invalid transaction leaves all balances, reservations and success-event history unchanged |
| M0-T08 | Cancel/disable or expire a reservation on the completion boundary. Unreachable targets fail. Repeat a release and a successful transaction key | Invalidation precedes completion. Release and completed transfer happen at most once, with deterministic reason and no invalid mutation |
| M0-T09 | Load typed-definition fixtures for duplicate namespaced ID, missing reference and invalid enum | Reject before starting with an actionable diagnostic for each case. Check a cycle only if shallow composition is approved. If an optional JSON Schema validator is selected, a remote reference to an unregistered URI fails without I/O |
| M0-T10 | Corrupt save metadata, remove required content, or use incompatible schema | Actionable error without partial install, silent loss or panic |
| M0-T11 | Load the explicit synthetic V1 save into V2 | Apply a documented migration with expected retained IDs/resource counts and known new-field defaults |
| M0-T12 | Compare real I0 host schedules with different render cadence, visibility and projection rebuild during the same admitted replay | I0-T01/T02 evidence on the same admitted batches, with equal canonical bytes/hash and ordered consequential events |
| M0-T13 | Inspect autonomous need/work and a resource dispute in the §5 scenario | Expected cause, relation change, task/transaction, IDs and ordered ticks. Decision traces connect the expected causal chain |
| M0-T14 | Run the 30-day workload with no window/GPU and no runtime network. Check the dependency graph and §1 conversion test | Completes at the selected horizon without renderer dependency. Captures required scheduler and benchmark evidence |
| M0-T15 | Repeat same-seed initial site/actor generation and run a second declared seed | Same inputs produce equal canonical initial bytes. Seed 101 has the authored expected valid variation and all references resolve |

## 4. Save and replay oracle

Before collecting golden output, author and review one scenario whose day-15 split contains a non-empty consequential-event prefix, at least one future admitted command, and in-flight task state. Pick a named command admitted before the split and due after it, such as a scheduled delivery in the second interval. Pin its boundary, target, actor, expected result and expected transaction delta. Compare the loaded state at the split with the uninterrupted reference. Feed both runs the same boundary inputs from the split onward. The queued delivery executes once at its declared tick. Compare the uninterrupted full stream with the retained pre-save prefix followed by newly emitted events after load. No prefix event repeats. The first new event uses the cursor that was persisted, and the end cursor/digest and checkpoint match. This procedure also checks that a buffer in the runner does not supply missing canonical pending work.

Randomized suites use independently declared invariants for ordering, actor interruption, job availability and LOD. Check reference validity, non-negative balances, singleton ownership, reservation capacity, event/trigger timing and exactly-once commits.

## 5. Causality and performance

The M0 scenario owner declares, before running: the named actor and resource for an autonomous need response, a completed work/transfer with expected signed ledger entries, and a single resource trigger with source event and affected faction IDs producing exactly one expected relation delta. For the simple authored dispute, the planned delta is −1; record its units and initial relation in the fixture. Assert event IDs, linked transactions and nondecreasing causal ticks. No need/work event before its declared wake may pass. A focused subfixture idles an actor across at least two earlier ticks, records zero evaluations for that actor before due, and records one at the declared tick.

Request full hashes at designated checkpoints. Preserve every consequential event, and document debug trace sampling while retaining every trace used by the expected chains. Record evaluations/day, wakeups, path calls, resources/events, total wall time, p50/p95 phase time, CPU and peak RSS. Include save size and load/save time where relevant. Record hardware and build profile, and try the low-end laptop if accessible. Register tolerances only after a measured baseline. An optimization needs equivalence and profiling evidence.

## 6. Owned downstream fixtures

### C0.2 — checkpoints and mode changes

- **C02-T01**, owned by the simulation checkpoint maintainer. Adapt the C0.1 slot oracle to the C0.2 versioned format. Save after tick 2 with its two emitted events and a further claim for tick 3 by actor 2, producer 1, sequence 1, admitted at boundary 1. On load preserve pending work, fixture identity, cursor 3, prefix digest and all RNG ordinals. Use an empty boundary-2 batch. Expect exactly event 3, at tick 3, rejecting actor 2 as occupied. The saved prefix plus this event equals the uninterrupted stream. Compare split and final canonical bytes, digest and cursor. Reuse the C0.1 event encoding for this fixture. Event 3 has bytes `03000000000000000300000000000000020000000100000001000000000000000202`; its resulting digest is `1a1cc22bf1041c19845bceb790e6761dad0edaf41cb3eca1f90b0ce06f1875b1` and the next cursor is 4. C0.2 also verifies a fixture ID cursor continues without reuse.
- **C02-T02**, owned by the domain scheduler maintainer. Begin with actors 1 and 2 active, each with one separately reserved synthetic cargo token and one task due at tick 3. An explicit deterministic fixture control switches them to scheduled at tick 2 and back at tick 4. Process mode triggers before due work that tick. At tick 3, each task completes and releases its reservation once, moves one token to its destination, and emits the completion for actor 1 then actor 2. Assert no completion before or after tick 3, stable IDs, two conserved tokens, empty reservations, valid completed tasks and exact event order. Drive the same scheduled input with single steps and as a grouped driver. If an optimization skips idle ticks, compare it to the single-step driver. Unsupported overdue state fails validation. No approximation is used.

### C0.3 — production reservations and transactions

- **C03-T01**, owned by the resource/ledger maintainer. Freeze the fixture's command priority, reservation keys and transaction IDs before writing production behavior. Use one unit of stock with two claimants. Declare the winner and loser diagnostic for forward/reversed input. At an identical completion/expiry tick, and separately completion/cancel and completion/disable, assert no stock transfer and one release. Replay a completion key and repeat release. In another fixture fail a later recipe leg and verify no earlier leg changes. Run the corresponding M0-T06/T07/T08 cases before promoting C0.3. Production fairness has its own ordered fixture.

### I0 — client integration

- **I0-T01**, owned by the bridge maintainer. Start a real simulation with at least one already admitted future command. Pause at the checkpoint and execute 1,000 seeded mixed materialize/dematerialize operations, including repeats on each named actor, camera pans and a complete projection teardown followed by rebuild. Verify no duplicate mappings during the sequence, exactly one projection per visible named actor after rebuild, equality of full canonical bytes/hash, and no change to tick, mode, pending queue or events. Resume with identical explicit inputs across at least three successful steps. Skip one resulting delta, deliver its successor to trigger gap detection and snapshot recovery, then deliver a stale duplicate. Verify current presentation and compare resulting checkpoint and event stream to a control with no projection manipulations.
- **I0-T02**, owned by the host maintainer. Feed the same pre-stamped log under two different frame delivery schedules, including frames with zero or multiple steps. Compare admitted batches, state bytes/hash and consequential events. Separately pause between steps, capture one input while paused, wait and resume. No steps or admissions occur while paused. That input enters the first eligible batch with the earliest target at boundary + 1. No wall-time backlog runs on resume.

## 7. Evidence and failure policy

Record exact commands, scenarios, toolchain, dependency lock, counts and outcome, expected/observed logic, hashes/event excerpts, metrics and hardware, skips, and independent review of the expected results. Unexplained resource creation, duplicate identity, replay divergence or silent migration loss block M0. Capture separate headless and bridge results. Optional visual effects belong to V0/M1.
