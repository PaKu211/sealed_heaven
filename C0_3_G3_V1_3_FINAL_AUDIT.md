# C0.3 G3 v1.3 Final Acceptance Audit

Version 1.0 · 17 November 2026  
Role: independent principal Rust simulation architect and deterministic systems auditor  
Subject: normative specification version **1.3-approved**  
Implementation: not started. This audit does not authorize S1 and does not edit normative text, golden vectors, or Rust.

## Verdict

**PASS**

The v1.3-approved contract determines unique canonical checkpoints for C03-T01a, M0-T06, and M0-T07. An independent packer, written from `docs/C0_3_IMPLEMENTATION_SPEC.md` §3 and the acceptance tables, reproduces the published SHA-256 values. NF-05 is closed. F01–F09 and NF-01–NF-04 stay closed. The mandatory exit is intact. Frozen C0.1 and C0.2 compatibility holds on the bytes and on `cargo test --locked --offline` (25 tests plus 2 doctests, all passed).

## Documents examined

| Document | Role |
| --- | --- |
| `docs/C0_3_IMPLEMENTATION_SPEC.md` | Normative specification, version 1.3-approved |
| `docs/C0_3_ACCEPTANCE_TESTS.md` | Exit procedures, version 1.3-approved |
| `docs/C0_3_ARCHITECTURE_REVIEW.md` | Architecture review, version 1.3-approved |
| `C0_3_G3_NF_RESOLUTION_MATRIX.md` | NF-01–NF-05 closure record |
| `C0_3_G3_NF_REMEDIATION_VERIFICATION.md` | Remediation notes |
| `C0_3_G3_FINDING_RESOLUTION_MATRIX.md` | F01–F09 closure record |
| `C0_3_G3_V1_2_INDEPENDENT_AUDIT.md` | Prior FAIL on NF-05 |
| `C0_3_BALANCED_DEVELOPMENT_PLAN.md` | S1–S4 plan |
| `C0_3_IMPLEMENTATION_GATE_CHECKLIST.md` | Gate state |
| `C0_3_OWNER_DECISION_RECORD.md` | OD-01–OD-16 and AM-01–AM-08 |
| `README.md`, `docs/M0_ACCEPTANCE_TESTS.md` | Milestone exit sentences |
| `C0_2_BASELINE_MANIFEST.md`, `src/tests/c02_golden.rs`, `src/tests/golden.rs` | Frozen C0.1/C0.2 literals |
| `Cargo.toml`, `Cargo.lock`, `src/**/*.rs` | Frozen kernel surface |

Normative file hashes are in `C0_3_G3_V1_3_VERIFICATION_LOG.txt`.

## Method

Checkpoint bodies were packed from specification §3 in field order: tag, revision, fixture id, scenario content hash, tick and seed, four cursors, scenario block, actors, sites, holders, edges, one grid, stocks, reservations, tasks, empty transaction, registry, pending, and trace lists, then `next_event_id` and the event digest. The blob is `body || SHA-256(body)`. Integers are little-endian. Records have no padding. `i64` values, including the five relationship minimums at `-9223372036854775808`, use two's complement. The content hash is SHA-256 of the scenario block alone.

Event digests use `H0 = SHA-256("SH:C03:EVENT-INIT:v1")` and then `SHA-256("SH:C03:EVENT:v1" || previous || canonical_bytes)`. Canonical bytes are `u64 id`, `u64 tick`, `u32 actor_id`, `u8 kind`, payload. This encoder was written for this audit. It is not the remediation packing log, and it was not added to the repository.

Mode `active` is the frozen C0.2 discriminant 1. The shared constructor's wake interval is 4 except where M0-T07 replaces it with 100. Actors are the actors the test uses. The 3×3 grid is row-major with the center cell blocked, which is byte index 4 in either axis order.

## 1. NF-05 is closed

NF-05 was the absence of `goal`, `recipe_id`, `interruption`, target locus/site/x/y, and `progress_ticks` on preinstalled tasks, which made the C03-T01a and M0-T06 checkpoints non-unique.

