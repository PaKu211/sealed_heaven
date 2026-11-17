# C0.3 G3 v1.2 Independent Audit

Date: 17 November 2026  
Role: Independent Principal Rust Simulation Architect and Deterministic Systems Auditor  
Reviewed specification: `docs/C0_3_IMPLEMENTATION_SPEC.md` version **1.2-approved**  
Acceptance: `docs/C0_3_ACCEPTANCE_TESTS.md` version **1.2-approved**  
Companion log: `C0_3_G3_V1_2_VERIFICATION_LOG.txt`

**Verdict: FAIL**

**Implementation remains NOT AUTHORIZED. S1 remains NOT STARTED. G3 PASS is not declared.**

This audit did not author the specification. No normative document, Rust source, golden vector, lockfile, or archive was modified. Packing used a separate Python reference packer, not `sealed_sim`. Developer verification reports were not treated as proof.

## 1. Executive conclusion

NF-01 through NF-04 are **closed** in the normative 1.2 text. The previously failing lifecycle for C03-T01a, C03-T01d tick 1, and M0-T06 is consistent with work overdue rules once §6.1 assigns `due_tick` before work. Resource definitions, Work discriminators, and existing-stock credit rules are explicit. H0 verifies. The published M0-T07 initial checkpoint hash re-derives byte-identically. C0.2 archive identity and Bevy exclusion hold. F01–F08 remain closed under the revised contracts.

The audit still **cannot PASS**. Acceptance §§3 and 11 still omit required task-record fields for preinstalled Queued tasks (`goal`, `recipe_id`, `interruption`, target locus/site/x/y, and a fully determined `progress_ticks` baseline before work). Without those fields, unique independent checkpoint goldens for C03-T01a and M0-T06 cannot be derived without inventing data. Task 9 of this review therefore fails the packing gate that forbids PASS when required goldens cannot be derived from the contract alone.

## 2. Evidence identities

| File | Bytes | SHA-256 |
| --- | --- | --- |
| `docs/C0_3_IMPLEMENTATION_SPEC.md` | 33536 | `103360f17ff971f394dbf3783b4f1524bf27084d686b5b6c2a9bbb49a108d6c4` |
| `docs/C0_3_ACCEPTANCE_TESTS.md` | 16303 | `248539164ab3291895a02a4dfff65ed46b2a59d2c1c1186beb3bf5f2ce268211` |
| `docs/C0_3_ARCHITECTURE_REVIEW.md` | 2311 | `8fb5a8ae2d5457b0830584b242b7049bbbcd5616eb9cfbf011afcb3498658d7c` |
| `C0_3_G3_NF_RESOLUTION_MATRIX.md` | 2521 | `41078e060036d606603eff0f0dc4b9ab7cd08246bba26877675b92aeb98bed15` |
| `C0_3_OWNER_DECISION_RECORD.md` | 8058 | `55db85c55dd1044309e77314547e8e1f0e113a20b1ad12eac8dde9fe76d018b2` |
| `C0_3_G3_FINAL_INDEPENDENT_AUDIT.md` (prior FAIL) | 16344 | `71a06bc1ddf209e3153e4b4c927eb4a38ee5f92acf860b77b8133dc6edcad56e` |

| Artifact | Result |
| --- | --- |
| `Sealed_Heaven_C0_2_Source.zip` | SHA-256 `2804d33a785acb7331a0d82fd1c080bec82d0add348578abb4b1a6807db60a40` |
| H0 | **PASS** `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c` |
| M0-T07 initial blob | **PASS** re-pack matches `bd803a2c4ae4320ce2a1a5ceb49a2941aec48921c4cbc0a60b15c607b62148c3` |

## 3. NF-01 through NF-04 closure

| ID | Independent check | Result |
| --- | --- | --- |
| NF-01 | Spec §6.1 assigns `due_tick = staged_tick + travel_ticks + 2` in the reservation commit before work; `created_tick` unchanged. C03-T01a: due 3, staged 1, rule 3 does not fire, rule 4 → Executing. M0-T06 winners identical. Creation-tick skip still keys only on `created_tick`. Rollback undoes the assignment | **CLOSED** |
| NF-02 | Spec §12 and acceptance §11: 101→def 3 class 2; 102→4/2; 103→1/1; 104→5/2 | **CLOSED** |
| NF-03 | Spec §7: tool = lowest class-2 definition 4; workstation = lowest class-2 definition 5; definitions differ | **CLOSED** |
| NF-04 | Spec §6.2: existing stock requires definition, class, and owner match; checked add; overflow/`InvalidLeg`; `stock_id` 0 allocates; class-2 quantity change on existing stock is `InvalidLeg`. Acceptance §8 names match and mismatch cases | **CLOSED** |

## 4. Previously failing scenarios

### C03-T01a

Initial Queued tasks with `due_tick` 0. Actor 1 reserve succeeds → §6.1 sets `due_tick` 3, stage Reserved → work Executing. Actor 2 rejected. Event order kind 1, 7 Reserved, 2, 7 Executing. Independently packed event-stream SHA-256 `6dddae6663e2d9107bfa9aa096c65ce9228c3b80da60864292ad1ca90c587bfe` (143 bytes, next cursor 5).

