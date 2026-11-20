# C0.2 Architecture Review

Version 0.1-review-r1.1 · 17 November 2026  
Status: G1 conditional-pass polish. C0.2 code is not authorized.

This review records the repository baseline and the corrections made after the independent G1 audit returned FAIL. The audit findings were supplied in the remediation assignment. They were checked against this specification set, the frozen `Event` API in `src/lib.rs`, and the G0 milestone boundaries. No kernel file was edited.

## 1. Repository inspection

The workspace is one library package, `sealed_sim` 0.1.0, edition 2024. `rust-toolchain.toml` pins Rust 1.100.0 with rustfmt and Clippy. Direct dependencies are `rand_chacha` exactly 0.10.0, default features off, and `sha2` exactly 0.11.1. `Cargo.lock` SHA-256 is `176e4bb5d145739c101c10296ce9f97d4a24917381be057e74134e0ba1779d75`, the same digest recorded in the C0.1 implementation report.

The simulation source is `src/lib.rs`, `src/encoding.rs`, and `src/tests/`. There is no `sealed_app`, content crate, or Bevy dependency. `Simulation` holds a fixture config, tick, slot owner, pending command map, four RNG ordinals, an event cursor, and an event digest. `checkpoint_bytes()` builds the in-memory `SH:C01:STATE` revision-1 observation. Nothing reads that observation back. There is no processing mode, allocator, cargo, task, reservation, or file I/O.

The G0 documents under `docs/` plus `G0_REVIEW_REPORT.md`, `G0_REMEDIATION_REPORT.md`, and `G0_CHANGELOG.md` are the approved architecture baseline named by archive SHA-256 `54398621beb611f4eccee3ba26df9ebf61e0b0ff33dd66b12e99460b984e9587`. The C0.1 implementation report and `C0_1_VERIFICATION_LOG.txt` record a Linux pass of C01-T01 through C01-T06. This tree does not contain a separate independent C0.1 audit file. The implementation report itself still says independent sign-off was pending when it was written. The handoff for this task states that the C0.1 final technical gate passed. The source matches the C0.1 specification, and the local suite below passes. Those facts support treating the kernel as the frozen baseline.

`README.md` still describes the G0 pack before implementation: R4 pending, and no implementation gate run. That sentence is stale. This review does not edit the README, because the authorized documentation change is the three C0.2 documents.

### What is already implemented

| Requirement | Where it stands |
| --- | --- |
| Deterministic tick boundary, batch admission, command key order, atomic rollback | Implemented and covered by C01-T01 through C01-T03 |
| Slot contention, disabled actors, probe ordinals, ChaCha12, event digest | Implemented and covered by C01-T02 through C01-T05 |
| Canonical in-memory checkpoint bytes | Implemented as an observation. Not a save |
| Stable actor IDs 1 and 2 | Fixture constants. No allocator |
| Headless crate, pinned toolchain, no Bevy | Implemented |

### What belongs to C0.2

ADR-003, ADR-009, ADR-017, ADR-022, Core §11, Technical Architecture §8, LOD §2–§4, and M0 §6 assign C0.2 exactly two executed proofs: C02-T01 checkpoint resume, and C02-T02 scheduled actors with exact cargo accounting. Both are specified only as oracles today. Neither exists in `src/`.

### What stays deferred

C0.3 owns needs, jobs, production reservations, expiry, and fairness. V0 and I0 own presentation and the host. M0 owns the 30-day scenario, `ticks_per_game_day`, and the first real schema migration in M0-T11. ADR-010 defers aggregate cohorts. ADR-007 and ADR-008 defer host cadence. Core reproducibility tier R2, cross-platform bitwise replay, stays deferred. Production activation policy and travel approximation stay open until M0-T05 supplies a comparison and ADR-009 records any approved field bound. C0.2 uses explicit commands and exact transfers instead of those open policies.

## 2. Baseline verification

The pinned toolchain was not on `PATH` at the start of this review. Rustup installed host toolchain `1.100.0-x86_64-pc-windows-msvc`, and `cargo fetch --locked` populated the registry from the committed lockfile. Offline checks then ran from the package root. No source file, lockfile, or golden literal was edited. C0.1 vectors were not regenerated.

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | Exit 0 |
| `cargo check --locked --offline` | Exit 0 |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | Exit 0, no warning output |
| `cargo test --locked --offline` | 14 unit tests and 1 doctest passed |
| `cargo test --release --locked --offline` | 14 unit tests and 1 doctest passed |

