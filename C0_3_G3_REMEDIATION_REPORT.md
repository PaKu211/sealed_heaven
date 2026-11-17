# C0.3 G3 Remediation Report

Version 2.1 · 17 November 2026  
Status: documentation remediation complete for R2.1-final owner approval and the NF-01–NF-04 blocker fix. Implementation is not authorized. G3 PASS is not declared.

Normative documents are version **1.2-approved**. The NF resolution matrix is `C0_3_G3_NF_RESOLUTION_MATRIX.md`. Verification: `C0_3_G3_NF_REMEDIATION_VERIFICATION.md`.

## 1. Approval chain

1. OD-01 through OD-16 approved from `C0_3_OWNER_DECISIONS_PENDING.md` version 1.2-proposal.
2. Independent G3 audit of the 1.0-approved normative docs returned FAIL (`C0_3_G3_INDEPENDENT_AUDIT.md`).
3. R2 then R2.1-final remediation proposed AM-01 through AM-08 without editing the normative specs.
4. The project owner approved AM-01 through AM-08 as documented in the R2.1-final packet, with ten binding conditions, and rejected OC-EDGE.
5. This remediation copies those approved contracts into the normative documents.

`C0_3_OWNER_DECISION_RECORD.md` version 2.0 is the approval record.

## 2. Changed files

| File | Change |
| --- | --- |
| `C0_3_OWNER_DECISION_RECORD.md` | Version 2.0. Records AM-01 through AM-08, OC-EDGE rejection, and ten binding conditions |
| `docs/C0_3_IMPLEMENTATION_SPEC.md` | Version 1.1-approved. Incorporates AM-01 through AM-08 |
| `docs/C0_3_ACCEPTANCE_TESTS.md` | Version 1.1-approved. Executable sequences from R2.1-final |
| `docs/C0_3_ARCHITECTURE_REVIEW.md` | Version 1.1-approved. Post-amendment readiness |
| `C0_3_G3_REMEDIATION_REPORT.md` | This report |
| `C0_3_OWNER_AMENDMENTS_PENDING.md` | Marked APPROVED with OC-EDGE REJECTED |

Not modified: Rust sources, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, C0.1 and C0.2 golden vectors, the C0.2 archive, and Bevy integration.

## 3. Owner decision traceability

| ID | Normative location |
| --- | --- |
| OD-01 through OD-16 | Implementation spec as previously mapped, as amended below |
| AM-01 | Spec §§5, 8, 12. Acceptance §12 |
| AM-02 | Spec §§3, 5, 6. Acceptance §§7, 9 |
| AM-03 | Spec §§3, 6. Acceptance §§8, 12 |
| AM-04 | Spec §§3.1, 3.2, 12. Acceptance §§3, 11, 12 |
| AM-05 | Spec §5. Acceptance §§5, 7 |
| AM-06 | Spec §7. Acceptance §14 |
| AM-07 | Spec §§5, 8. Acceptance §§3, 11, 12 |
| AM-08 | Spec §§8, 9. Acceptance §14. OC-EDGE absent from §12 |
| Binding conditions 1–10 | Decision record §2. Spec §§1, 5, 6, 8, 9, 11, 12 |

## 4. Audit finding closure in documentation

| Finding | Documentation disposition |
| --- | --- |
| F01 | Creation-tick skip kept. M0-T07 is the ten-tick table. Task 405 later-leg uses stock 14 |
| F02 | Consumed registry stores submitted and resolved ids. Lowest key is the only transaction |
| F03 | Legs carry definition and class. Holders table. Other-task reservations protected. Later-leg rollback |
| F04 | One `next_record_id`. Cross-table collision rejected. C03-T01a starts at 3. M0-T06 starts at 21 |
| F05 | First-match ingress. Submitted-id no-op. Different payload is `ConflictingIdentity` |
| F06 | Five goal plans with due, expiry, and enable reschedule |
| F07 | `Queued` overdue exempt. Tick 9/10 later-leg executable. Work events retained |
| F08 | Residual edge travel. No teleport. Approved graph unchanged. OC-EDGE rejected |
| F09 | Acceptance sequences name initial state and expected results. Hashes withheld for independent packing |

These closures are documentation closures. They are not an independent G3 PASS.

## 5. Consistency checks performed

| Check | Result |
| --- | --- |
| H0 unchanged | Pass. `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c` |
| Creation-tick skip present and ordered first in work | Pass |
| `completion_requested` flag removed from normative schema | Pass. Replaced by unconsumed list and consumed registry |
| Stock 13 not used as task 405 first leg | Pass. Stock 14 is the available debit |
| OC-EDGE absent from fixture graph | Pass |
| Fixture 2 synthetic completion preserved | Pass |
| C0.1/C0.2 goldens not regenerated | Pass. Not touched |
| Full C03-T01 and M0-T06/T07/T08 retained | Pass |
| Bevy prohibited | Pass |
| Slices named without treating them as exits | Pass |
| No Rust implementation started | Pass |

## 6. Remaining risks for independent G3

| Risk | Why it remains |
| --- | --- |
| Checkpoint SHA-256 values are not frozen yet | They require an independent packer during G3 |
| Fixture constants can be mistaken for final balance | The owner condition says they are fixture configuration only |
| Prior proposal files still describe historical unapproved states | The decision record version 2.0 and normative 1.1-approved docs supersede them |
| C0.2 suite was not re-executed in this documentation pass | The source was not modified. The next implementation authorization still owes regression on the frozen goldens |

## 7. Recommendation

The revised documents are ready for the mandatory independent G3 re-review. They are not ready for coding. Implementation requires that review and a separate project-owner authorization naming a slice. S1 is not started. C0.4 is not started. Bevy is not started.

**Documentation readiness: READY FOR INDEPENDENT G3 REVIEW. Implementation readiness: NOT AUTHORIZED. G3 PASS: NOT DECLARED.**