Version 1.3 names those fields in specification §§3, 8, and 12 and in acceptance §§1, 3, and 11:

| Field | Preinstalled value | After the successful work transition |
| --- | --- | --- |
| `goal` | 0 | unchanged |
| `recipe_id` | 0 | unchanged |
| `interruption` | 0 | unchanged |
| target | locus 1, site 1, x 0, y 0 | unchanged |
| `progress_ticks` | 0 | 1 on each winner; 0 on each loser |
| `created_tick` | 0 | 0 |
| `due_tick` | 0 while Queued | 3 on each winner after §6.1; 0 on each loser |

The independent blob hashes match the published oracles. Replacing task 1's goal with 1, or its `progress_ticks` with 0, changes the C03-T01a blob hash. Those fields are load-bearing. No extra task field had to be invented to reach a unique body.

Normative references: specification §§3, 6.1, 8, 12; acceptance §§1, 3, 11.

## 2. Scripted goal 0

Goal 0 is legal for a scripted task that decision did not create. The fixture contract requires it on the preinstalled carriers and replaces it where a row names another goal.

| Surface | Goal byte | Authority |
| --- | --- | --- |
| C03-T01a tasks 1 and 2, initial and final | 0 | Acceptance §3; specification §12 |
| M0-T06 tasks 1–20, winners and losers | 0 | Acceptance §11; specification §12 |
| C03-T01b / C03-T01d continuations of that packing | 0 | Acceptance §§4 and 6, pointing at §1 and §3 |
| Decision plans | 1 eat, 2 rest, 3 work, 4 haul, 5 return home | Specification §7 |
| No eligible decision goal | winner 0 and task id 0; no task is created | Specification §7 |
| M0-T07 hops 1–3 and task 405 | 4 | Acceptance §12 replaces the packing default |
| M0-T07 task 404 | 3 | Acceptance §12 |

Decision evaluates goals 1 through 5 only. A winner byte of 0 is the no-task outcome, not a goal-0 task record. The packed C03-T01a and M0-T06 images contain goal 0 on every preinstalled task.

## 3. `progress_ticks`

Specification §6.1 assigns `due_tick` during reservation promotion and does not increment `progress_ticks`. Specification §8 increments `progress_ticks` by 1 on an advancing work-phase transition and nowhere else in the contract.

Checked transitions:

| Case | Staged inputs | First matching work rule | `progress_ticks` |
| --- | --- | --- | --- |
| C03-T01a task 1 after promotion | created 0, stage Reserved, due 3, empty path, at target, staged tick 1 | Rule 4 → Executing | 0 through promotion, then 1 |
| C03-T01a task 2 and M0-T06 tasks 6–20 | stage Queued, due 0 | Rule 2, no event | stays 0 |
| M0-T06 tasks 1–5 | same shape as the C03-T01a winner | Rule 4 | 1 |
| C03-T01b stated history | Reserved → Executing → Completing | Rules 4 and 9 | 2, as acceptance §4 states |
| Creation tick | `created_tick` equals the staged tick | Rule 1 | no change and no event |

Rule 3 (`due_tick` below the staged tick) does not fire on these winners because §6.1 has already set `due_tick` to `1 + 0 + 2 = 3`. Queued losers stay on rule 2, so a due tick of 0 does not fail the boundary. Invalidation, mode, decision, and residual travel (`task_id` 0) have no `progress_ticks` write in the contract.

## 4. Canonical checkpoints

### C03-T01a post-boundary

Tick 1, seed 100, wake interval 4. Cursors `(next_actor_id, next_record_id, next_transaction_id, next_stock_id) = (3, 4, 1, 2)`. `next_event_id` 5.

Events at tick 1, in order: kind 1 reservation 3 stock 1 quantity 1 actor 1; kind 7 task 1 stage Reserved; kind 2 stock 1 quantity 1 reason 4 actor 2; kind 7 task 1 stage Executing.

