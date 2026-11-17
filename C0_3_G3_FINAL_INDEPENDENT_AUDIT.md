# C0.3 G3 Final Independent Audit

Date: 17 November 2026  
Role: Independent Principal Rust Simulation Architect and Deterministic Systems Auditor  
Reviewed specification: `docs/C0_3_IMPLEMENTATION_SPEC.md` version **1.1-approved**; acceptance version **1.1-approved**  
Approval record: `C0_3_OWNER_DECISION_RECORD.md` version **2.0**  
Companion log: `C0_3_G3_FINAL_VERIFICATION_LOG.txt`

**Verdict: FAIL**

**Implementation remains NOT AUTHORIZED. S1 remains NOT STARTED. G3 PASS is not declared.**

This audit did not author the specification. No normative document, Rust source, golden vector, lockfile, or archive was modified. Independent packing used a separate Python reference packer, not `sealed_sim`.

## 1. Executive conclusion

The 1.1-approved packet correctly incorporates most of AM-01 through AM-08: creation-tick skip, completion submitted/resolved provenance, later-leg rollback on stock 14, residual edge travel, OC-EDGE rejection, shared `next_record_id`, and the five-phase order. H0 verifies. Fixture 2 isolation and the frozen C0.2 archive identity verify. Bevy is absent from the kernel dependencies.

However, the normative work table and the mandatory acceptance exits for C03-T01a and M0-T06 are contradictory on `due_tick`. After a reservation promotes a task from `Queued` (`due_tick` 0) to `Reserved`, work rule 3 requires `InvalidState` because `due_tick < staged tick`, while acceptance requires an `Executing` stage event in the same boundary. The remediation proposal's "promotion sets `due_tick`" sentence was not copied into the normative specification. Without inventing that rule, the mandatory exits cannot be executed as written.

Additional high gaps remain for M0-T06 stock `definition_id` values and Work-goal tool/workstation identification. A complete set of gameplay checkpoint goldens therefore cannot be derived without inventing missing contract data.

## 2. Evidence and reviewed identities

| File | Bytes | SHA-256 |
| --- | --- | --- |
| `C0_3_OWNER_DECISION_RECORD.md` | 8058 | `55db85c55dd1044309e77314547e8e1f0e113a20b1ad12eac8dde9fe76d018b2` |
| `docs/C0_3_IMPLEMENTATION_SPEC.md` | 29761 | `da69aebcecbd9d82ebda79d890db747b6e55106d323eb61d2d345366627f5f0c` |
| `docs/C0_3_ACCEPTANCE_TESTS.md` | 14284 | `89f7d96d3a330bacf128512fc112748d48e127c2c6559baecc0dcf9125ffbe3a` |
| `docs/C0_3_ARCHITECTURE_REVIEW.md` | 3513 | `0bbb9c1c286bbc37ef78d5ba1ee9177043acbdd04be2b58b1e38751396d1572e` |
| `C0_3_G3_REMEDIATION_REPORT.md` | 5047 | `d453aeb970d3cb38485db97e9da4ed08412f58375969f4e61700b2d32391bfb0` |
| `C0_3_G3_FINDING_RESOLUTION_MATRIX.md` | 1665 | `8759212f3381b20ab35a148fcb05d3ffa90221cf8027cf8df6815e9b6a5adee0` |
| `C0_3_G3_INDEPENDENT_AUDIT.md` (prior FAIL) | 40041 | `ffa3f7500cc4f38c59754993e0bcbedd5a5cd1fb234c26edad6a083c9f6a60f0` |
| `C0_3_BALANCED_DEVELOPMENT_PLAN.md` | 4436 | `b4760b9a65732075dbe52173ff0ff92ff83fb49ef41d190657d984cf8d9f67fc` |
| `C0_3_VERTICAL_SLICE_TEST_PLAN.md` | 6879 | `55da3ec8997a981ed439d74db9eb72fd34177f8647610cec036df64d17e1130e` |
| `C0_3_IMPLEMENTATION_GATE_CHECKLIST.md` | 2598 | `12ecc0ae29c3696daad2e4aef4bc93480db2de3814fc6898ee2e49a645943508` |

Also inspected: prior R2.1-final proposal files, G0 Core / Technical Architecture / ADR / M0 acceptance, `C0_2_BASELINE_MANIFEST.md`, `Cargo.toml` / `Cargo.lock` / `rust-toolchain.toml`, and fixture-2 completion paths in `src/lib.rs` / `src/domain.rs` for regression boundary only.

