# C0.3 Owner Decision Matrix

Version 1.0 · 17 November 2026  
Status: recommendations for the project owner. None of these rows is approved by this document.

The G3 audit verdict is **FAIL**. Each recommendation below is an input to a later specification revision. It does not change `docs/C0_3_IMPLEMENTATION_SPEC.md`, the frozen C0.2 archive, or any golden vector.

Approval column: **Required** means coding that area before a recorded owner decision would invent a contract. **Confirm deferral** means an approved document already names a later gate, and the owner still needs to say that C0.3 will not absorb it.

## D1 — Checkpoint format and revision

| | |
| --- | --- |
| Normative requirement | Core §5 and §11: a new future-affecting field needs a versioned schema. C0.2 spec: `SH:C02:STATE` revision 1 has no successor; a revision-1 reader rejects every other revision; M0-T11 owns the first migration. Baseline manifest: published revision 1 blobs stay valid. |
| Alternatives | A. New tag `SH:C03:STATE`, revision 1, new fixture id, SHA-256 trailer, C0.2 decoder untouched. B. `SH:C02:STATE` revision 2 with no migrator. C. Extend revision 1 with defaulted fields. |
| Recommendation | A. Reject C. Revision 1 stays closed. B is possible and still needs a full field list; it couples the production fixture to the C0.2 tag. |
| Justification | A new tag keeps the frozen decoder closed and leaves M0-T11 as the first real migrator. |
| Determinism | Field order, counts, and the digest domain string become part of the replay identity. |
| Compatibility | Revision 1 blobs restore exactly as they do now. A production blob presented to the C0.2 decoder fails with the existing unsupported-format class. There is no silent upgrade. |
| Acceptance | C03-T01g plus a named rejection matrix copied from the C0.2 precedence: truncated, wrong tag, wrong revision, checksum, corrupt count, unknown fixture, content mismatch. |
| Risks | The tag text and field order, once published, are as frozen as C0.2 revision 1. Approving A without the field list does not close G3-B01. |
| Owner approval | Required |

## D2 — Winner and loser diagnostics

| | |
| --- | --- |
| Normative requirement | C03-T01: one winner and one loser diagnostic, identical for forward and reversed submission. Core §4 and ADR-021: the C0.1 key is not production fairness. GDD: gameplay fairness needs its own rules before adoption. "Production fairness has its own ordered fixture." |
| Alternatives | A. Inside the C03 fixture only, the lower C0.1 key `(target tick, actor ID, producer ID, sequence)` wins. The loser gets a new production diagnostic, not a C0.1 `Outcome` variant. B. Lower actor ID wins and ignores producer and sequence. C. Submission order wins. D. A new explicit priority field. |
| Recommendation | A, limited to this fixture. Reject B and C. D waits for a later fairness ADR. |
| Justification | A is already a total order, already stable under reversal, and already the admission identity. B collides when one actor submits two claims. C contradicts the reversal test. |
| Determinism | The winner is a function of the keys and the stock, not of batch order. |
| Compatibility | Fixture 1 and fixture 2 command bytes stay unchanged. The new diagnostic is a new dialect. |
| Acceptance | C03-T01a in both orders, plus the duplicate-key case already rejected by C0.1 admission. State the diagnostic name and byte before the test is normative. |
| Risks | Adopting A in prose that calls it "the game's fairness rule" would exceed ADR-021. The decision record should call it the C03 fixture order. |
| Owner approval | Required |

## D3 — Cancellation command