| Task | stage | due | created | progress | goal | reservations |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Executing (4) | 3 | 0 | 1 | 0 | 3 active, acquired 1, expiry 5 |
| 2 | Queued (1) | 0 | 0 | 0 | 0 | none |

Stock 1 on-hand 1, active reservation quantity 1. Body 1230 bytes. Blob 1262 bytes.

| Hash | Value | Result |
| --- | --- | --- |
| Blob SHA-256 | `11f9aae46d7f0f04b6c3efead0a25c3086bc56904877d237c0a79ae032a293d9` | Match |
| Body SHA-256 (derived) | `e48ec6e15929152f53c8f6812dcc2f7b08c0588d5e7b9886f0e053e5199cc41d` | Independent derivation |

Event digests after events 1–4:

```text
33dafb9c588fd7299c5742a873e8356e1cbda54998c3d9b2b55a4f89a2311e91
40076c534936ba6eaf9fa061d5b6e3e2f00a79123e92987e036b82ab18735d62
8b04d4b8805008055104ddfde0bfa51810a5f1adcdfed010e9208dfa8b97cf37
5fdbc5707211fbfc309f5ed75e21eaa83ebf3d7307da60cc442a3cd7f8340908
```

### M0-T06 post-boundary

Tick 1, seed 100, wake interval 4. Cursors `(21, 41, 1, 105)`. `next_event_id` 46. Forty-five events.

Winners 1–5: goal 0, stage Executing, `due_tick` 3, `created_tick` 0, `progress_ticks` 1, four active reservations. Actor `a` holds reservation ids `21 + 4*(a-1)` through `24 + 4*(a-1)` on stocks 101, 102, 103, 104, acquired 1, expiry 10. Losers 6–20: goal 0, Queued, `due_tick` 0, `progress_ticks` 0, no reservations.

Stocks 101–104 stay at on-hand 5. Active quantity is 5 on each, so availability for any further claimant is 0. Food is not debited.

Event order: for each winner in actor order, four kind 1 events in reservation-id order, then one kind 7 Reserved; then each loser, one kind 2 on stock 101 quantity 1 reason 4; then one kind 7 Executing per winner. Final digest `9a6933915fde0ab4ccb2965a16c49278da2b8ca389ef768d22dd24bfce62d76c`.

| Hash | Value | Result |
| --- | --- | --- |
| Blob SHA-256 | `528bc334bc1f081ccbf06c3247ec46909abd6f1073594dfb13ebd7ae16d640c9` | Match |
| Body SHA-256 | `bccf8e1adf8bc22623202a58fd5d84b9911d8d1e42959331c692fcc55e3f1ed0` | Match |

Body 6721 bytes. Blob 6753 bytes.

### M0-T07 initial

Tick 0, seed 100, wake interval 100, `next_wake_tick` 100. One actor, `task_id` 0, so `next_actor_id` 2. Other cursors `(401, 1, 11)`. `next_event_id` 1. Digest H0. Stock 10, definition 1, class 1, quantity 1, owner kind 1, owner 1. No tasks and no reservations. Holders 20 kind 2 and 30 kind 4. Recipe 1 as in specification §12.

| Hash | Value | Result |
| --- | --- | --- |
| Blob SHA-256 | `bd803a2c4ae4320ce2a1a5ceb49a2941aec48921c4cbc0a60b15c607b62148c3` | Match |
| Body SHA-256 | `787a40d5eabadfeb9294851ee967dd0a81122fac44f91b04b961f1ced59f6f78` | Match |

Body 830 bytes. Blob 862 bytes. A body with `next_actor_id` 3, and a body that keeps wake interval 4, both miss this published blob. The acceptance initial image is unique under those two readings.

H0 matches `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c`.

## 5. Ordering, restore, and replay

