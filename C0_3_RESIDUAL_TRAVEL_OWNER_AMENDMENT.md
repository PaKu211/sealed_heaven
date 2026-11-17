# C0.3 Residual Travel Owner Amendment — RT-01

Date: 18 November 2026
Branch: `feat/c03-s4-integration`
Audited baseline: `1dbe07e632efa55596cb33e17d6470afa0e0824c`
Authority: the owner's explicit **Option B — next-boundary residual movement** instruction, recorded in `C0_3_OWNER_DECISION_RECORD.md` version 2.2, RT-01.

## Contradiction confirmed

Before this change, implementation specification §5 cleared a cancelled task during invalidation, then §8 required residual work for every on-edge/taskless actor. This implied movement in the same successful boundary. Acceptance §14 instead required a post-cancel checkpoint still on edge 1 at traversed cost 0 and arrival on the next boundary. An atomic `drive` cannot expose both checkpoint states. The final independent audit classified this as F01 and returned INCONCLUSIVE.

Evidence read: `C0_3_FINAL_INDEPENDENT_AUDIT.md` and `C0_3_RESIDUAL_TRAVEL_RECONCILIATION.md`, retained outside the repository under `C:\Users\PC_4\AppData\Local\Temp\opencode\sealed-heaven-c03-final-audit-1dbe07e`. These historical independent reports have not been edited or relabelled.

## Binding amendment

1. In successful staged tick `T`, cancellation is processed during invalidation under the existing reason precedence.
2. A task actually cancelled (reason 2) follows the existing terminal transition, releases/provenance handling and `task_id` clearing.
3. The actor's mid-edge locus, edge id and traversed edge cost are preserved.
4. Clearing that task by cancellation does not cause residual movement in tick `T`.
5. Starting at the next successful boundary, staged tick `T+1`, the existing residual work rule may spend one edge-cost unit per tick.
6. Remaining cost is never discarded.
7. Disabled actors still perform residual movement.
8. Arrival selects the destination site, clears the edge id and resets traversed cost to zero.
9. Residual movement/arrival emit no event. Other approved events remain unchanged.
10. Save/restore preserves exact mid-edge bytes and continuation, without early movement, an extra wait, repeated events or hidden readiness state. Failed calls preserve the complete pre-call state and tick; successful calls advance one logical tick.

This supersedes **only** the conflicting same-boundary cancellation reading of §8. The broad audit suggestion to qualify all newly taskless actors is **not** adopted as a new general rule. Other interruption/residual cases, unrelated task transitions, completion, navigation ordering, reason precedence, and all frozen vectors remain unchanged. If disablement outranks a simultaneous cancellation, reason 1 remains governed by its existing rules, not by this reason-2 exception.

## Compatibility and implementation

No other binding condition conflicts with this amendment. OD-14's invalidation precedence, OD-10/AM-01's work restrictions, OD-11/AM-08's exact position/cost preservation, and the canonical/save/replay/frozen-vector conditions remain in force. AM-08 now cross-references RT-01 specifically for cancellation timing.

Production Rust already conforms: `fixture3::drive` collects residual eligibility from the pre-boundary world; cancellation in the staged invalidation phase leaves position intact; `work` resumes already-taskless actors on the next boundary, including disabled actors. No production Rust, dependency, toolchain, schema, graph or golden file is changed.

Two focused regressions in `src/tests/c03_s3.rs` check:

- Exact cancellation event bytes, no same-tick travel, tick-1 checkpoint, next-boundary enabled/disabled arrival, save/restore at the relevant boundaries, silent continuation and stable cursor/digest. The three checkpoint hashes and event digest were independently packed during the prior audit, not generated as new expected values by this implementation change.
- Invalid next-boundary input rolls back completely and does not spend residual cost; the valid disabled retry arrives without a restore-induced extra wait.

The authored graph remains cost-1 edges 1/2 and sites 1/2/9. These tests install the approved legal mid-edge snapshot; they do not invent an observable partial-travel boundary on a cost-1 edge or introduce OC-EDGE.

## Authorization and next gate

The owner authorizes targeted documentation/tests, required verification, commit and push to this feature branch. No merge into main, C0.4, Bevy, golden replacement or independent C0.3 PASS is authorized or claimed. See `C0_3_RESIDUAL_TRAVEL_VERIFICATION_LOG.txt` for command outputs and `C0_3_FINAL_GATE_RECONCILIATION.md` for the requested targeted independent re-review.