| | |
| --- | --- |
| Normative requirement | Core §7 and C03-T01: cancellation at the completion tick invalidates before completion and releases once. Core invariant 5. No existing payload is a cancel. |
| Alternatives | A. Admitted `CancelReservation` using the existing command key, naming the reservation ID, applied in the command phase. B. Cancellation only as a restored task stage, with no command. C. Expiry only, with no cancel input. |
| Recommendation | A. |
| Justification | The tie is an input at a boundary. LOD requires replayable admitted controls. B cannot be supplied by the command log. C fails C03-T01c. |
| Determinism | The command sorts with other commands. Invalidation then observes the cancelled reservation before work. |
| Compatibility | New payload tag on the new fixture only. Fixture 2 keeps `SetMode` and `MintRecord`. |
| Acceptance | C03-T01c. Unknown reservation and a second cancel are no-ops or typed errors, using the one rule chosen in the repeat-release decision. |
| Risks | A cancel of a reservation that completes on a later tick must say whether the release happens at the cancel tick or at the old due tick. Recommendation: release in the invalidation phase of the cancel's target tick. |
| Owner approval | Required |

## D4 — Disablement command

| | |
| --- | --- |
| Normative requirement | C03-T01 and M0-T08: disablement at the completion tick transfers no stock and releases once. C0.2: enabled flags do not change after construction. C0.3 draft §6: a disabled claimant does not acquire. |
| Alternatives | A. Production-fixture command that sets the enabled bit, applied in the command phase before invalidation. Fixture 1 and fixture 2 ignore it. B. Construction-time flag only. C. A checkpoint field that flips at a tick with no command. |
| Recommendation | A. |
| Justification | B cannot both acquire and then lose eligibility on a later tick. C is harder to see in the command log. G3-B03. |
| Determinism | The bit changes only from an admitted command, in key order, before work. |
| Compatibility | C0.2 blobs keep immutable flags. The production schema records the bit and any still-pending disable command. |
| Acceptance | C03-T01d setup is: acquire while enabled, disable on the completion tick, no transfer, one release. A separate case rejects a new acquisition while disabled. |
| Risks | Reusing the C0.2 flag mutator on fixture 2 would change published restores. The command has to be fixture-scoped. |
| Owner approval | Required |

## D5 — Transaction identity

| | |
| --- | --- |
| Normative requirement | Core §7: a completion has a stable task or transaction identity and applies every validated leg once. C0.2 generated events have no command key. Record IDs are monotonic and are not reused. |
| Alternatives | A. One new monotonic transaction ID, allocated when the completion is admitted, with leg indexes inside it. Replay of that ID is a no-op. B. Use the command key as the identity. C. Hash the leg list and use the hash as the identity. |
| Recommendation | A. |
| Justification | B does not cover a generated completion. C changes if canonicalization of an equivalent leg list changes. The existing record cursor is the allocation rule already proven in C0.2. |
| Determinism | The ID is the next cursor, not a hash of map iteration. |
| Compatibility | Fixture 2's next record ID and its published blobs stay on revision 1. The production fixture has its own cursor in the new schema. |
| Acceptance | C03-T01e and C03-T01f. A failed transaction allocates nothing that a retry could observe as a spent ID, or the spec states that a failed attempt does consume an ID. Recommendation: failed admission does not consume the ID, matching C0.2 mint rejection. |
| Risks | Consuming IDs on failure is deterministic and also surprises replay comparisons. The choice has to be explicit. |
| Owner approval | Required |

## D6 — Recipes and constants

| | |
| --- | --- |
| Normative requirement | C03-T01: fail a later recipe leg and leave earlier legs unchanged. Core §7: recipes consume declared inputs and create declared outputs; a balance need not stay constant if the ledger traces the change. M0-T07 adds the four-location chain. No item IDs are approved. |
| Alternatives | A. Abstract two-leg recipe: debit stock definition 1 by 1, credit stock definition 2 by 1; the failing fixture rejects the credit. B. Full M0 starter catalog: four occupations, 3–5 jobs, six resources, 8–12 items. C. No recipe until M0 content exists. |
| Recommendation | A for the first C0.3 fixture. B stays M0 §1. C cannot satisfy C03-T01. |
| Justification | A is the smallest declared transformation that can fail on the second leg. It does not pretend to be the content catalog. |
| Determinism | Definitions are constants in the fixture content hash. |
| Compatibility | New content hash. C0.1 and C0.2 content hashes remain `042797b9…` and `043d3507…`. |
| Acceptance | One successful trace with signed deltas, then C03-T01f. The four-location chain is D11, not this row. |
| Risks | Approving real item names here will collide with the later authored M0 catalog. Keep the IDs abstract. |
| Owner approval | Required |

