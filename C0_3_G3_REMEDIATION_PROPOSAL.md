# C0.3 G3 Remediation Proposal

Version 1.2-proposal · 17 November 2026  
Status: proposal for the project owner. Recommendations are **NOT APPROVED**. Implementation is not authorized. No normative specification, source file, or golden vector was edited.

Companion files: `C0_3_SCOPE_TRACEABILITY.md`, `C0_3_OWNER_DECISIONS_PENDING.md`.

## 1. Blocking findings

| ID | Severity | What remains unresolved until the owner records a decision |
| --- | --- | --- |
| G3-B01 | BLOCKER | Schema and keys now have a complete proposed encoding. It is not accepted |
| G3-B02 | BLOCKER | Needs, jobs, navigation, and M0-T06 through M0-T08 stay in the exit. Their procedures are proposed, not accepted |
| G3-B03 | BLOCKER | The disable-after-acquire sequence is specified. The `SetEnabled` command is not accepted |
| G3-H01 | HIGH | Fixture 2 preservation is a confirmation, not an archive edit |
| G3-H02 | HIGH | The transfer order is Actor → Store → Cargo → Site. The invalid later leg rolls back only its own boundary |
| G3-H03 | HIGH | Unreachable failure has a position, a search, and a release rule |
| G3-H04 | HIGH | Repeat release is a silent no-op in every proposed test |
| G3-H05 | HIGH | One reason, ordered disablement, cancellation, expiry |
| G3-H06 | HIGH | The 20-actor script uses one current task and one bundle per actor. The one-unit script remains |
| G3-M01 | MEDIUM | The work transition table gives each Core stage a tick |
| G3-M02 | MEDIUM | Zero quantity, an empty interval, overflow, and partial fill are preflight failures |
| G3-M03 | MEDIUM | The C0.2 rejection classes are named for the new tag |
| G3-M04 | MEDIUM | Mode does not change reservation order |
| G3-M05 | MEDIUM | Fixture 3 has five phases. Fixtures 1 and 2 do not enter the last three |
| G3-L01 | LOW | A disabled acquire is a rejection with no id |
| G3-L02 | LOW | Cursor overflow is `CapacityExceeded`. A corrupt count is `Corrupt` |
| G3-I01 | INFORMATIONAL | No golden hex is published ahead of an independent oracle |
| G3-I02 | INFORMATIONAL | R2 and timing budgets stay outside this gate |
| G3-I03 | INFORMATIONAL | G3 did not re-run the C0.2 suite |

The exit is unchanged. Internal stages still do not close the gate.

## 2. Inconsistencies closed in this refinement

| Earlier gap | Revised rule | Decision |
| --- | --- | --- |
| Actor position was absent | Locus, site, cell, edge, and traversed cost are checkpoint fields | OD-01 |
| One reservation id per task contradicted Core §6 | A task stores an ascending list of reservation ids | OD-01, OD-04 |
| Work asked for a tool and a workstation without an atomic rule | `ReserveBundle` accepts both or neither | OD-04, OD-13 |
| Beds, tools, food, and workstations were the same kind of stock | Class 1 is consumed on completion. Class 2 is released and the quantity stays | OD-04 |
| Edge cost could be either a search weight or a duration | It is both. One cost unit is one work-phase tick | OD-11 |
| Needs had no phase, no actor order, and no all-ineligible result | Decision phase, ascending actor id, winner 0 when none are eligible | OD-09, OD-15 |
| A restored actor could be scored again | `next_wake_tick` and the stored trace suppress evaluation until the wake | OD-09 |
| A newly chosen task could move on the decision tick | Work skips `created_tick` equal to the staged tick | OD-10 |
| Cancel and disable had no speaker | Producer 1 only. Cancel also requires the claimant actor id. I0 is not consulted | OD-06, OD-07 |
| Repeat release disagreed with itself | Silent no-op, no event, no cursor advance | OD-08 |
| M0-T07 could be read as rolling back earlier hops | Earlier committed hops stay. Legs inside the failing transaction do not | OD-05 |
| M0-T07 started at a site stockpile | The first owner is actor inventory. Then container 20, cargo 30, and site stockpile 2 | OD-05, OD-10 |
| Stage events were optional or alternate | Every stage change emits kind 7, in the one order in OD-02 | OD-02 |
| An actor could hold four current tasks | `actor.task_id` is the only non-terminal task. The 20-NPC bundle hangs on that task | OD-01, OD-13 |
| `CompleteTask` could be read as applying legs immediately | It only sets `completion_requested`. Invalidation still runs before work | OD-10, OD-15 |

## 3. Recommended decisions