| Artifact | Result |
| --- | --- |
| `Sealed_Heaven_C0_2_Source.zip` | SHA-256 `2804d33a785acb7331a0d82fd1c080bec82d0add348578abb4b1a6807db60a40` matches the frozen baseline |
| Direct deps | `rand_chacha = "=0.10.0"`, `sha2 = "=0.11.1"`; no Bevy |
| Toolchain pin | Rust 1.100.0 present |

## 3. Owner decision compliance

| Decision | Normative presence | Independent result |
| --- | --- | --- |
| OD-01–OD-16 | Spec §§1–12; decision record §§3 | Present. Amended by AM rows below |
| AM-01 | Spec §§5, 8; acceptance §12 | Creation-tick skip retained and ordered first. M0-T07 two-tick hops present |
| AM-02 | Spec §§3, 5, 6; acceptance §§7, 9 | Submitted/resolved ids and 41-byte registry present |
| AM-03 | Spec §§3, 6; acceptance §§8, 12 | Legs carry definition/class. Stock-14 later-leg present. Other-task reservation protection present |
| AM-04 | Spec §§3.1, 3.2, 12; acceptance §§3, 11, 12 | Shared cursor; C03-T01a starts at 3; M0-T06 at 21; M0-T07 at 401 |
| AM-05 | Spec §5; acceptance §§5, 7 | First-match table present; submitted-id no-op; conflicting payload rejection |
| AM-06 | Spec §7; acceptance §14 | Five plans present; enable reschedule present |
| AM-07 | Spec §§5, 8; acceptance §§3, 11, 12 | Queued overdue exempt. **Due-tick promotion after reserve not normative** → see NF-01 |
| AM-08 | Spec §§8, 9; acceptance §14 | Residual travel present; no teleport |
| OC-EDGE | Spec §§11–12; decision record §2 condition 7 | **REJECTED** and absent from the fixture graph |

No unapproved Bevy, second record cursor, same-tick completion exception, or cost-2 edge was found in the normative text.

Binding conditions 1–10 are present in the decision record and reflected in the specification authority section. Condition 10 correctly keeps S1–S4 as internal slices, not exits.

## 4. Previous findings F01–F09

Remediation-report claims were not accepted as proof. Each row cites the normative text and states whether the original defect is closed.

| ID | Original defect | Normative evidence | Acceptance evidence | Independent disposition |
| --- | --- | --- | --- | --- |
| F01 | Same-tick completion vs OD-10 skip | Spec §8 rule 1; §6 legs require `created_tick` ≠ staged tick | Acceptance §12 ten-tick table; create then apply | **CLOSED** |
| F02 | Completion flag lost identity | Spec §3 unconsumed list + 41-byte registry; §6 lowest key / roles | Acceptance §§7, 9 | **CLOSED** |
| F03 | Leg missing definition/class; recipe binding | Spec §3 leg fields; §6 recipe and conservation | Acceptance §§8, 12 stock-14 later-leg | **CLOSED** for the original encoding gap |
| F04 | Impossible reservation ids | Spec §3.1; restore check 2 | Acceptance §§3, 11, 12 cursors 3/21/401 | **CLOSED** |
| F05 | Replay vs LateTarget unordered | Spec §5 rules 5–10 | Acceptance §§5, 7 | **CLOSED** |
| F06 | Five goals under-specified | Spec §7 table | Acceptance §14 | **PARTIAL** — plans exist; fixture stock definitions for Work/Eat incomplete (NF-02, NF-03) |
| F07 | Queued/overdue/work events | Spec §8 rules 2–3; acceptance §§3, 11 | Acceptance expects Executing after reserve | **OPEN** — Queued exemption exists, but reserve→work with `due_tick` 0 contradicts rule 3 (NF-01) |
| F08 | Mid-edge / graph | Spec §§8–9 residual travel; §12 no edge 4/5 | Acceptance §14 residual case | **CLOSED** |
| F09 | Incomplete oracles | Acceptance 1.1 sequences | Packing log | **OPEN** — M0-T07 initial pack succeeds; C03-T01a/M0-T06 post-boundary checkpoints cannot be derived without inventing due-tick promotion |

## 5. New findings

### NF-01 — BLOCKER — `due_tick` after reservation contradicts mandatory exits

**Evidence.** Acceptance §3 installs tasks with `due_tick` 0. Command accepts a reservation and emits kind 7 `Reserved`. Spec §8 work rule 3: if stage is not `Queued` and `due_tick` is below the staged tick, the boundary is `InvalidState`. On staged tick 1, stage is `Reserved` and `due_tick` is 0, so rule 3 fires before rule 4 (`Reserved` → `Executing`). Acceptance §3 cursor 4 and §11 nevertheless require kind 7 `Executing`.

