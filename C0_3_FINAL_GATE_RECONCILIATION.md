# C0.3 Final Gate Reconciliation — RT-01

Date: 18 November 2026
Branch: `feat/c03-s4-integration`
Audited baseline: `1dbe07e632efa55596cb33e17d6470afa0e0824c`

## Status

**Owner reconciliation and targeted independent re-review complete: RT-01 PASS.** This document records the request and pre-review status; the completed review and milestone evidence are archived in `C0_3_RT01_INDEPENDENT_REVIEW.md`.

## F01 disposition

| Evidence | Before | Targeted reconciliation |
| --- | --- | --- |
| Owner authority | No decision selecting the incompatible residual boundary oracles | Explicit cancellation-only Option B, RT-01 in decision record 2.2 |
| Implementation §8 | Unqualified every-on-edge/taskless-actor residual work implied same-boundary cancellation movement | Express reason-2 cancellation exception: preserve tick-T position; start residual work at T+1 |
| Implementation §9 | Referred cancellation and failure to residual work | Cancellation specifically cross-references §8 RT-01; failure rules unchanged |
| Acceptance §14 | Post-cancel on-edge save, next-boundary arrival | Original four steps retained; exact boundary/key/event/restore/error-retry timing clarified |
| Production Rust | Already deferred newly cancelled actors until the next boundary | Unchanged |
| Canonical state/events | Audited residual bytes and published vectors | Existing bytes/digests preserved; focused comparisons added |

No other binding owner condition was found incompatible. This amendment does not change other causes of task-id clearing, the invalidation priority, task-owned work, completion identities, or other residual-work cases. It is narrower than adopting a new boundary-start eligibility rule for every possible interruption.

## Reconciled exact trace

Initial snapshot: tick 0, seed 100, actor 1 enabled, edge 1/cost 1/traversed 0, travelling Return Home task 50/due 4, no reservations, cursor 1/H0. Approved graph unchanged.

| Committed tick | Input | Position/task | Events/cursor |
| --- | --- | --- | --- |
| 1 (`T`) | `drive(0, [CancelReservation(0)])`, key `(1,1,1,1)` | locus 3, site 0, edge 1, traversed 0, task_id 0; task 50 Cancelled/reason 2 | id 1 kind 7 Cancelled; id 2 kind 5 reason 2; cursor 3 |
| save/restore at 1 | exact checkpoint | same mid-edge/taskless bytes | no prefix events |
| failed tick-2 attempt | invalid `SetEnabled(2)` at boundary 1 | no committed tick advance or movement; complete bytes unchanged | InvalidPayload; cursor/digest unchanged |
| 2 (`T+1`) | empty drive, or `SetEnabled(0)` key `(2,1,1,2)` | site 2, edge 0, traversed 0, task_id 0; disabled in the latter branch | no events; cursor/digest unchanged |
| save/restore at 2, then tick 3 | empty drive | remains at endpoint; no task created before stored wake | no repeated arrival/event |

The failure/retry regression uses the same edge/task shape with its original due 3; the prior-audit byte-vector regression uses due 4 exactly. Terminal due time has no residual effect and neither test alters production rules.

## Canonical evidence retained

Prior independently packed residual reference values asserted by the new regression:

| Observation | SHA-256 / chained digest |
| --- | --- |
| Post-cancel checkpoint, tick 1 | `5a632f69783424998b68fab64cd9101fc0e8747a46ef0c598be09e7497d130b6` |
| Enabled arrival checkpoint, tick 2 | `3692e9b96d2cf046f1e251ac2663a81dc6fe15e18506bc0c43868be87bb4ee32` |
| Disabled arrival checkpoint, tick 2 | `c8b2c75819bd12c6f8e38a16c404933bbc8491a96aad7bff587a059e5033d101` |
| Digest after cancellation and silent continuation | `bdc5acacb73293eaea7d3e63d8062b8c6a639e4f6d22071e2a6aee10c3682d68` |

Frozen H0, C03-T01a, M0-T06 and M0-T07 initial values are not replaced. The required debug/release regression suites and the contract-reference packer check retain their original assertions. Exact results and preservation comparisons are in `C0_3_RESIDUAL_TRAVEL_VERIFICATION_LOG.txt`.

## Targeted independent re-review — completed

Please review the resulting feature-branch commit against audited baseline `1dbe07e632efa55596cb33e17d6470afa0e0824c`:

1. Confirm RT-01 accurately records the owner's cancellation-only approval and supersedes only the conflicting §8 cancellation timing.
2. Verify §8/§9 and acceptance §14 agree on tick T preservation, T+1 enabled/disabled continuation, task clearing, endpoint normalization, silent events, failed-call rollback and exact save/restore.
3. Check that no general new timing rule was introduced for other task-id-clearing causes or residual cases.
4. Re-run the two `rt01_*` tests, the exact public-API residual probes from the prior audit, and the frozen-vector/required-toolchain checks as independently appropriate.
5. Confirm production Rust, dependencies, toolchain and existing golden values are unchanged, then independently decide whether F01 is closed and whether the retained milestone gate can receive a new verdict.

Commit/push are authorized only to `feat/c03-s4-integration`; the published commit is identified by Git and the final delivery response rather than a circular self-hash inside this file. No merge into main or independent milestone PASS is declared here.


## Review result

RT-01 PASS at 90b94d8866330141962cd915f6cc8eee402e757. The independent targeted reviewer found no concrete defect; the required Cargo fmt, check, Clippy, debug test, and release test commands were independently run at that exact commit and passed (82 tests plus 2 doctests in each test profile). The standalone canonical-vector check passed. See C0_3_RT01_INDEPENDENT_REVIEW.md for scope and limitations.
