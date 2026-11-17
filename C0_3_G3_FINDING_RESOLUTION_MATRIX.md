# C0.3 G3 Finding Resolution Matrix

Version 1.2-approved · 17 November 2026  
Status: F01–F09 addressed in 1.1; NF-01–NF-04 remediated in 1.2. Implementation is not authorized. G3 PASS is not declared.

Audit: `C0_3_G3_INDEPENDENT_AUDIT.md` and `C0_3_G3_FINAL_INDEPENDENT_AUDIT.md`. Normative copy: `docs/C0_3_IMPLEMENTATION_SPEC.md` version 1.2-approved. NF matrix: `C0_3_G3_NF_RESOLUTION_MATRIX.md`. H0 remains `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c`.

| ID | Correction now normative | Approval | Tests |
| --- | --- | --- | --- |
| F01 | Creation-tick skip first. M0-T07 ten-tick table. Task 405 later-leg debits stock 14 | AM-01 APPROVED | Acceptance §12 |
| F02 | 41-byte consumed registry with submitted and resolved ids. Lowest key only as transaction | AM-02 APPROVED | Acceptance §§7, 9 |
| F03 | Leg definition/class, holders, other-task reservation protection, later-leg rollback | AM-03 APPROVED | Acceptance §§8, 12 |
| F04 | Shared `next_record_id`. Cross-table collision rejected. Cursors 3 / 21 / 401 | AM-04 APPROVED | Acceptance §§3, 11, 12 |
| F05 | First-match ingress. Submitted-id no-op. Different payload is `ConflictingIdentity` | AM-05 APPROVED | Acceptance §§5, 7 |
| F06 | Five goal plans, due/expiry, enable reschedule | AM-06 APPROVED | Acceptance §14 |
| F07 | Queued overdue exempt. Tick 9/10 later-leg. Work events retained | AM-07 APPROVED | Acceptance §§3, 11, 12 |
| F08 | Residual edge travel. Approved graph unchanged. OC-EDGE rejected | AM-08 APPROVED | Acceptance §14 |
| F09 | Executable sequences in acceptance 1.1-approved. Hashes withheld for independent packing | Follows AM-01–AM-08 | All retained exits |

Fixture 2 synthetic work is unchanged.
