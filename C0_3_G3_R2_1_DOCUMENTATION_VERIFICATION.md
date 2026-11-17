# C0.3 G3 R2.1 Documentation Verification

Version R2.1-final · 17 November 2026  
Status: documentation check. This report does not declare G3 PASS and does not authorize implementation.

## Scope that was edited in R2.1-final

| File | Action |
| --- | --- |
| `C0_3_G3_R2_OWNER_AMENDMENTS_PENDING.md` | R2.1-final. Later-leg stock 14, residual travel, submitted payload id |
| `C0_3_G3_R2_REMEDIATION_PROPOSAL.md` | R2.1-final. Matching technical text |
| `C0_3_G3_R2_ACCEPTANCE_RECONCILIATION.md` | R2.1-final. Matching sequences |
| `C0_3_G3_R2_FINDING_MATRIX.md` | R2.1-final |
| `C0_3_OWNER_AMENDMENTS_PENDING.md` | Pointed at R2.1-final |
| `C0_3_G3_FINDING_RESOLUTION_MATRIX.md` | Aligned |
| `C0_3_VERTICAL_SLICE_TEST_PLAN.md` | S1/S3 oracles updated |
| `C0_3_BALANCED_DEVELOPMENT_PLAN.md` | Version pointer only |
| `C0_3_IMPLEMENTATION_GATE_CHECKLIST.md` | Version pointer only |
| This file | Updated |

Not edited: normative C0.3 specs, Rust sources, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, and C0.1/C0.2 goldens.

## Exact corrections

### 1. M0-T07 later-leg rollback

| Before R2.1-final | After |
| --- | --- |
| Task 405 transform-out stock 13, then credit owner 0 | Task 405 debits stock 14 (available after the successful recipe), then credits owner 0 |
| First leg would fail for depleted stock 13 | First leg succeeds on the staged copy. Second leg fails. Whole boundary rolls back |
| Risk: wrong failure mode | Expected failure is specifically the later leg. Ticks 1–9, including stock 14 quantity 1, are preserved |

### 2. Mid-edge invalidation

| Before R2.1-final | After |
| --- | --- |
| Cancel left locus 3, but only task-owned work could spend cost | Residual work-phase travel for `locus` 3 and `task_id` 0 |
| No phase owner after `task_id` clear | Work phase, after task-owned work, ascending actor id |
| — | One cost per tick, no events, arrival on destination site, enabled or disabled, save restores mid-edge |

### 3. Completion payload identity

| Before R2.1-final | After |
| --- | --- |
| Registry matched on resolved task id only | Registry stores `submitted_task_id` and `resolved_task_id` |
| `CompleteTask(0)` and `CompleteTask(401)` could no-op each other | Same key with different submitted id is `ConflictingIdentity` |
| Record 33 bytes | Record 41 bytes. Bounded by unread body |

## Regression implications

| Surface | Impact |
| --- | --- |
| C0.1 / C0.2 goldens | None. Untouched |
| H0 | None |
| Fixture 2 synthetic completion | None |
| Published C0.3 acceptance tables | Still stale until a post-approval normative edit |
| S1 tests | Must pack the 41-byte registry and the stock-14 later-leg oracle |
| S3 tests | Must include residual arrival after cancel, including a disabled residual tick |
| Gameplay hashes | Still withheld |
| Architectural reopenings | None. Skip, shared cursor, five goals, and approved graph stay |

## Readiness

The R2.1-final packet is ready for an owner decision. The normative specification is not repaired. Implementation is not authorized. G3 is not declared passed.
