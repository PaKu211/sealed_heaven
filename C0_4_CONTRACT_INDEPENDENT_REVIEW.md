# C0.4 independent contract / architecture review

Date: 18 November 2026  
Reviewer: independent contract-review session; no C0.4 implementation authored  
Verdict: **PASS — contract and architecture only**

**The conditional owner implementation authorization is now effective for this
reviewed scope.** This follows the owner's supplied authorization and the
pre-implementation review condition in the reviewed specification. It does not
authorize C0.5/R2, kernel changes, or broader gameplay work.

**This is not implementation final acceptance.** No C0.4 app, 100-tick Bevy parity
result, native interactive smoke, or performance result is asserted by this report.
Those remain mandatory, followed by a separate independent final review.

## 1. Reviewed tree and contract identity

- Worktree: `D:\projek\sealed\Sealed_Heaven_C0_1_Source (1)\sealed-heaven-c04`.
- Branch observed: `feat/c04-first-playable`.
- HEAD and accepted baseline observed:
  `14d229bedc308d7e50225122caaec0ce5732a201`.
- `git diff --name-only <baseline>` returned no tracked changes. The C0.4
  contracts/progress files were untracked at review start; they were not confused
  with baseline files. No `apps/sealed_playable` implementation existed.
- Reviewed contract versions: both `1.0-contract`, 18 November 2026.
- SHA-256 of `docs/C0_4_IMPLEMENTATION_SPEC.md`:
  `c4827e8375888c5026e3e0e15da5c687787860f9c965a633209e348e74d18a3f`.
- SHA-256 of `docs/C0_4_ACCEPTANCE_TESTS.md`:
  `7351aa4bece96d4db28fcc330db902983382169fcea9c3a09386bdbb6167139e`.

The conclusions below come from the contracts and existing source/tests, not
previous implementation reports, audit verdicts, or handoff claims. Material
changes to the reviewed scope or authority boundary require renewed review.

## 2. Findings and feasibility

### CR-01 — Frozen public API is sufficient; no extension needed

`src/lib.rs:648–664,805–815` exposes domain checkpoint bytes/hash, restore, and
`drive(current_tick, batch)`. Tick, event cursor, and digest are public at
`597–599,624–630`. `ActorId::new`, `Command`, `SetMode`, and `SetEnabled` support
the required comparison schedule without adding UI gameplay commands.

There is no public twenty-actor constructor or complete fixture-3 actor/task view.
The proposed authored checkpoint plus read-only schema decoder genuinely resolves
this limitation. `World`/`Actor`/`Task` are crate-private, not an API an app can
import. Fixture-1/2 enabled/mode/task accessors are not valid fixture-3 views:
they inspect compatibility fields or two-actor indexing (`lib.rs:601–603,
684–728`; `fixture3.rs:247–273`). The specification explicitly prohibits that
shortcut. **No blocking API contradiction.**

### CR-02 — The exact 20-actor, five-goal demo exists in source

`src/tests/c03_s4.rs:321–403` constructs exactly the initial state described in
C0.4 §3, not the different scripted M0-T06 contention fixture earlier in that file.
`World::standard` and `insert_actor` (`fixture3.rs:135–196`) supply the remaining
defaults: wake interval 4, active/enabled actors, initial site 1, fixed topology,
grid and holders. Initial cursors are actor 21, record 1, transaction 1, stock
103, event 1; digest is H0. Actor-owned stock IDs equal their actor IDs.

Reading `commit_goal` and `step_task` gives the following first-task boundaries
for every group, absent additional commands:

| Group / goal | Tick 1 | Tick 2 | Tick 3 | Tick 4 |
| --- | --- | --- | --- | --- |
| IDs 1,6,11,16 / eat | Executing | Completing | Done | remains Done |
| IDs 3,8,13,18 / work | Executing | Completing | Done | remains Done |
| Rest / haul / return groups | Travelling | Executing at site 2 | Completing | Done |

Completion clears the actor's current task ID; terminal task records remain.
Food is debited once; durable stocks 101/102 stay quantity 4; haul credits sum
to four units of definition 7 at home site 2. The named existing test asserts all
20 first tasks complete, durable release, positions, and haul quantity, and passed
in this review. **The four-tick requirement is achievable without invented rules.**

Stored channels do not decay or automatically decrease on task completion. After
food/cargo exhaustion, later choices can differ from the first five groups; there
is no requirement or authorization to force those goals to recur. Cost-1 edges
arrive within one later work tick, not a visible multi-frame physics simulation.
The contract correctly acknowledges both limitations.

### CR-03 — Projection is a decoder, not a second simulation

C0.3 specification §§2–4 and `fixture3.rs:1821–2602` agree on revision-1 layout:
scenario/hash, actors, topology, stocks, reservations, variable-length tasks and
paths, transactions, registry, pending commands, traces, cursor/digest, checksum.
All requested inspector fields are present. No private Rust memory layout is
needed. App bounds and checked reads must apply to variable counts, nested lists,
arithmetic, and unsupported tags/revisions, not just actor records.

Kernel restore accepts only the frozen scenario forms (wake 4 or 100 and exact
scenario bytes, `fixture3.rs:2400–2407`); the proposed wake-4 fixture fits. Its
scenario hash is not a separate hash of the topology or all initial records.
The packer therefore must verify the complete authored fixture, not merely this
content hash. C04-T02 requires that independent repack and roundtrip.

Projection must only observe validated canonical records. It must not select
goals, search paths, infer stock transfers, advance stages, or write changes back.
Authored coordinates and deterministic visual offsets are permissible rendering
metadata. **The declared boundary is feasible and preserves one gameplay authority.**

### CR-04 — Determinism and failure semantics are testable as written