The C03-T01a and M0-T06 streams follow specification §4: acceptance is kind 1 in new-reservation order, then one kind 7; a bundle rejection is one kind 2; work that changes stage without finishing is one kind 7. Command-key order makes actor 1 win C03-T01a in either submitted batch order. M0-T06 admits actors 1 through 20 in key order. Decision does not run, because `next_wake_tick` is 100 and the staged tick is 1.

A structural walk of each packed body confirmed specification §3.2 for these images: fixture 3, content hash equal to the scenario block, trailer equal to SHA-256(body), cursors strictly above their namespaces and nonzero, strictly increasing ids, task ids disjoint from reservation ids, channels in range, mode and locus legal, positive edge costs, center-blocked 3×3 grid, active quantities within on-hand, expiry greater than acquired, reservation ids on each task ascending and owned by that task, `path_index` within the step count, and `next_event_id` at least 1. The stored digest is the chain digest. Repacking the same inputs produced identical bytes.

Replay equivalence in the contract: the checkpoint holds the cursor and the digest (specification §10; acceptance §§3, 11, and 12). Restore continues at event 5, event 46, or event 1 respectively and does not re-emit the prefix. A failed boundary, including the M0-T07 tick-10 later leg, restores the pre-call bytes and leaves the cursor and digest unchanged (specification §§6.2 and 10; acceptance §§8, 10, and 12). No Rust fixture-3 kernel exists to execute, which is the required state of this gate. The oracles themselves are replay-stable byte strings.

## 6. Finding closure

| ID | Status | Evidence |
| --- | --- | --- |
| F01 | Closed | §8 rule 1 skips work on `created_tick`. Acceptance §12 applies each hop on the next tick. Task 405 rolls back on the later leg |
| F02 | Closed | §3 registry record is the four key fields, submitted id, resolved id, and role. That is the stated 41-byte record. Acceptance §§7 and 9 |
| F03 | Closed | §6.2 leg definition, class, and owner checks. Other-task availability. Stock 14 later-leg rollback. Acceptance §§8 and 12 |
| F04 | Closed | One `next_record_id`. Starts at 3, 21, and 401. Packed images show no task/reservation collision |
| F05 | Closed | §5 first-match table. Same submitted id is a no-op. A different submitted id is `ConflictingIdentity` |
| F06 | Closed | §7 five goals, diagonal weights, wake 4 except the M0-T07 row, neutral `i64` minimums |
| F07 | Closed | §6.1 sets due tick before work. §8 rule 2 exempts Queued. Winners reach Executing with due 3 |
| F08 | Closed | §8 residual edge travel after `task_id` 0. §9 and §11 reject OC-EDGE. Approved edges stay cost 1 |
| F09 | Closed | C03-T01a and M0-T06 post-boundary blobs, and the M0-T07 initial blob, re-derive to the published hashes |
| NF-01 | Closed | Same due-tick assignment as F07. `created_tick` stays 0, so the creation-tick skip does not swallow the reserve tick |
| NF-02 | Closed | Stocks 101–104 are definitions 3, 4, 1, 5 and classes 2, 2, 1, 2. Those bytes are in the M0-T06 body |
| NF-03 | Closed | Work tool is class-2 definition 4. Workstation is class-2 definition 5. One stock cannot fill both roles |
| NF-04 | Closed | Nonzero credit matches definition, class, and owner, then adds with a checked `u64`. Mismatch, overflow, and class-2 quantity change are `InvalidLeg` |
| NF-05 | Closed | Section 1 of this audit |

## 7. Mandatory acceptance scope