## D7 — Actor needs model

| | |
| --- | --- |
| Normative requirement | Core preamble and README name production needs. Core §6 lists goals and DecisionTrace fields and gives no channel, threshold, weight, or wake tick. M0-T13 owns the causal need scenario. |
| Alternatives | A. Defer all scoring to M0. C0.3 ships no need update. B. Owner supplies channels, formulas, and wake ticks before C0.3 coding. C. Implement a placeholder curve. |
| Recommendation | A, if the owner narrows the deliverable cell. B is the other lawful choice. Reject C. |
| Justification | A placeholder would become the replay oracle. Nothing in the approved texts selects its numbers. |
| Determinism | Any later formula has to be integer arithmetic on tick and persisted need values, evaluated on scheduled ticks. |
| Compatibility | Leaving needs out of the first production schema avoids a migration when the real formula arrives, provided D1's schema does not reserve dummy fields. |
| Acceptance | C03-N01 stays blocked under A. Under B it becomes a named trace with approved scores. |
| Risks | A leaves the README deliverable cell unmet until the owner edits that cell or accepts a documented narrowing. |
| Owner approval | Required |

## D8 — Job timing and lifecycle

| | |
| --- | --- |
| Normative requirement | Core §6 stage names, interruption, and save/load of current task and progress. No durations. C02-T02 already has pending and completed for synthetic tasks. |
| Alternatives | A. Ledger fixture uses an explicit reduced map: `Reserved`, then `Completing` or a failure branch, zero travel ticks, and the Core names that are unused are documented as unused. B. Block the ledger until every stage has a duration. C. Invent durations. |
| Recommendation | A. |
| Justification | C03-T01 needs a reservation and a completion. It does not need travel time. B stalls the gate on data Core does not contain. C is an unapproved mechanic. |
| Determinism | Unused stages are absent, so they cannot tie. |
| Compatibility | Fixture 2 pending/completed encoding stays. The reduced map lives in the new schema. |
| Acceptance | C03-T01b through C03-T01d name the stage before and after the tie. C03-J01 stays blocked for catalog timing. |
| Risks | A reduced map can be mistaken for a waiver of Core §6. The decision record should say the remaining stages arrive with the job catalog. |
| Owner approval | Required |

## D9 — Navigation model

| | |
| --- | --- |
| Normative requirement | Technical Architecture §§2 and 10 assign initial navigation and a navigation fixture to C0.3. Core §8 assigns the two-site graph to M0 and requires successor and equal-cost tests when the first navigation fixture exists. No graph is approved. M0-T08 requires unreachable targets to fail. |
| Alternatives | A. Defer route search, movement timing, and unreachable-target failure until the owner supplies the M0 graph. No route crate. B. Owner supplies a minimal graph, costs, successor order, and the unreachable diagnostic as part of C0.3. C. Add a pathfinding crate with a default order. |
| Recommendation | A, together with the D11 narrowing. Reject C. |
| Justification | Core §8 already withholds the algorithm until a fixture exists, and no fixture data exists. TA §10 cannot be satisfied by an invented graph. |
| Determinism | When a graph arrives, successor order and equal-cost ties have to be total orders in the test, not library defaults. |
| Compatibility | No new dependency. `Cargo.lock` stays at the baseline hash until a crate is separately approved. |
| Acceptance | C03-P01 stays blocked. The unreachable sentence is removed from the C0.3 promotion set or implemented under B. |
| Risks | A conflicts with a literal reading of Technical Architecture §10 until the owner records the deferral. |
| Owner approval | Required |

## D10 — Automatic scheduling policy