Fixture-3 drive stages a clone and publishes only after all phases succeed
(`fixture3.rs:276–359`). It enforces producer 1 and known actors. Resource events
are returned by `DriveOutput::resource_events()`, whereas `events()` is empty for
fixture 3 (`lib.rs:810–815`). Comparison and inspector code must use the former
and `ResourceEvent::canonical_bytes()`, preserving returned order.

An integer 500,000,000 ns accumulator and eight-tick-per-frame cap can retain
debt. Clearing debt on explicit pause/load/reset is a specified control action,
not silent loss during ordinary catch-up. FPS equality is correctly qualified by
equal admitted tick/command boundaries: unequal wall-clock UI timings need not
produce equal worlds. Bevy plugin tests must observe every successful tick even
when one frame drives several; comparing only final frame state is insufficient.
**No time or parity contradiction.** The existing four-tick/grouping tests are
supporting kernel evidence, not a substitute for the new 100-tick plugin tests.

### CR-05 — Persistence and localization belong entirely in the app

Public restore dispatches multiple fixtures (`lib.rs:659–664`), so kernel restore
success alone cannot satisfy wrong-fixture rejection: the app must also enforce
its supported fixture/layout before replacing live state. Restore and projection
can validate a candidate while retaining the current simulation. The public file
helpers use unbounded ordinary read/write; they do not supply C0.4's 16 MiB bound
or replacement safety. App-owned bounded I/O and safe sibling-file replacement
are required. On Windows, delete-old-then-rename is not a safe substitute for
replacement; failure must preserve the prior save and report failure.

Flat UTF-8 JSON and required-key checks need no kernel changes. Duplicate-key
detection must precede lossy conversion into a normal map. Strict completeness
of shipped dictionaries and defensive runtime fallback/unknown-key diagnostics
are compatible requirements. Language and error formatting must not enter
canonical state. **No persistence/localization blocker; these are implementation
obligations, not already delivered behavior.**

### CR-06 — Dependency isolation and exit requirements are coherent

The root is an ordinary edition-2024 package with only pinned `rand_chacha` and
`sha2`. An independent nested package with path dependency `../..` can use its own
manifest/lock without making the root a workspace or introducing Bevy into it.
A pinned minimal-feature Bevy desktop build on unchanged Rust 1.100.0 is a
plausible implementation choice; the actual release/features/transitive lock
must still be selected and built. No dependency resolution or GPU availability
was proved in this contract-only review. Normal Bevy font/asset support needed
by text/UI is not authorization for a separate content-authoring pipeline.

C04-T01–T09 preserve frozen checks, require actual Bevy integration, per-tick
100-tick parity, real file failures, external localization checks, native build,
interactive smoke and modest measured observations. A screenshot alone does not
prove all interactive actions. Desktop/GPU/input inability remains incomplete,
not a replacement headless PASS. Final independent review and remote SHA evidence
remain exit work. **No material contradiction requiring contract amendment found.**

## 3. Evidence executed by this reviewer

Commands ran from the reviewed worktree using the supplied Git/Cargo executables
(Python from PATH). All completed successfully:

| Command | Observed result |
| --- | --- |
| `git status --short --branch`, `git rev-parse HEAD` | branch/baseline as above; only pre-existing untracked C0.4 documents |
| `git diff --name-only 14d229bedc308d7e50225122caaec0ce5732a201` | empty tracked diff |
| `cargo --version` | `cargo 1.100.0 (3acc79c0c 2026-11-05)` |
| `cargo fmt --all --check` | exit 0 |
| `cargo check --locked --offline` | exit 0 |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0 |
| `cargo test --locked --offline` | 82 unit tests + 2 doctests passed |
| `cargo test --release --locked --offline` | 82 unit tests + 2 doctests passed |
| `python docs/tools/c03_vectors.py --check` | contract reference verified without simulation output |

These are baseline feasibility/regression observations, not C0.4 row closure.
The reviewer added only this report; no gameplay/app code or frozen file was
changed, and no commit/push was performed by this reviewer.

## 4. Exact files examined

Read directly (complete unless ranges are given):

- `docs/C0_4_IMPLEMENTATION_SPEC.md`
- `docs/C0_4_ACCEPTANCE_TESTS.md`
- `docs/C0_3_IMPLEMENTATION_SPEC.md`
- `docs/C0_3_ACCEPTANCE_TESTS.md`
- `docs/C0_3_VECTOR_REFERENCE.txt`
- `docs/tools/c03_vectors.py`
- `Cargo.toml`
- `Cargo.lock`
- `rust-toolchain.toml`
- `src/lib.rs`
- `src/fixture3.rs`
- `src/fixture3/validation.rs`
- `src/fixture3/navigation.rs`
- `src/tests/c03_s2.rs`
- `src/tests/c03_s3.rs`
- `src/tests/c03_s4.rs`
- `src/tests/c03_remediation.rs` (lines 1–350, 385–509, 780–884).

The entire tracked tree was compared against the baseline; this is not a claim
to have manually read every C0.1/C0.2 test or normative document. Root tests
executed their regressions. Prior reports were not used as evidence.

## 5. Authorized boundary and remaining gates

Proceed only with the reviewed native Bevy 2D presentation app, authored exact
20-NPC checkpoint/independent packer, read-only views, controlled tick adapter,
pause/step/reset, safe save/load, external en/id resources, required tests,
modest performance observations, interactive evidence, and delivery documentation.

Do not change C0.1–C0.3 source/API/schema/vectors/dependencies/toolchain; introduce
another gameplay simulator, decay/resupply/new routes, automatic mode policies,
cohorts, migrations, networking/3D/audio scope; or merge to main. Bevy selection
and implementation must stay inside the separate app package.

**Contract gate: PASS. Implementation acceptance: not reviewed/not granted.**
