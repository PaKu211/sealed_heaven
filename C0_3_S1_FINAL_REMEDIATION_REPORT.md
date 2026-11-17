# C0.3 S1 Final Remediation Report

**Baseline:** `552ed25` — "Implement the C0.3 S1 resource core."
**Branch:** `fix/c03-s1-independent-audit`
**Date:** 17 November 2026
**Supersedes:** `C0_3_S1_REMEDIATION_REPORT.md` and the blocked H09 status in `C0_3_S1_FINDING_CLOSURE_MATRIX.md`.
**Scope:** B01 and H01–H10 only. No S2, S3, S4, or Bevy. No push or merge. No independent S1 PASS is declared.

**Result:** **B01 and H01–H10 are CLOSED.** H09 was closed by the owner's Option B amendment. All 59 unit tests and 2 doctests pass in debug and release, H0/T01a and the frozen C0.1/C0.2 vectors reproduce, and the checkpoint revision and published bytes are unchanged.

---

## 1. Final disposition

| Finding | Severity | Final status | Closing evidence |
| --- | --- | --- | --- |
| B01 | Blocker | **CLOSED** | Semantic restore validator; 57/57 restore expectations |
| H01 | High | **CLOSED** | Count/grid/framing regressions |
| H02 | High | **CLOSED** | Batch and future Begin ownership regressions |
| H03 | High | **CLOSED** | Same-phase zero-completion regressions |
| H04 | High | **CLOSED** | Checked aggregate reservation regressions |
| H05 | High | **CLOSED** | First-match precedence regression |
| H06 | High | **CLOSED** | Actor-then-task invalidation regression |
| H07 | High | **CLOSED** | Schema/new-stock class regressions |
| H08 | High | **CLOSED** | Scenario fidelity and rejection regressions |
| H09 | High | **CLOSED** | Owner amendment, spec/acceptance reconciliation, 2 replay regressions |
| H10 | High | **CLOSED** | Normative §8/§8.1 clarification and 4 regressions |

No finding remains blocked.

---

## 2. What changed for H09

The owner approved H09 Option B and recorded it in `C0_3_OWNER_DECISION_RECORD.md` §2A. The amendment:

- narrows binding condition 4 for fixture 3 to durable identities;
- preserves the 41-byte completion registry and checkpoint revision 1;
- removes the requirement to persist every historically applied non-completion key;
- defines old-target reuse as `LateTarget` after earlier structural validation;
- permits no hidden replay cache and requires identical future behavior from identical bytes.

`docs/C0_3_IMPLEMENTATION_SPEC.md` §5 row 7 now covers only the consumed registry. A following paragraph defines durable identities, old-target `LateTarget`, future-target ordinary commands, and resume equivalence.

`docs/C0_3_ACCEPTANCE_TESTS.md` §9A adds positive old-target replay, negative structural cases, future-target admission, and save/restore equivalence.

**No production Rust change was required.** The existing implementation already behaved this way. Two focused regressions lock the contract:

- `h09_applied_noncompletion_replay_is_late_and_identity_free`
- `h09_pending_duplicate_and_future_target_follow_first_match`

The external audit case `applied_noncompletion_key_reuse` still restores `OK` and returns `LateTarget`; that is now the approved result, replacing the audit's prior `ConflictingIdentity` expectation for fixture 3.

Detailed evidence: `C0_3_S1_H09_RESOLUTION.md`.

---

## 3. Preserved corrections from the prior remediation

| Finding | Correction summary |
| --- | --- |
| B01 | `src/fixture3/validation.rs` rejects invalid cursors, links, duplicates, registry rows, reservations, transactions, pending, and traces before publication |
| H01 | Bounded counts, grid limits, truncation/checksum/parse/content precedence |
| H02 | Batch-aware scheduled Begins and execution-time ownership recheck |
| H03 | Zero completion requires an actual same-phase Begin |
| H04 | Checked aggregate holds including the requesting task |
| H05 | Producer → actor → duplicate/pending order before late target |
| H06 | Invalidation sorted by actor, then task |
| H07 | Full Begin/Task/leg domain validation at ingress and application |
| H08 | Scenario retained and validated; altered consistent content rejected |
| H10 | Goal 0 is scripted; request promotes one transition; no deadline extension; §8.1 |