**Lifecycle: PASS.** **Post-boundary checkpoint golden: FAIL uniqueness** (NF-05).

### C03-T01d tick 1

Same promotion: `due_tick` 3, Executing present. Later disable path unchanged. **Lifecycle: PASS.** Same packing gap for the tick-1 save if required.

### M0-T06

Definitions explicit. Winners 1–5 get `due_tick` 3 before work and Executing events. Losers stay Queued with `due_tick` 0 (rule 2 exempt). Supplies unchanged. **Lifecycle: PASS.** **Full-world checkpoint golden: FAIL uniqueness** (NF-05).

### M0-T07

Ten-tick table consistent with creation-tick skip and stock-14 later-leg rollback. Initial checkpoint re-packs to the published hash. **PASS.**

### M0-T08

Cancel/disable/expiry priority, unreachable, residual travel, replay no-ops remain consistent. **PASS** for stated procedures.

## 5. F01–F09 under v1.2

| ID | Disposition |
| --- | --- |
| F01 | Closed — creation-tick skip + ten-tick M0-T07 |
| F02 | Closed — submitted/resolved registry |
| F03 | Closed — leg definition/class; stock-14 later-leg |
| F04 | Closed — shared cursor; explicit starts |
| F05 | Closed — first-match ingress |
| F06 | Closed — five plans + NF-02/03 definitions |
| F07 | Closed for the due_tick/Executing contradiction via §6.1 |
| F08 | Closed — residual travel; OC-EDGE absent |
| F09 | **Open residual** — event oracles for C03-T01a pack; full C03-T01a/M0-T06 checkpoint oracles still require invented task fields (NF-05) |

## 6. New finding

### NF-05 — HIGH — Preinstalled task fields incomplete for unique packing

**Normative sections.** Spec §3 task record; acceptance §§3 and 11 initial states.

**Concrete input.** C03-T01a installs tasks 1 and 2 as `Queued` with `created_tick` 0 and `due_tick` 0, then expects a post-tick-1 checkpoint with task 1 `Executing` and `due_tick` 3. The acceptance text does not state `goal`, `recipe_id`, `interruption`, target (`locus`, `site`, `x`, `y`), or the pre-work `progress_ticks` for those tasks. M0-T06 repeats the omission for tasks 1–20.

**Expected.** Every byte of a required acceptance checkpoint is determined by the contract.

**Specified.** Lifecycle and reservation fields are determined. Several task header fields are not.

**Minimal correction.** Name those fields in acceptance §§3 and 11 (and M0-T06), e.g. `goal` 0 or a stated ordinal, `recipe_id` 0, `interruption` 0, target locus 1 site 1, `progress_ticks` 0 before work, and `progress_ticks` 1 after the Reserved→Executing transition if that transition counts.

**Blocks implementation authorization?** Yes for freezing unique C03-T01a / M0-T06 checkpoint goldens. No for the prior due_tick correctness defect (already closed).

A trial pack that invents `goal=0` and target site 1 produced blob SHA-256 `11f9aae46d7f0f04b6c3efead0a25c3086bc56904877d237c0a79ae032a293d9`. That value is **not** an acceptance oracle.

## 7. Other verified surfaces

| Surface | Result |
| --- | --- |
| Reservation promotion before work; atomic with accept | Pass §6.1 |
| Exactly-once release and completion replay | Pass §§5–6; acceptance §§7–8 |
| Resource conservation / later-leg rollback | Pass §§6.2, 12 |
| Residual edge travel; OC-EDGE rejected | Pass §§8–9, 11–12 |
| Fixture 2 synthetic completion isolation | Pass §1 |
| Bevy absent from kernel deps | Pass |
| Full exit list C03-T01 and M0-T06–T08 retained | Pass |
| S1–S4 remain non-exits | Pass |
| Reopening AM-01–AM-08 / OC-EDGE | No evidence of correctness defect requiring reopen |

## 8. Golden packing summary

| Vector | Status |
| --- | --- |
| H0 | Pass |
| C03-T01a event stream | Pass (wire digests in log) |
| M0-T07 initial checkpoint | Pass (matches published hash) |
| C03-T01a post-tick-1 checkpoint | **Not uniquely derivable** (NF-05) |
| M0-T06 full-world checkpoint | **Not uniquely derivable** (NF-05) |

## 9. Readiness

| Question | Answer |
| --- | --- |
| NF-01–NF-04 closed? | Yes |
| Prior F01–F08 closed? | Yes |
| F09 fully closed? | No — residual NF-05 |
| Complete required goldens without invention? | No |
| Implementation-ready? | No |

### VERDICT: FAIL

Required before PASS: close NF-05 by stating every task-record field for C03-T01a and M0-T06 initial and post-promotion states so an independent packer can emit unique checkpoint oracles without invention.