`rustc` is 1.100.0, commit `0ef1bc386059596dada8fb65b50425aab2ea637a`, host `x86_64-pc-windows-msvc`. The earlier C0.1 log used `x86_64-unknown-linux-gnu`. This Windows pass is a second execution of the same suite. It is not an R2 certification.

## 3. Proposed C0.2 architecture

Keep one headless library. Add a domain blob beside the frozen observation.

`SH:C01:STATE` revision 1 stays the output of `checkpoint_bytes()` for fixture 1 states the current API can already represent. Restore rejects that tag. The persistent format is `SH:C02:STATE` revision 1: the same future-affecting slot, queue, ordinal, cursor, and digest fields, plus actor mode, `next_actor_id`, `next_record_id`, and the location, cargo, reservation, task, and marker tables. A SHA-256 trailer covers the body. The external checkpoint hash covers the trailer as well.

Two fixtures share that schema and do not share payloads. Fixture 1 keeps claims, probes, C0.1 event bytes, and the approved event-3 digest `1a1cc22bf1041c19845bceb790e6761dad0edaf41cb3eca1f90b0ce06f1875b1`. Fixture 2 installs two actors, two origin-owned cargo tokens, two reservations, and two tasks. Mode changes are admitted `SetMode` commands. Record IDs 1 through 9 are issued at construction. Later mints continue at 10. There is no free list.

Each successful transition still admits one batch atomically, then advances one tick. Callers use `drive`. Fixture 1 may also use `step`, which returns the same command events. Fixture 2 `step` returns `UseDrive` and does not mutate. The order after the tick advance is mode phase, command phase, work phase. Mode commands emit no events. A mint keeps its real command key. A task completion is a generated event and has no command key. A mint or a task completion stores the mode already applied in that tick, which makes a wrong phase order fail a golden blob. Scheduled work at tick 3 moves each token once and releases each reservation once. Active and scheduled modes use that same transition. No API skips idle ticks. Grouped replay is a loop of the same boundaries.

Resume loads the blob and continues at its tick. Fixture 1 may continue with `step` or `drive`. Fixture 2 continues only with `drive`; `step` returns `UseDrive` and does not mutate. The resume emits only new events. Pending commands, ordinals, cursor, and digest come from the blob. The host keeps historical event bytes outside it. The driver compares prefix plus suffix with an uninterrupted run.

## 4. Decisions that review must accept

G0 requires the results and leaves several byte-level choices open. The specification closes them in its §14. Approval of this document is approval of those choices:

- A new tag and revision, with a checksum trailer, rather than a loadable C0.1 observation or a serde document.
- No migrator in C0.2. Foreign tags and other revisions fail. M0-T11 owns the first migration.
- Fixture 1 and fixture 2 reject each other's payloads so the frozen event dialect stays intact.
- Explicit `SetMode` commands, applied before commands and tasks on the due tick.
- Mode changes are state, not events. Witness bytes make the phase order testable.
- One monotonic record cursor, fixed actors 1 and 2, and no ID reuse.
- Two concrete cargo tokens with one owner each. Reservations are not a second owner.
- No idle-tick skip. The event log stays outside the blob.
- `Event` remains the C0.1 command event. Fixture 2 mints and task completions use separate public types, and fixture 2 is driven by `drive`.

The acceptance file pins the resulting bytes, including the tick-2 save SHA-256 `446a3b22d12c800c95c40811038531422afe419fb8b9923bba2df6381c3ee6f7` and the tick-3 resume SHA-256 `53939438b57d0dcc685929484ac15f54eea1a5f82fff44b8862943de2fadf3ae`. The approved event-3 digest was recomputed from the published H2 and event bytes before those blobs were packed.

## 5. G1 findings and remediation

The first C0.2 review treated several byte-level choices as closed and said no blocker remained. That claim was wrong. G1 found two blockers and two validation defects. This revision resolves them in the specification and the acceptance tests. No previously published C0.2 golden blob changes, because the canonical encodings and successful outcomes did not change.