---

## 4. Verification

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | exit 0 |
| `cargo check --locked --offline` | exit 0 |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0 |
| `cargo test --locked --offline` | **59 unit + 2 doctests passed** |
| `cargo test --release --locked --offline` | **59 unit + 2 doctests passed** |

Test composition: 14 C0.1 + 11 C0.2 + 6 C03 S1 + 28 remediation (26 prior + 2 H09) = 59 unit tests.

### Oracles

- H0: `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c`
- T01a body: `11f9aae46d7f0f04b6c3efead0a25c3086bc56904877d237c0a79ae032a293d9`, length 1262
- T01a digest: `5fdbc5707211fbfc309f5ed75e21eaa83ebf3d7307da60cc442a3cd7f8340908`
- T01a forward and reverse restore/reload byte-identical
- C0.1/C0.2: 25 tests pass; no source or golden-vector diff

### Independent audit corpus

- 99 cases in debug and release
- 31 unchanged / 68 intentionally changed
- 57/57 named restore expectations PASS (baseline 6/57)
- 0 target panics
- Debug and release comparison files identical

The H09 case remains `LateTarget` by the approved amendment.

---

## 5. Preserved invariants

- Checkpoint tag `SH:C03:STATE`, revision `1`, fixture `3`.
- 305-byte scenario block, 131-byte actor records, 41-byte completion registry.
- T01a tick-1 blob and digest unchanged.
- H0 unchanged.
- C0.1/C0.2 behavior and vectors frozen.
- Deterministic integer simulation, `BTreeMap`/`BTreeSet`, explicit ordering, atomic rollback.
- No hidden replay cache; identical canonical bytes produce identical future behavior.
- `Cargo.toml`, `Cargo.lock`, and `rust-toolchain.toml` unchanged; toolchain `1.100.0`.

---

## 6. Files changed in this final correction

| File | Change |
| --- | --- |
| `C0_3_OWNER_DECISION_RECORD.md` | Version 2.1; condition 4 narrowed; §2A H09 amendment |
| `docs/C0_3_IMPLEMENTATION_SPEC.md` | §5 row 7 narrowed; durable-identity paragraph; H09 status note |
| `docs/C0_3_ACCEPTANCE_TESTS.md` | §9A H09 replay cases; coverage row |
| `src/tests/c03_remediation.rs` | 2 H09 regressions; `ProcessingMode` import |
| `C0_3_S1_H09_RESOLUTION.md` | added |
| `C0_3_S1_FINAL_REMEDIATION_REPORT.md` | added |
| `C0_3_S1_FINAL_VERIFICATION_LOG.txt` | added |

The earlier B01/H01–H10 source corrections and their 26 tests remain in place. No production Rust file was changed for H09.

---

## 7. Remaining limits

- This report does not declare independent S1 PASS. Independent S1 re-audit is the required next step.
- S2, S3, S4, Bevy, autonomous decisions, needs, scoring, and full navigation remain out of scope and unimplemented.
- The H09 contract intentionally does not retain identity for applied non-completion commands after they leave canonical state. Future-target reuse is an ordinary new command; old-target reuse is `LateTarget`.
- No push or merge was performed. `/main` remains untouched.

---

## 8. Requested next action

Stop here and request an **independent S1 re-audit** covering:

1. the H09 amendment and §9A cases;
2. the unchanged checkpoint revision, 41-byte registry, H0, and T01a bytes;
3. the 59-test debug/release suite and the 99-case external corpus;
4. the remaining scope restrictions before any S1 acceptance decision.