| | |
| --- | --- |
| Normative requirement | LOD §2 and §5: production activation is chosen only with a deterministic fixture; C0.2 uses explicit `SetMode`; approximation waits for M0-T05 and an ADR-009 amendment. Visibility never selects mode. |
| Alternatives | A. C0.3 does not select modes. Actors change mode only through admitted `SetMode`, as in C0.2. B. Add a distance or workload policy now. |
| Recommendation | A. |
| Justification | The owning documents already defer the policy. B would be a new mechanic and would need an ADR-009 amendment. |
| Determinism | Mode remains an admitted input. Active and scheduled production work, once D2 exists, should use the same reservation outcome. That test is G3-M04 and can be added without a new policy. |
| Compatibility | C0.2 mode phase and its witness hashes stay. |
| Acceptance | C03-R01. Add the mode-neutrality case when the production fixture exists. |
| Risks | Calling jobs "C0.3" can be misread as permission to wake actors automatically. This row closes that reading. |
| Owner approval | Confirm deferral |

## D11 — Who owns the 20-actor scenario

| | |
| --- | --- |
| Normative requirement | README: exit on C03-T01 and the applicable M0-T06 through M0-T08 fixtures. C03-T01: run the corresponding cases before promoting C0.3. M0-T06 is 20 NPCs plus the C03 ordering case. M0 §1 authors that world later and pins `ticks_per_game_day`. |
| Alternatives | A. C0.3 promotion is C03-T01, the one-unit ordering case, the abstract recipe rollback, and the expiry/cancel/disable ties. The 20-NPC scenario, four-location chain, and unreachable targets stay M0 unless a later authorization adds them. B. C0.3 includes full M0-T06, M0-T07, and M0-T08. C. C0.3 is only C03-T01 and ignores the "corresponding cases" sentence. |
| Recommendation | A. |
| Justification | B requires content, day length, a graph, and four owner locations that no document defines. C drops a sentence the exit criterion includes. A keeps every ledger result that is already written and names the rest as M0. |
| Determinism | The promotion set has a finite command log. The 20-NPC run does not, until M0 authors it. |
| Compatibility | A does not require `ticks_per_game_day` in the first production schema. |
| Acceptance | C03-T01a through C03-T01i after D1–D6. M0-T06's 20-NPC procedure, M0-T07's four-location trace, and M0-T08's unreachable sentence are listed as not required for this promotion. |
| Risks | A is a narrowing of "applicable" and "corresponding". It is valid only when the owner records it. The README cell should be updated in that same decision so the next audit does not reopen G3-B02 and G3-H06. |
| Owner approval | Required |

## Repeat-release event rule

This rule is part of D2 and D5 and is split out because the current draft contradicts itself (G3-H04).

| | |
| --- | --- |
| Recommendation | A repeated terminal release and a replayed successful transaction ID change no state and emit no event. The event cursor does not advance. |
| Why | Core §7 calls the repeated release a no-op. A rejection event is a second observable and needs its own approved diagnostic. |
| Owner approval | Required |

## Simultaneous causes

| | |
| --- | --- |
| Recommendation | On one tick, apply at most one release. Record the first matching reason in this order: disablement, cancellation, expiry. Completion does not run. |
| Why | M0-T08 requires one deterministic reason. The order matches the Core §7 phrase "actor disable/cancel and expiry" and keeps expiry last. |
| Owner approval | Required |

## Invalidation phase

| | |
| --- | --- |
| Recommendation | On the production fixture, after the tick advances: mode, command, invalidation, work. Fixtures 1 and 2 do not enter the invalidation phase. |
| Why | Core §7 requires invalidation before completion on a tie. Isolating the phase protects published event order. |
| Owner approval | Required |

## What remains blocked after these recommendations

Recording the recommendations is not the same as editing the specification. Until a revised spec contains the approved schema, diagnostics, commands, abstract recipe, reduced stage map, and the narrowed exit list, G3-B01 and G3-B02 stay open and the verdict stays **FAIL**.

Implementation still requires a separate project-owner authorization after that revision.