The same contradiction appears in acceptance §6 (C03-T01d tick 1) and §11 (M0-T06). Spec §§6–8 never state that `ReserveStock` / `ReserveBundle` assigns `due_tick`. The earlier remediation proposal's promotion sentence is not normative.

**Effect.** C03-T01a, C03-T01d, and M0-T06 are not jointly executable with the work table. Inventing a promotion rule would be an unapproved architectural departure.

### NF-02 — HIGH — M0-T06 stock `definition_id` values are unspecified

Spec §12 and acceptance §11 name stocks 101–104 with classes but not `definition_id`. Spec §7 Eat requires class-1 **definition-1**. An independent packer cannot assign definition bytes for the twenty-actor world without invention. Needs/Eat acceptance §14 also assumes a food unit whose definition is 1, but the shared constructor does not install that stock's definition field explicitly for the M0-T06 supplies.

### NF-03 — HIGH — Work-goal tool and workstation selection is incomplete

Spec §7 says “lowest tool” and “lowest workstation” without a normative discriminator (definition id, class pair, or fixture tag). Fixture stocks 102 and 104 are named in prose only. Deterministic selection cannot be packed or tested without inventing that mapping.

### NF-04 — HIGH — Credit / transform-in to an existing non-zero `stock_id` is incomplete

Spec §6 defines allocation for `stock_id` 0. It does not state whether a credit to an existing stock of matching definition, class, and owner adds quantity, rejects, or allocates anew. Autonomous haul credits owner kind 3 at home and can collide with a pre-existing stockpile row. M0-T07 always uses `stock_id` 0 for credits, so that script is unaffected; the five-goal haul path is affected.

## 6. Schema, events, and independent packing

### 6.1 Derived widths (independent)

| Record | Width (bytes) |
| --- | --- |
| Actor | 131 |
| Site | 9 |
| Holder | 9 |
| Edge | 32 |
| Stock | 34 |
| Reservation | 53 |
| Leg | 35 |
| Unconsumed completion record | 40 |
| Consumed registry record | 41 |
| DecisionTrace | 324 |
| Event kinds 1–7 | 45, 38, 30, 37, 30, 78, 30 |

Canonical order, little-endian integers, count-prefixed lists, body‖SHA-256 trailer, restore precedence, and bounded count/grid checks are present in spec §§2–3. Future-affecting fields listed in §10 include position, residual mid-edge state, unconsumed list, and consumed registry.

### 6.2 H0 and vectors

| Vector | Result |
| --- | --- |
| H0 = SHA-256(`SH:C03:EVENT-INIT:v1`) | **PASS** `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c` |
| C03-T01a prescribed event stream (kinds 1,7,2,7) independent digests | Packed in the verification log. These are wire digests of the published event table, **not** proof that the lifecycle that emits them is legal under §8 |
| M0-T07 initial checkpoint | Packed. Body SHA-256 `787a40d5eabadfeb9294851ee967dd0a81122fac44f91b04b961f1ced59f6f78`. Blob SHA-256 `bd803a2c4ae4320ce2a1a5ceb49a2941aec48921c4cbc0a60b15c607b62148c3` |
| Empty structural probe | Packed as a non-acceptance probe only |
| Post-tick-1 C03-T01a / M0-T06 checkpoints | **Not derived** — blocked by NF-01 |
| Full M0-T06 world with definition ids | **Not derived** — blocked by NF-02 |

Gameplay checkpoint hashes are therefore not established as implementation oracles beyond the M0-T07 initial state.

## 7. Completion provenance

| Requirement | Spec | Result |
| --- | --- | --- |
| Submitted and resolved ids | §3, §6 | Present |
| 41-byte registry | §3 | Present; independently sized |
| Lowest-key source-1 selection | §6 | Present |
| Role-1 / role-2 | §3, §6 | Present |
| Identical submitted replay no-op | §5 rules 5; acceptance §7 | Present |
| Different submitted id / payload → `ConflictingIdentity` | §5 rules 6–7; acceptance §7 | Present |
| `CompleteTask(0)` vs `CompleteTask(N)` | §6; acceptance §7 | Present |
| Save/restore of unconsumed list | §3, §10; acceptance §9 | Present |
| Exactly-once transaction | §6 | Present |

No new provenance blocker beyond the lifecycle contradiction in NF-01.

## 8. Inventory and transactions

