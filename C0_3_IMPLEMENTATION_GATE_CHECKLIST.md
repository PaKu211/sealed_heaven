# C0.3 Implementation Gate Checklist

Version 1.3-approved · 17 November 2026  
Status: governance. Normative docs are 1.3-approved (NF-01–NF-05 remediated). Implementation is not authorized. This checklist does not declare G3 PASS.

## 1. Decision classes

| Class | Examples | What happens |
| --- | --- | --- |
| A. Owner amendment | Any change to H0, a frozen golden, fixture 2, a phase, the creation-tick skip, OC-EDGE, or an approved AM contract | Stop. Record a pending amendment. Do not code it |
| B. Determined by contract | Field order, command-key sort, the AM-05 table, reason bytes, the five phases, producer 1, whole-boundary rollback, residual travel | Implement as written after G3 PASS and slice authorization |
| C. Ordinary defect | An offset, a sort, or a cursor update disagrees with the approved text | Fix inside the authorized slice. Do not reopen the architecture |

## 2. Gates before any Rust edit

| Check | State today |
| --- | --- |
| C0.2 archive hash `2804d33a785acb7331a0d82fd1c080bec82d0add348578abb4b1a6807db60a40` | Frozen |
| OD-01 through OD-16 | Approved |
| AM-01 through AM-08 | Approved |
| OC-EDGE | Rejected |
| Normative spec updated to the approved rows | Done. Version 1.3-approved (includes NF-01–NF-05) |
| Independent G3 re-review of that repaired spec | Required next on 1.3-approved. This file does not pass it |
| Owner authorization naming the slice | Absent |
| Bevy absent from `sealed_sim` | Required and currently true |
| `Cargo.lock` and Rust 1.100.0 unchanged | Required |

## 3. Per-slice authorization

The owner may authorize S1, S2, S3, and S4 separately after G3 PASS. Authorizing one slice does not authorize the next. A slice exit is not the C0.3 exit.

| Slice | Cannot start unless |
| --- | --- |
| S1 | Independent G3 PASS, then owner names S1 |
| S2 | S1 exit, owner names S2 |
| S3 | S2 exit, owner names S3 |
| S4 | S3 exit, owner names S4 |

The C0.3 milestone exit is S4 plus the frozen C0.1 and C0.2 suites. Neither a slice exit nor the milestone exit starts C0.4.

## 4. Verification the implementer owes

Inside an authorized slice, in order: codec tests against the independent pack, state-machine tests, the two-task conservation oracle, the stock-14 later-leg rollback, exactly-once release, save and restore of the 41-byte consumed registry, replay of role 1 and role 2 including submitted-id conflict, residual mid-edge arrival, both command orders, the slice's integration rows, then C0.1 and C0.2. Debug and release.

## 5. Current verdict

Documentation is ready for independent G3 re-review. Implementation is not authorized. S1 is not started. G3 PASS is not declared.