| Exit item | Where it is specified | Audit result |
| --- | --- | --- |
| C03-T01 | Acceptance §§3–10 | Two-claimant oracle packed. Expiry, cancel, disable, replay, later-leg failure, reservation protection, existing-stock credit, unconsumed-list restore, and hostile-save diagnostics are stated and consistent with §§5–6 and §3.2 |
| M0-T06 | Acceptance §11, including §3 as the one-unit case | Packed. On-hand conserved. Five winners, fifteen lost-contention rejections |
| M0-T07 | Acceptance §12 | Initial image packed. Actor → holder 20 → holder 30 → site 2, then recipe output definition 2 at site 2. Tick 10 `InvalidLeg` restores tick 9 |
| M0-T08 | Acceptance §13; specification §§4–6 | Disablement outranks cancellation and expiry, so the reason byte is 1 when all three apply. Order is kind 3, kind 7 Failed, kind 5. Unreachable site 9 fails before a task exists. Repeat release emits nothing |
| Five goals | Specification §7; acceptance §14 | Eat, rest, work, haul, return home. Eat score at hunger 1000 is 1000000. Creation tick emits no work event |
| Resource conservation | Specification §6.2 | Every committed quantity change is a stored leg. Rollback changes no on-hand quantity. Class-2 quantity stays immutable. M0-T06 on-hand stays 5 |
| Navigation | Specification §9; acceptance §14 | Site 1 → site 2 is edge 1, cost 1. Grid `(0,0)` → `(2,2)` with the center blocked: the shortest paths cost 4, and the lexicographically smaller direction sequence is north, north, east, east |
| Atomic rollback | Specification §§6.1, 6.2, and 10; acceptance §§3, 8, 10, and 12 | A later failure undoes reservation writes, `due_tick`, legs, cursors, and the digest |
| C0.1 / C0.2 regression | Acceptance §2 | Section 8 of this audit |

Acceptance §15 still maps every mandatory exit sentence. OC-EDGE is rejected and is not an acceptance case.

## 8. Frozen C0.1 and C0.2 compatibility

| Check | Result |
| --- | --- |
| C0.2 archive `Sealed_Heaven_C0_2_Source.zip` | SHA-256 `2804d33a785acb7331a0d82fd1c080bec82d0add348578abb4b1a6807db60a40` |
| `Cargo.lock` | SHA-256 `176e4bb5d145739c101c10296ce9f97d4a24917381be057e74134e0ba1779d75` |
| Toolchain | Rust and Cargo 1.100.0 |
| Direct dependencies | `rand_chacha` `=0.10.0`, `sha2` `=0.11.1`. No Bevy crate |
| Witness tick 1 | 566 bytes, SHA-256 `35b5104ca23dc7723baffbe76948a044fdda583eb8374604149133986e2f4ac2` |
| Event 3 digest | Present in the frozen `T01_FINAL` blob as `1a1cc22bf1041c19845bceb790e6761dad0edaf41cb3eca1f90b0ce06f1875b1` |
| `src/**/*.rs` | No `SH:C03` tag and no Bevy import |
| `cargo test --locked --offline` | 25 tests passed, 2 doctests passed, 0 failed |

Specification §2 keeps `SH:C01:STATE` and `SH:C02:STATE` as `UnsupportedFormat` on the fixture-3 decoder, and `SH:C03:STATE` as `UnsupportedFormat` on the C0.2 decoder. There is no migrator. Fixture 2 keeps its own synthetic work phase, including completion while disabled.

## 9. Balanced plan S1–S4

`C0_3_BALANCED_DEVELOPMENT_PLAN.md` §2 keeps the README exit: needs, jobs, inventory, and resource transactions, evidenced by C03-T01 and the applicable M0-T06, M0-T07, and M0-T08 procedures. §4 leaves S1 without needs, movement, and the 20-actor case; S2 without the other four goals and navigation; S3 without the 20-actor integration. S4 is the first slice that can meet the exit, together with the frozen C0.1 and C0.2 suites. Specification §1 and the gate checklist say the same thing. The plan's baseline line still says the normative documents are 1.1-approved. That status line is behind version 1.3-approved. It does not delete or narrow an exit row.

S1 has not started. `src` contains the frozen C0.1/C0.2 kernel only.

## Boundaries observed

Normative documents were not modified. Golden vectors were not modified. No Rust fixture-3 implementation was written. Bevy integration was not started. S1 was not started. The audit stops here.

## Verdict

**PASS**