| Requirement | Result |
| --- | --- |
| One owner per quantity | Present §3, §9 |
| Available = on-hand − other tasks' active reservations | Present §3, §6 |
| Atomic bundles | Present §6 |
| Class-1 via legs only; class-2 immutable quantity | Present §6 |
| Recipe binding | Present §6 |
| Definition/class on legs | Present §3 |
| Monotonic cursors; cross-table uniqueness | Present §§3.1–3.2 |
| Later-leg rollback; stock 14 first leg succeeds | Present acceptance §12; **CLOSED** for the prior stock-13 defect |
| Exactly-once release | Present §6 |

Gaps: NF-02, NF-04.

## 9. Scheduling and lifecycle

Phase order Mode → Command → Invalidation → Decision → Work is normative (§5). Creation-tick skip is first in work (§8). Invalidation clears `task_id`, preserves mid-edge locus, and moves unconsumed keys to role 2. Terminal cleanup and completion event order are present.

**Failing interaction:** NF-01 makes the reserved-to-executing path required by C03-T01a / M0-T06 illegal under the same work table.

## 10. Navigation

Approved graph: sites 1, 2, 9; edges 1 and 2 only; OC-EDGE absent. Grid tie north, north, east, east is normative. Unreachable site 9, mid-edge persistence, residual travel after cancel, residual travel while disabled, no teleport, and preserved remaining cost are present (§§8–9; acceptance §14). Independently closed for F08.

## 11. Five goals

| Goal | Eligibility / selection / due / terminal | Independent result |
| --- | --- | --- |
| Eat | Spec §7; acceptance §14 scores | Traceable except shared food `definition_id` installation for general fixtures (NF-02) |
| Rest | Spec §7 | Traceable |
| Work | Spec §7 | Blocked on tool/workstation discriminator (NF-03) |
| Haul | Spec §7 | Blocked on existing-stock credit rule (NF-04) |
| Return home | Spec §7 | Traceable |

DecisionTrace fields and wake interval behavior are present.

## 12. Acceptance coverage

| Exit | Independent result |
| --- | --- |
| C03-T01a | Event table packable; **lifecycle illegal under §8** (NF-01) |
| C03-T01b/c | Consistent with invalidation order |
| C03-T01d | Tick 1 inherits NF-01 |
| C03-T01e | Provenance rules consistent |
| C03-T01f | Later-leg and two-task reservation oracles consistent |
| C03-T01g/h/i | Schema/restore intent consistent; hostile-save matrix present |
| M0-T06 | Cursor and reservation id plan consistent; **Executing requirement illegal under §8** (NF-01); definitions missing (NF-02) |
| M0-T07 | Ten-tick sequence consistent with skip and stock-14 later-leg; initial checkpoint independently packed |
| M0-T08 | Cancel/disable/expiry/unreachable/replay covered; residual travel covered |
| M0-T07 task 405 | Debits stock 14 on staged copy; fails later invalid owner; rolls back whole tick-10 boundary; preserves ticks 1–9 — **PASS for this specific requirement** |

## 13. Frozen regression boundary

| Check | Result |
| --- | --- |
| C0.2 archive hash | Matches frozen baseline |
| No Bevy in `sealed_sim` deps | Pass |
| Fixture 2 disabled synthetic completion retained in normative §1 and OD-16 | Pass |
| Spec forbids production phases 3–5 for fixtures 1/2 while preserving fixture 2 work | Pass |
| C0.1/C0.2 goldens not regenerated by this audit | Pass (read-only) |

## 14. Balanced development plan

S1–S4 are internal slices. The plan and gate checklist keep the full C03-T01 and M0-T06/T07/T08 exit on S4 and do not authorize implementation before G3 PASS. Slices do not silently reduce the milestone exit. This structural point **passes**. It does not overcome NF-01.

## 15. Readiness verdict

| Question | Answer |
| --- | --- |
| Owner decisions reflected? | Yes, including OC-EDGE rejection |
| Prior F01–F09 all closed? | No. F07 and F09 remain open via NF-01; F06 partial |
| Complete deterministic acceptance executable without invention? | No |
| Independent goldens complete? | No. H0 and M0-T07 initial pack succeed; mandatory contention checkpoints do not |
| Implementation-ready? | No |

### VERDICT: FAIL

Required before any PASS re-review:

1. Normatively resolve NF-01 without inventing behavior in code (owner-approved due-tick promotion on reservation, or revised acceptance initial `due_tick` / work rules that remain consistent).
2. State M0-T06 `definition_id` values and Work tool/workstation discriminators.
3. Define credit behavior for existing non-zero stock ids used by haul.

Until those contracts are owner-approved and copied into the normative documents, independent G3 cannot pass, S1 must not start, and implementation remains unauthorized.