All of these are **NOT APPROVED**. Byte layouts and the worked bundle are in `C0_3_OWNER_DECISIONS_PENDING.md`.

| ID | Recommendation |
| --- | --- |
| OD-01 | Fixture 3, one current task per actor, `completion_requested`, planned legs, and `BeginTask` |
| OD-02 | `SH:C03:EVENT:v1`, kinds 1 through 7, one event order, cursor and digest advanced once per event |
| OD-03 | Command-key order inside fixture 3, all-or-nothing requests |
| OD-04 | Many reservations per task, atomic bundles, consumable versus durable |
| OD-05 | Transaction id only after every leg validates |
| OD-06 | Producer 1, and only the claimant actor id, may cancel |
| OD-07 | Producer 1 may disable or enable the subject actor |
| OD-08 | Silent no-op for a repeated release and a replayed completion |
| OD-09 | Decision phase, checked `i64` scores, neutral relationship gate, no GOAP |
| OD-10 | One work transition per later tick, including `Completing` before legs |
| OD-11 | Graph and grid are separate searches. Cost selects the path and times the travel |
| OD-12 | No automatic mode policy |
| OD-13 | Twenty actors, one bundle each, four stocks of quantity 5, plus the separate one-unit case |
| OD-14 | One release reason: disablement, then cancellation, then expiry |
| OD-15 | Mode, command, invalidation, decision, work |
| OD-16 | Fixture 2 synthetic completion stays frozen |

## 4. Acceptance sequences

Every fresh fixture starts `next_event_id` at 1 and the digest at `H0 = SHA-256("SH:C03:EVENT-INIT:v1")`. Each event below is the only legal event at that cursor. The digest after event N is `SHA-256("SH:C03:EVENT:v1" || previous || canonical_bytes)` using OD-02. No hex is published here. An independent oracle packs it after the owner accepts the layout. A second order is a failure. Replay repeats the same commands and the same bytes. A failed preflight or a failed later leg leaves the previous bytes, cursor, and digest unchanged.

Shared phases after the tick increment are mode, command, invalidation, decision, work. Unless a script sets a wake, the decision phase emits nothing. `CompleteTask` only sets `completion_requested` in the command phase.

### 4.1 C03-T01a — two claimants

Initial: stock 1, class 1, quantity 1, owner kind 3, owner id 1. No reservation. Actor 1 `task_id` 11. Actor 2 `task_id` 12. Both tasks are `Queued`. Cursor 1.

Commands, either batch order: `ReserveStock` actor 1 producer 1 sequence 1, and actor 2 producer 1 sequence 1. Target tick 1. Quantity 1. Expiry 5. Each `task_id` equals that actor's current task.

Command phase sorts actor 1 first.

| Cursor | Kind | Payload |
| --- | --- | --- |
| 1 | 1 | reservation 1, stock 1, quantity 1, actor 1 |
| 2 | 7 | task 11, stage `Reserved` |
| 3 | 2 | stock 1, quantity 1, reason 4, actor 2 |

End cursor 4. On-hand 1. Available 0. Task 12 stays `Queued`. Reversing the batch produces the same three events and the same bytes.

### 4.2 C03-T01b — expiry

Initial cursor 1. Reservation 1 acquired at tick 1, expiry 3, task 11 `Completing`, `due_tick` 3, actor enabled, `completion_requested` 0. No cancel and no disable.

Tick 3 invalidation runs before work.

| Cursor | Kind | Payload |
| --- | --- | --- |
| 1 | 3 | reservation 1, reason 3 |
| 2 | 7 | task 11, stage `Failed` |
| 3 | 5 | task 11, reason 3 |

End cursor 4. No kind 4. On-hand unchanged. A later release emits nothing and leaves cursor 4 and the digest unchanged.

### 4.3 C03-T01c — cancel

Same initial task, expiry 9. Command: `CancelReservation` producer 1, actor id equal to the claimant, reservation 1, target tick 3.

| Cursor | Kind | Payload |
| --- | --- | --- |
| 1 | 3 | reservation 1, reason 2 |
| 2 | 7 | task 11, stage `Cancelled` |
| 3 | 5 | task 11, reason 2 |

A command whose actor id is not the claimant fails preflight. Cursor and digest stay at the pre-call values.

### 4.4 C03-T01d — disable after acquire

Actor 1 starts enabled. Task 11 is the only current task. Stock 1 as in §4.1. Expiry of the new reservation is 4.

