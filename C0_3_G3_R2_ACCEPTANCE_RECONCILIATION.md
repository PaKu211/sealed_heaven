# C0.3 G3 R2.1 Acceptance Reconciliation

Version R2.1-final · 17 November 2026  
Status: proposed expected results. Not normative. Gameplay hashes are withheld.

`docs/C0_3_ACCEPTANCE_TESTS.md` stays as published. These rows become executable only after the owner approves the R2.1-final amendments and a later edit copies them into that file. H0 remains `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c`.

Shared constructor, except where a row replaces a field: seed 100, wake 4, the approved weights and relationship minimums, recipe 1, holders 20 and 30, sites 1, 2, and 9, edges 1 and 2 only. Actors used by a test are enabled, mode active, channels 0 except where a row sets hunger, relationship term 0, `next_wake_tick` 100, locus site 1, empty path. M0-T07 uses wake 100 and the cursors in the remediation proposal §1. OC-EDGE is absent.

## Retained exits

| Exit | Proposed result | Blocked on |
| --- | --- | --- |
| C03-T01a | Tasks 1 and 2. `next_record_id` 3. Reservation 3. Events: kind 1, kind 7 `Reserved`, kind 2, kind 7 `Executing`. `next_event_id` 5 | AM-04, AM-07 |
| C03-T01b | Task already `Completing` from an earlier tick. Disablement emits kind 3, kind 7 `Failed`, kind 5. No kind 4 | AM-04 |
| C03-T01c | Cancellation emits kind 3, kind 7 `Cancelled`, kind 5. A cancel whose actor is not the stored claimant is `InvalidPayload` before any no-op. A later cancel of that same terminal reservation by the claimant is a no-op | AM-05 |
| C03-T01d | Tick 1 reserves and work enters `Executing`. Tick 2 admits `SetEnabled` for tick 3 and does not apply it. Tick 3 emits kind 3, kind 7 `Failed`, kind 5. No kind 4 | AM-05, AM-07 |
| C03-T01e | Replay of role 1 and role 2 with the same submitted task id emits nothing. The same key with a different submitted id is `ConflictingIdentity` and does not advance the tick | AM-02, AM-05 |
| C03-T01f | The two-task failure oracle in the proposal §3, and M0-T07 tick 10 as a later-leg failure against stock 14. Prior committed hops remain | AM-01, AM-03 |
| C03-T01g | Save while the unconsumed list is non-empty and the stage is not `Done`. Resume consumes the lowest key and retains every key, including submitted ids, in the registry | AM-02 |
| C03-T01h | `CapacityExceeded` for cursor, score, and route-sum overflow. `Corrupt` for a registry or table count that does not fit. `InvalidState` for a dangling task id, a task id equal to a reservation id, or `resolved_task_id` 0 | AM-04, AM-05 |
| C03-T01i | The same command list, one boundary at a time and grouped, including work events | AM-07 |
| M0-T06 | Tasks 1 through 20. `next_record_id` 21. Reservations 21 through 40 for actors 1 through 5. Then one `Executing` event per winner. Losers stay `Queued`. Supplies stay 5 | AM-04, AM-07 |
| M0-T07 | The ten-tick table in the proposal §1. Tick 9 leaves task 405 `Completing` with legs debit 14 then credit owner 0. Tick 10 is later-leg `InvalidLeg` and restores tick 9, including stock 14 quantity 1 | AM-01, AM-03, AM-07 |
| M0-T08 | Disablement wins when all three causes exist. Site 9 is reason 5. Rest with no reservation uses kind 7 and kind 5 on disable, or empty kind 4 and kind 7 `Done` on success | AM-06, AM-07 |
| Five goals | One row per goal in the proposal §5, including a save on the eat decision, a rest or return-home path, the work bundle, and one haul transaction | AM-06 |
| Navigation | Grid path north, north, east, east. Edge 1 arrives on the later work tick. Site 9 fails. Residual travel after cancel arrives with no event. No snap | AM-08 |
| C0.1 and C0.2 | Unchanged commands and frozen hashes | None |

## Provenance sequence

Start from a create-tick save whose registry holds sequence 2, submitted 0, resolved 401, role 1.

1. Replay `CompleteTask(0)` under that key. No-op. Tick advances. No second transaction.
2. Restore the same save. Replay `CompleteTask(401)` under that key. `ConflictingIdentity`. Tick unchanged.
3. From a non-zero admission whose registry holds submitted 11 and resolved 11, replay submitted 11. No-op. Replay submitted 0 under that key. `ConflictingIdentity`.

Repeat with a checkpoint whose registry contains sequence 7 as role 1 and sequence 8 as role 2, both submitted 11. Replay of either key with submitted 11 is a no-op. The blob still contains both records, so it does not match a blob that contains only sequence 7.

## Conservation and later-leg sequence

Use the two-task oracle in the proposal §3. The success drive and the quantity-2 failure are both required.

M0-T07 tick 9 is `drive(8, [BeginTask sequence 9 with debit 14 then credit owner 0, CompleteTask sequence 10 submitted 0])`. The resulting bytes show task 405, stage `Completing`, `created_tick` 9, `due_tick` 10, stock 14 quantity 1, and sequence 10 unconsumed with submitted 0 and resolved 405. `drive(9, [])` returns `InvalidLeg`. The bytes equal the tick-9 save. `next_event_id` is 14. Stock 14 remains quantity 1. Stocks created on ticks 2 through 8 remain.

## Residual edge sequence

No extra edge is installed. Actor 1 is on edge 1 with `traversed_cost` 0 and travelling task 50 with an empty path.

1. Cancel task 50. Events: kind 7 `Cancelled`, kind 5. `task_id` 0. Locus still 3.
2. Save and restore. Locus 3 and `traversed_cost` 0 remain.
3. Empty `drive`. Actor arrives at site 2. No event.
4. Repeat from the post-cancel save, disable the actor, then empty `drive`. Arrival still occurs. No task is created.

## Queued sequence

M0-T06 begins with twenty `Queued` tasks whose `due_tick` is 0. `drive` to staged tick 1 does not return `InvalidState`.

## Split and replay

C03-T01g saves the first checkpoint whose unconsumed list is non-empty and whose stage is not `Done`. Prefix events are not emitted again. Grouped `drive` is compared after every boundary.

## Hashes

H0 is safe to keep. The audit's event-header samples are wire examples. They are not retargeted onto these ids. The audit's empty-world probe is not an acceptance fixture. No gameplay checkpoint hash is promoted by this reconciliation.