| Finding | What was wrong | Remediation |
| --- | --- | --- |
| B01 Fixture-specific restore | Shared semantic rules accepted fixture 1 states with scheduler records or non-active modes, and fixture 2 states with an occupied slot, a drawn ordinal, or a foreign payload. Probe ranges and `next_event_id >= 1` were not required. Unreachable task combinations could pass | §10 now lists shared rules, then fixture 1 rules, then fixture 2 rules, in deterministic order. Checksum-repaired negative cases are C02-T01f. Every published golden blob remains a positive restore |
| B02 Scheduler event API | Fixture 2 bytes were specified, but callers only had `Event`, whose `key()` returns a `CommandKey`. A task completion has no producer or sequence. Putting one there would invent an admission the kernel never made | `Event` and its four methods stay the C0.1 API. Mints are `SchedulerCommandEvent` values whose `key()` is the real mint command. Task completions are `GeneratedEvent` values with `tick()` and `actor_id()` and no key. Fixture 2 uses `drive`. `step` on fixture 2 returns `UseDrive` and does not mutate. C02-T04 is the compile example |
| M01 Checksum precedence | "Invert one non-trailer bit" could invert the tag or revision and correctly return `UnsupportedFormat` or `UnsupportedRevision` instead of `ChecksumMismatch` | Offset 0 is the tag case. Offset 12 is the revision case, and it keeps the original trailer. Offset 14 is the body case and requires `ChecksumMismatch` |
| M02 Bounded parsing | A checksum-valid `u64::MAX` count could panic or allocate before the body was known to contain those records | The decoder checks `count * minimum_size` with checked arithmetic against the unread body before any list allocation. Impossible counts return `Corrupt`. C02-T01g covers actor, pending, and cargo counts |

`ticks_per_game_day` stays out of revision 1 because Core §3 and ADR-024 attach it to scenarios that measure days. This fixture's due ticks are absolute. Disabled synthetic tasks still complete, as specified in implementation §8, because Core §12 invariant 4 and the C03-T01 disablement rule belong to production jobs. LOD §4 requires the synthetic tokens to be conserved. Those two statements name different milestones, so they do not contradict each other.

## 6. Findings that remain outside C0.2

| Item | Disposition |
| --- | --- |
| When the simulation should schedule an actor without an explicit command | Open at M0-T05. C0.2 does not guess a policy |
| Which fields, if any, may later be approximate | Open until an ADR-009 amendment. C0.2 forbids approximation |
| Anonymous cohorts | Deferred by ADR-010 |
| Cross-platform bitwise replay | Deferred R2. The Windows re-run does not close it |
| Crash-safe file replacement | Not required. Torn files must fail validation |
| Stale README gate paragraph | Recorded here. Not edited in this change |
| No separate C0.1 audit document in the tree | The handoff declares the gate passed. Local tests passed before this documentation revision. A missing narrative audit is not a C0.2 schema gap |
| Event digest is not recomputed from a missing log | Stated in §10. The trailer covers accidental change. Reference comparison covers a substituted digest |

No G1 blocker remains inside the revised text. A reviewer who rejects the `drive` split, or the decision that synthetic tasks ignore the disabled flag, should return the document. An implementer should not invent a replacement during coding.

## 7. Implementation sequence after approval

1. Add domain encode and decode without changing C0.1 golden bytes.
2. Land fixture 1 restore, ordinal resume, file round-trip, checksum precedence, bounded counts, and the fixture-specific rejection matrix.
3. Land `drive`, `SchedulerCommandEvent`, and `GeneratedEvent` without changing `Event::key` or `Event::canonical_bytes` for C0.1 outcomes.
4. Land the record cursor and mint continuity.
5. Land fixture 2 and the tick 0 through tick 4 oracle.
6. Land the phase witness, disabled mint, overdue rollback, and cursor overflow.
7. Add the C02-T04 doctests. Re-run the locked format, lint, debug, and release tests. Do not edit published vectors to fit a divergent encoder.

Stop if a C0.1 vector changes. That is a regression, not a C0.2 adaptation.

## 8. Readiness

| Question | Verdict |
| --- | --- |
| C0.1 source modified by this remediation | No |
| Previously published C0.2 golden blobs modified | No |
| B01 fixture restore rules | Closed in specification §10 and C02-T01f |
| B02 public event API | Closed in specification §4 and §9 and C02-T04 |
| M01 checksum precedence | Closed in C02-T01d |
| M02 bounded counts | Closed in specification §10 and C02-T01g |
| Deferred day-length and disabled-job rules | Explicit, and aligned with Core §3, Core §6, Core §12, ADR-024, and C03-T01 |
| Implementation | Not started and not authorized |

**Documentation verdict: READY FOR G1 RE-REVIEW.**  
**Implementation verdict: NOT AUTHORIZED.**