| Tick | Events in order | End cursor | End state |
| --- | --- | --- | --- |
| 1 | Kind 1 reservation 1 stock 1 quantity 1. Kind 7 task 11 `Reserved` | 3 | Acquired tick 1. Actor enabled |
| 2 | None. `SetEnabled(actor 1, false)` is only admitted for target 3 | 3 | Actor still enabled |
| 3 | Kind 3 reservation 1 reason 1. Kind 7 task 11 `Failed`. Kind 5 task 11 reason 1 | 6 | On-hand 1. `completion_requested` clear. No kind 4 |

`CompleteTask` may share tick 3. Its command phase sets the flag. Invalidation still emits the three events above, and work emits nothing. Fixture 2 is a separate frozen run. Its disabled synthetic tasks still complete, and its published hashes stay the baseline values.

### 4.5 C03-T01e — replay

Initial: one applied transaction, kind 4 already in the prefix, reservation released reason 8, cursor C, digest D.

The same completion key, then another release: no event. Cursor stays C. Digest stays D. Owners and the applied bit stay. The tick advances.

### 4.6 C03-T01f — invalid later leg

Initial: stock 13 quantity 1, owner kind 3, owner id 2. Planned legs: transform out stock 13 quantity 1, then transform in definition 2 with `owner_id` 0.

The boundary rolls back. Event count 0. Cursor and digest stay. Tick stays. No transaction id is consumed.

### 4.7 C03-T01g and C03-T01i

Save while reservation 1 is active and completion is in the future. The saved cursor is the next event id. Restore emits none of the prefix. The next event uses that cursor. Grouped `drive` matches one boundary at a time, including the event order above.

### 4.8 C03-T01h, overflow, and restore errors

Over-reserve, quantity 0, an empty interval, and a `u64` reservation sum that would overflow fail before publication. A scoring product or sum that overflows `i64`, and a route-cost sum that overflows `u64`, return `CapacityExceeded` and emit nothing. Cursor overflow returns `CapacityExceeded`. A count, grid dimension above 4096, or a grid product that does not fit the unread body returns `Corrupt`. Truncation, the wrong tag, the wrong revision, a bad checksum, an unknown fixture, and a content mismatch stay distinct. A C0.2 blob is `UnsupportedFormat`.

### 4.9 M0-T06

Initial cursor 1. Actors 1 through 20, each with exactly one `Queued` task, ids 1 through 20. Stocks 101, 102, 103, and 104, quantity 5, classes 2, 2, 1, and 2. Owner kind 3, owner id 1.

One `ReserveBundle` per actor, producer 1, sequence 1, target tick 1, the four stocks in id order, quantity 1, expiry 10.

Actors 1 through 5, in actor order, each emit kind 1 for stocks 101, 102, 103, and 104, then kind 7 `Reserved`. Those are cursors 1 through 25. Actors 6 through 20 each emit one kind 2 for stock 101, quantity 1, reason 4. Those are cursors 26 through 40. End cursor 41. No kind 7 for a loser. On-hand stays 5. Available becomes 0. Food is not consumed. §4.1 is the included one-unit case and is not this batch.

A second bundle from actor 1 for stock 103 emits one kind 2 reason 4. Scheduled mode on actor 6 does not change cursors 1 through 40.

### 4.10 M0-T07

Initial cursor 1. `next_record_id` 401. `next_stock_id` 11. `next_transaction_id` 1. Actor 1 enabled, `task_id` 0, at site 1. Stock 10, definition 1, class 1, quantity 1, owner kind 1, owner id 1. Container 20, cargo 30, and site stockpile 2 are empty.

Each hop is `BeginTask` sequence 1 and `CompleteTask` task id 0 sequence 2, producer 1, one current task, then that task ends before the next hop.

| Tick | Transfer | Events | Balances after the tick |
| --- | --- | --- | --- |
| 1 | Actor inventory → store. Debit stock 10. Credit stock 11, owner kind 2, owner id 20 | Cursor 1 kind 7 task 401 `Completing`. Cursor 2 kind 4 transaction 1. Cursor 3 kind 7 task 401 `Done` | Stock 10 quantity 0. Stock 11 quantity 1 at container 20. `task_id` 0 |
| 2 | Store → cargo. Debit stock 11. Credit stock 12, owner kind 4, owner id 30 | Cursors 4, 5, 6: kind 7 task 402 `Completing`, kind 4 transaction 2, kind 7 `Done` | Stock 11 quantity 0. Stock 12 quantity 1 at cargo 30 |
| 3 | Cargo → site. Debit stock 12. Credit stock 13, owner kind 3, owner id 2 | Cursors 7, 8, 9: kind 7 task 403 `Completing`, kind 4 transaction 3, kind 7 `Done` | Stock 12 quantity 0. Stock 13 quantity 1 at site 2 |
| 4 | Recipe. Transform out stock 13. Transform in stock 14, definition 2, quantity 1, owner kind 3, owner id 2 | Cursors 10, 11, 12: kind 7 task 404 `Completing`, kind 4 transaction 4, kind 7 `Done` | Stock 13 quantity 0. Stock 14 quantity 1 at site 2 |

Tick 5 plans a recipe whose first leg debits stock 14 and whose second leg credits `owner_id` 0. The boundary rolls back. Cursor stays 13. Digest stays the digest after event 12. Tick stays 4. Transactions 1 through 4 remain. No stock 15 appears.

Final: definition 1 quantity 0. Definition 2 quantity 1 at owner kind 3, owner id 2. Actor inventory, container 20, and cargo 30 hold quantity 0 of both definitions.

### 4.11 M0-T08

Cancel, disable, and expiry are §4.3, §4.4, and §4.2. When all three are true, the only reason byte is 1. The event order is still kind 3, kind 7 `Failed`, kind 5.

Unreachable, one held reservation, target site 9, empty path, cursor 1:

| Cursor | Kind | Payload |
| --- | --- | --- |
| 1 | 3 | that reservation, reason 5 |
| 2 | 7 | the task, stage `Failed` |
| 3 | 5 | the task, reason 5 |

The stock owner does not change. Repeat release and replay are §4.5.

Rest at home with no reservation, already `Completing`, `completion_requested` 1, actor enabled: cursor 1 kind 4 with an empty leg list, cursor 2 kind 7 `Done`. No kind 3 and no kind 5. The same task with `SetEnabled` false on that tick: cursor 1 kind 7 `Failed`, cursor 2 kind 5 reason 1. No kind 4. Return Home uses those same two sequences.

### 4.12 Core §§6–8

Needs, cursor 1, actor 1, hunger 1000, other channels 0, relationship term 0, neutral minimums, wake equal to the staged tick, `task_id` 0, one food unit in actor inventory, actor already at that site.

| Cursor | Kind | Payload |
| --- | --- | --- |
| 1 | 1 | the new food reservation |
| 2 | 7 | the new task, stage `Executing` |
| 3 | 6 | winner eat, that task id, next wake = tick + 4, five scores |

Work emits nothing on that tick. The next tick emits one kind 7 `Completing`. The tick after that, if invalidation left the reservation active, emits kind 4, kind 3 reason 8, and kind 7 `Done`, in that order. Restore before the stored wake emits no second kind 6.

No eligible goal: one kind 6, winner 0, task 0. Cursor advances by 1. No kind 1 and no kind 7.

A save on a cost-2 edge restores `path_index` and `traversed_cost` and does not search again. The remaining tick emits one kind 7 only if the stage changes. Site 1 to site 2 is one movement tick and one kind 7 into `Executing` when the edge completes. Two equal-cost edges select the lower destination id, then the lower edge id. Site 9 uses the unreachable order in §4.11.

### 4.13 Provenance

A command with producer 0 fails admission. Cursor and digest stay. A generated completion stores producer 0 and sequence 0 on the transaction and can be created only by work after invalidation. That stored key replays as §4.5.

## 5. Consistency

OD-01 through OD-16 now use one current task, one event order, and completion only after invalidation. Core §§4–8 are met by the command-key order, the one-owner ledger, the five goals, the scheduled trace, and the separate graph and grid searches. C03-T01 and the written M0-T06, M0-T07, and M0-T08 procedures remain, including Actor → Store → Cargo → Site. No exit row was removed. C0.1 and C0.2 encodings and goldens are untouched. The recommendations are not approved, so the G3 verdict remains FAIL until the owner records the rows and a specification revision is reviewed.

## 6. Remediation sequence

R1 carries OD-01 through OD-05, OD-08, OD-14, and OD-15 into a future specification revision. Tests are §4.1, §4.2, §4.5, §4.6, §4.7, and §4.8. R2 carries OD-06, OD-07, and OD-16. Tests are §4.3, §4.4, and the frozen C0.2 suite. R3 carries OD-09 through OD-13. Tests are §4.9 through §4.13. R4 is a new architecture review. This proposal does not clear the FAIL. Coding needs a separate authorization after that review.

## 7. Files and non-modification

Updated for refinement 1.2: this file, `C0_3_OWNER_DECISIONS_PENDING.md`, and `C0_3_SCOPE_TRACEABILITY.md`.

Not modified: `docs/C0_3_IMPLEMENTATION_SPEC.md`, `docs/C0_3_ACCEPTANCE_TESTS.md`, `docs/C0_3_ARCHITECTURE_REVIEW.md`, the C0.1 and C0.2 specifications, `src/`, `Cargo.toml`, `Cargo.lock`, and every golden vector.
