# C0.3 Implementation Specification

Version 1.3-approved · 17 November 2026  
Status: owner-authorized documentation after NF-05 and the targeted S1 H10/H09 clarifications. Implementation authorization is limited to those targeted corrections; this is not an S1 acceptance verdict.

S1 H10 clarification: the project owner's targeted-remediation authorization
permits the goal-0 clarification in §8 below. Other slice authorization and the
frozen compatibility boundaries are unchanged. This is not an S1 acceptance verdict.

S1 H09 clarification: the project owner's H09 Option B amendment narrows §5
row 7 to durable identities. Applied non-completion keys have no persistent
history. Checkpoint revision 1, the 41-byte registry, and published vectors are
unchanged.

This specification incorporates OD-01 through OD-16 as approved in `C0_3_OWNER_DECISION_RECORD.md` from `C0_3_OWNER_DECISIONS_PENDING.md` version 1.2, as amended by AM-01 through AM-08 from the R2.1-final remediation packet, the NF-01 through NF-04 documentation remediation, and the NF-05 preinstalled-task packing-field remediation. OC-EDGE is rejected. Fixture numbers in §12 are C0.3 fixture configuration. They are not the final gameplay balance of Sealed Heaven.

## 1. Authority and exit

The exit is C03-T01 and the applicable M0-T06, M0-T07, and M0-T08 procedures, as written in the README and M0 §6. The deliverable is needs, jobs, inventory, resource transactions, and the first navigation fixture. Socialize, react-to-threat, automatic mode policy, travel approximation, cohorts, day length, the 30-day run, save migration, JSON content, Bevy, and R2 stay on the gates that already own them.

`sealed_sim` is the headless kernel. No Bevy crate, no route-search crate, and no new direct dependency is part of this gate. Camera visibility does not select a mode, a job, or a path.

C0.1 encodings, C0.2 encodings, and their golden vectors stay frozen. Fixture 2 still completes a due synthetic task when its actor is disabled. Production invalidation, decision, and production work apply only to fixture 3. Fixture 2 keeps its own synthetic work phase.

The Balanced Development Strategy is approved as internal slices S1 through S4. An internal slice is not the C0.3 milestone exit. S4 plus frozen C0.1 and C0.2 regression is the first point that can meet that exit. Slice authorization is separate from this documentation approval.

## 2. Encoding rules

Integers are unsigned little-endian except `i64` scores, weights, role bonuses, relationship terms, and relationship minimums. Tags are exact ASCII with no length prefix and no terminator. Lists begin with a `u64` count. Records have no padding. Tables use the sort key named below. A zero id means none where this specification says so. The save blob is `body || SHA-256(body)`.

Counts and grid dimensions are checked before allocation. If `count * minimum_record_size` overflows, or the product exceeds the unread body, the blob is `Corrupt`. A count of `u64::MAX` is `Corrupt`. A grid width or height above 4096, a single zero dimension, a `u32` product overflow, or a product larger than the unread body is `Corrupt`.

`SH:C01:STATE` and `SH:C02:STATE` presented to the fixture-3 decoder are `UnsupportedFormat`. `SH:C03:STATE` presented to the C0.2 decoder is `UnsupportedFormat`. There is no migrator. Revision other than 1 on the C0.3 tag is `UnsupportedRevision` before the checksum is consulted.

## 3. Checkpoint

Tag `SH:C03:STATE`, `u16` revision 1, fixture id 3.

| Order | Field |
| --- | --- |
| 1 | ASCII `SH:C03:STATE`, `u16 = 1` |
| 2 | `u32` fixture id 3, 32-byte content hash |
| 3 | `u64` tick, `u64` seed |
| 4 | `u64 next_actor_id`, `u64 next_record_id`, `u64 next_transaction_id`, `u64 next_stock_id` |
| 5 | Scenario block. Its SHA-256 is field 2 |
| 6 | Actors, count then ascending `u32` id |
| 7 | Sites, count then ascending `u64` id |
| 8 | Holders, count then ascending `u64` id. Each holder is `u64 id`, `u8 kind` where kind is 2, 4, or 5 |
| 9 | Edges, count then ascending `(from_site, to_site, edge_id)` |
| 10 | One grid descriptor |
| 11 | Stocks, count then ascending id |
| 12 | Reservations, count then ascending id |
| 13 | Tasks, count then ascending id |
| 14 | Applied transactions only, count then ascending id |
| 15 | Consumed completion registry, count then ascending command key |
| 16 | Pending commands in command-key order |
| 17 | Decision traces, count then ascending actor id, at most one per actor |
| 18 | `u64 next_event_id`, 32-byte event digest |

Scenario block: `u64 wake_interval`; five goals in ordinal order, each with five `i64` weights; five `i64` relationship minimums; `u64 recipe_count`; each recipe is `u64 recipe_id`, `u64 input_definition`, `u64 input_quantity`, `u64 output_definition`, `u64 output_quantity`, `u8 output_owner_kind`, `u64 output_owner_id`. Recipes are stored in ascending `recipe_id`.

Actor: `u32 id`, `u8 enabled`, `u8 mode`, `u64 next_wake_tick`, five `u32` channels in `0..=1000` (hunger, fatigue, duty, haul, homesickness), five `i64` role bonuses, one `i64 relationship_term`, `u64 home_site_id`, `u64 task_id`, then position. Position is `u8 locus`, `u64 site_id`, `u32 grid_x`, `u32 grid_y`, `u64 edge_id`, `u64 traversed_cost`. Locus 1 is a site, 2 is a grid cell of that site, 3 is on a route edge. Unused coordinates are zero. Any other locus, a non-zero unused field, or `traversed_cost` greater than the edge cost is `Corrupt`. `task_id` is the actor's only non-terminal task, or 0. Locus 3 with `task_id` 0 is legal residual travel.

Site: `u64 site_id`, `u8 has_grid`. Edge: `u64 from_site`, `u64 to_site`, `u64 edge_id`, `u64 cost`. Cost is positive. Zero cost is `Corrupt`. Grid: `u64 site_id` or 0, `u32 width`, `u32 height`, then `width * height` passability bytes, row-major, 1 passable and 0 blocked. Both dimensions are zero when the site id is zero.

Stock: `u64 id`, `u64 definition_id`, `u8 class`, `u64 quantity`, `u8 owner_kind`, `u64 owner_id`. Class 1 is consumable. Class 2 is durable capacity. Owner kinds: 1 actor inventory, 2 container, 3 site stockpile, 4 in-transit cargo, 5 explicit reserve. Availability for a named task is on-hand minus active reservation quantities held by other tasks. A reservation is not an owner.

Reservation: `u64 id`, `u64 stock_id`, `u32 claimant`, `u64 quantity`, `u64 acquired_tick`, `u64 expiry_tick`, `u64 task_id`, `u8 state`. State 1 is active. State 2 is released. The active interval is `[acquired tick, expiry tick)`.

Task: `u64 id`, `u32 actor_id`, `u8 goal`, `u64 recipe_id`, `u8 stage`, `u8 interruption`, target (`u8 locus`, `u64 site`, `u32 x`, `u32 y`), `u64 progress_ticks`, `u64 created_tick`, `u64 due_tick`, unconsumed completion list, `u64 path_index`, `u64 step_count`, steps, `u64 reservation_count`, ascending reservation ids, `u64 planned_leg_count`, planned legs. `recipe_id` 0 means the planned legs are the declaration. Nonzero binds one scenario recipe.

Unconsumed completion list: `u64 count`, then records sorted by command key. Each record is `u64 target_tick`, `u32 actor_id`, `u32 producer_id`, `u64 sequence`, `u64 submitted_task_id`, `u64 resolved_task_id`. `submitted_task_id` is the payload id as admitted (`0` for the same-phase form). `resolved_task_id` is the non-zero task after resolution. Duplicates are rejected.

A path step is `u8 kind`, then `u64 edge_id` for kind 1, or `u8 direction` for kind 2. Directions are 1 north, 2 east, 3 south, 4 west. `interruption` is 0 while the task is non-terminal. `Done` stores interruption 0. Other terminal stages store the reason byte.

Stages: 1 Queued, 2 Reserved, 3 Travelling, 4 Executing, 5 Completing, 6 Done, 7 Cancelled, 8 Failed. Goals: `0` means a scripted task that was not created by decision; `1` eat, `2` rest, `3` work, `4` haul, `5` return home.

A second non-terminal task for the same actor is `InvalidState` on load and is rejected if a command would create it. Terminal tasks remain in the table. A reservation command whose task id is not `actor.task_id` fails preflight and does not mutate.

Transaction: `u64 id`, `u64 task_id`, `u8 source`, completion key (`u64 target_tick`, `u32 actor_id`, `u32 producer_id`, `u64 sequence`), `u8 applied` (always 1), `u64 leg_count`, legs. A leg is `u8 op`, `u64 stock_id`, `u64 definition_id`, `u8 class`, `u64 quantity`, `u8 owner_kind_after`, `u64 owner_id_after`. Quantity is at least 1. Ops: 1 debit, 2 credit, 3 transform out, 4 transform in. Debit and transform-out require an existing stock whose definition and class match, and owner-after fields 0. Credit and transform-in with stock id 0 allocate `next_stock_id` using the leg's definition and class. Owner kind 1 requires that actor. Owner kind 3 requires that site. Owner kinds 2, 4, and 5 require a holder record. A failed transaction is not stored and does not consume a transaction id.

Consumed completion registry: `u64 count`, then records sorted by command key. Each record is 41 bytes: the four key fields, `u64 submitted_task_id`, `u64 resolved_task_id`, `u8 role`. Role 1 is the source-1 transaction identity. Role 2 is an accepted key that did not create a transaction. The count is checked against the unread body before allocation. Duplicate keys, `resolved_task_id` 0, or two role-1 records for one task are `InvalidState`.

Pending command: `u64 target_tick`, `u32 actor_id`, `u32 producer_id`, `u64 sequence`, then the payload.

| Tag | Payload |
| --- | --- |
| 1 `SetMode` | `u8 mode` |
| 2 `CancelReservation` | `u64 reservation_id` |
| 3 `SetEnabled` | `u8 enabled` |
| 4 `ReserveStock` | `u64 stock_id`, `u64 quantity`, `u64 expiry_tick`, `u64 task_id` |
| 5 `ReserveBundle` | `u64 task_id`, `u64 expiry_tick`, `u64 pair_count`, then `(u64 stock_id, u64 quantity)` sorted by stock id |
| 6 `CompleteTask` | `u64 task_id`. Zero means the task created earlier in this command phase for this actor |
| 7 `BeginTask` | `u8 goal`, `u8 stage` (`Queued` or `Completing` only), `u64 recipe_id`, `u8 target_locus`, `u64 target_site`, `u32 target_x`, `u32 target_y`, `u64 due_tick`, then the planned-leg list |

Decision trace: `u32 actor_id`, `u64 evaluated_tick`, `u64 next_evaluation_tick`, `u8 winner_goal`, `u64 task_id`, then five candidates in goal order. Each candidate is `u8 goal`, `u8 eligible`, `u8 rejection_reason`, `i64 score`, five `i64` channel terms, and one `i64` relationship term.

### 3.1 Cursors

| Cursor | Issues | Rule |
| --- | --- | --- |
| `next_actor_id` | Actors | Greater than every actor id, and not zero |
| `next_record_id` | Tasks and reservations | Greater than every id in either table, and not zero. Task ids and reservation ids are one namespace |
| `next_stock_id` | Stocks | Greater than every stock id, and not zero |
| `next_transaction_id` | Applied transactions | Greater than every transaction id, and not zero |

### 3.2 Restore order

After a complete body and a matching trailer, first match: `Truncated`, `UnsupportedFormat`, `UnsupportedRevision`, `ChecksumMismatch`, trailing bytes `Corrupt`, then the semantic matrix. Count and grid bounds stay `Corrupt` during the parse. Semantic failure is `InvalidState` unless named otherwise:

1. Fixture id is 3 and the content hash equals the scenario block.
2. No task id equals a reservation id.
3. Cursors are above the ids in their namespaces and are not zero.
4. `next_event_id` is at least 1.
5. Ids inside each table are strictly increasing.
6. Channels are in `0..=1000`. Enabled, mode, and locus fields are legal. Holders exist for every referenced kind 2, 4, and 5.
7. `task_id` is 0 or one non-terminal task of that actor.
8. Reservation task ids exist. The claimant is that task's actor. Active quantities do not exceed on-hand. Expiry is greater than acquired.
9. Task reservation ids exist, are ascending, and name that task. `recipe_id` is 0 or a scenario recipe. `path_index` is at most the step count.
10. Source 1 transactions use producer 1 and a role-1 registry record with the same key, submitted id, and resolved task id. Source 2 uses producer 0 and sequence 0 and has no registry record. `applied` is 1.
11. Role-2 records share that resolved task and do not have their own transaction. Registry keys and pending keys are strictly sorted. An unconsumed key is not also in the registry. Every registry record has non-zero `resolved_task_id`.
12. At most one trace per actor. Holder ids are unique. Edge cost is positive. Locus 3 with `task_id` 0 is legal.
13. The body ends at the trailer.

## 4. Events

`H0 = SHA-256("SH:C03:EVENT-INIT:v1")` = `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c`. This digest was computed from that ASCII string. It was not taken from a simulation run.

Each later digest is `SHA-256("SH:C03:EVENT:v1" || previous || canonical_bytes)`. Canonical bytes are `u64 id`, `u64 tick`, `u32 actor_id`, `u8 kind`, then the payload.

| Kind | Name | Payload |
| --- | --- | --- |
| 1 | ReservationAccepted | `u64 reservation_id`, `u64 stock_id`, `u64 quantity` |
| 2 | ReservationRejected | `u64 stock_id`, `u64 quantity`, `u8 reason` |
| 3 | ReservationReleased | `u64 reservation_id`, `u8 reason` |
| 4 | LegsApplied | `u64 transaction_id`, `u64 task_id` |
| 5 | TaskFailed | `u64 task_id`, `u8 reason` |
| 6 | DecisionMade | `u8 winner_goal`, `u64 task_id`, `u64 next_wake_tick`, five `i64` scores |
| 7 | TaskStage | `u64 task_id`, `u8 stage` |

Reason bytes: 0 none, 1 disablement, 2 cancellation, 3 expiry, 4 lost contention, 5 unreachable, 6 insufficient availability, 7 invalid leg, 8 released on completion, 9 actor already has a non-terminal task, 10 no legal target, 11 relationship constraint.

A new fixture starts the cursor at 1 and the digest at H0. Each emitted event consumes the current cursor, then the cursor increases by 1, then the digest updates. A rolled-back boundary changes neither. A silent no-op emits nothing. Fixture 1 keeps `SH:C01:EVENT:v1`. Fixture 2 keeps `SH:C02:EVENT:v1`. Production events do not enter the C0.1 `Outcome` enum.

One order is legal:

| Commit | Order |
| --- | --- |
| Accept a reservation and change stage | Kind 1 in ascending new reservation id, then one kind 7 |
| Reject a reservation or bundle | One kind 2 |
| Invalidate | Kind 3 in ascending reservation id, then kind 7 `Failed` or `Cancelled`, then kind 5. Zero reservations omit kind 3 |
| Decision with a winner | Kind 1 in stock-id order, then kind 7, then kind 6 |
| Decision with no winner | Kind 6 only |
| Work changes stage and does not finish | One kind 7 |
| Work completes | Kind 4, then kind 3 reason 8 in ascending reservation id, then kind 7 `Done`. Zero reservations omit kind 3 |
| Residual edge progress or residual arrival | No event |

## 5. Admission and phases

Fixture 3 is driven with `drive`. `step` returns `UseDrive` and does not mutate. Fixture 1 and fixture 2 keep their frozen precedence. The table below is fixture 3 only.

Rules are tested in order. The first match is the result. An error rolls the whole boundary back. A no-op does not emit and does not admit the command. A boundary whose commands are all no-ops still advances the tick.

| # | Condition | Result |
| --- | --- | --- |
| 1 | `submission_tick` is not the current tick | `WrongBoundary` |
| 2 | Any producer is not 1 | `InvalidProducer`. Producer 0 never replays a generated transaction |
| 3 | Any actor id is unknown | `UnknownActor` |
| 4 | The same key appears twice in the batch, or the key is already pending | `ConflictingIdentity` |
| 5 | The key is in the consumed registry and the submitted `CompleteTask` id equals the stored `submitted_task_id` | No-op, including when the target tick is old. Role 1 and role 2 both match this way |
| 6 | The key is in the consumed registry and the submitted `CompleteTask` id differs from the stored `submitted_task_id` | `ConflictingIdentity`, even if both ids would resolve to the same task |
| 7 | The key is in the consumed registry and the payload is not `CompleteTask` | `ConflictingIdentity` |
| 8 | The payload is a cancel, the reservation exists, and the command actor is not the stored claimant | `InvalidPayload` |
| 9 | The payload is a cancel, the reservation exists, the actor is the claimant, and that task is already terminal | No-op, including when the target tick is old |
| 10 | `target_tick` is less than or equal to the current tick, and no earlier rule matched | `LateTarget` |
| 11 | Bad tag, unsorted or repeated bundle, non-positive quantity, an empty reservation interval, or `BeginTask` while the actor `locus` is 3 | `InvalidPayload` |
| 12 | `CompleteTask` does not name the actor's current non-terminal task, and it is not the same-phase zero form that resolves to the task created earlier in this phase | `InvalidPayload` |
| 13 | Non-zero cancel whose reservation task is non-terminal and is not `actor.task_id` | `InvalidPayload` |
| 14 | Unknown cancel id, or cancel 0 when the actor has no current task | No-op |
| 15 | Otherwise | Admit |

Rows 4 and 7 are the durable-identity rules. For fixture 3, a durable identity is a pending command or a consumed completion registry record. An ordinary non-completion command that has been applied and is no longer present in canonical state has no persistent identity: a later submission with a target tick at or before the current tick reaches row 10 and returns `LateTarget`, while a later submission with a future target is an ordinary new command. Rows 2–4 still run before row 10. Restore does not reconstruct hidden history, and identical canonical checkpoint bytes produce identical later behavior. This is the approved H09 amendment; it does not change revision 1, the 41-byte registry, or any published vector.

Due commands sort by `(target tick, actor ID, producer ID, sequence)`, ascending. That order is the C03 fixture order. It is not global gameplay fairness.

After admission succeeds, the tick increments, then:

1. Mode. Apply due `SetMode` in key order. Emit nothing.
2. Command. Apply `SetEnabled`, `BeginTask`, reservations, cancel marks, and `CompleteTask` in key order. `CompleteTask` appends a completion record and does not apply legs.
3. Invalidation. One reason, in this priority: disablement, cancellation, expiry. Walk actors, then tasks, in ascending id. Release each active reservation of that task once, in ascending id, with that reason. Move any unconsumed completion records to the consumed registry as role 2. Set `Failed` for disablement, expiry, and unreachable. Set `Cancelled` for cancellation. Emit the invalidation order. Then set `actor.task_id` to 0. Do not apply legs. Do not clear mid-edge locus.
4. Decision. §7.
5. Work. §8 and §9, including residual edge travel after task-owned work.

Fixtures 1 and 2 do not enter production phases 3, 4, or 5. Fixture 2 still runs its synthetic work phase.

## 6. Reservations, bundles, and transactions

A request is accepted only when every named stock has available quantity at least the requested amount, where available quantity excludes other tasks' active reservations. Ids are allocated from `next_record_id` in ascending stock-id order. A rejection allocates nothing. A bundle is all or nothing. Quantity zero, an expiry less than or equal to the acquired tick, an unsorted or repeating pair list, and a `u64` sum that would overflow fail preflight. There is no partial fill.

### 6.1 Reservation promotion and `due_tick`

When a `ReserveStock` or `ReserveBundle` command is accepted for the actor's current non-terminal task, or when a decision commit reserves the winner's bundle, the following occurs in one atomic commit inside the command or decision phase, before invalidation ends and before work runs:

1. Allocate reservation ids and write active reservation rows.
2. Set the task stage to `Reserved` when the prior stage was `Queued`, or keep the stage already chosen by decision creation (`Reserved`, `Travelling`, or `Executing`).
3. Emit the acceptance event order for that commit.
4. Assign `due_tick = staged_tick + travel_ticks + 2`, where `travel_ticks` is the cost of the task's stored path, or 0 if the path is empty. This is the approved due formula evaluated at the reservation commit's staged tick. For a task created in that same staged tick, `created_tick` equals that staged tick, so the assignment equals `created_tick + travel_ticks + 2`.
5. Do not change `created_tick`. The creation-tick work skip continues to key off `created_tick` alone.
6. For reservations created by decision, each new reservation's `expiry_tick` is `due_tick + 1`. For scripted `ReserveStock` / `ReserveBundle`, the command's `expiry_tick` is stored as submitted and must be greater than the acquired tick; it is not rewritten by step 4.

A rejected reservation or bundle performs none of those writes. If a later check in the same boundary fails, the whole boundary rolls back, including any `due_tick` assignment from this commit.

`BeginTask` requires `actor.task_id` 0 and `locus` not 3. It allocates the task, stores that id on the actor, sets `created_tick` to the staged tick, and leaves the unconsumed completion list empty. A disabled subject emits kind 2 with stock 0, quantity 0, reason 1, and the boundary commits. No task is created. Scripted `BeginTask` carries its own `due_tick` and does not use §6.1 unless a later reservation promotion overwrites `due_tick` under step 4.

`CompleteTask(0)` is legal only in the same command phase as a `BeginTask` for that actor. It stores `submitted_task_id` 0 and `resolved_task_id` equal to the task created earlier in that phase. `CompleteTask(N)` stores both fields as `N` when that is the actor's current non-terminal task.

Cancel with a non-zero id requires the command actor id to equal the claimant and the reservation's task to equal `actor.task_id` while that task is non-terminal. Cancel with id 0 marks the current task. The command phase records the mark. Invalidation releases. An unknown id, or a cancel of an already terminal task by the stored claimant, is a no-op after the structural checks in §5.

`SetEnabled` writes the subject bit in the command phase. Clearing the bit fails the current task in invalidation, including a task with no reservations. Setting the bit from 0 to 1 creates no task, suppresses that actor's decision on the same tick, and, when `next_wake_tick` equals that tick, moves the wake forward by the wake interval. Fixture 1 and fixture 2 reject this payload. Their enabled flags do not change after construction.

Legs copy the planned-leg list and apply in order on a staged copy inside the work commit, only when invalidation left the stage at `Completing` and either the unconsumed list is non-empty or `due_tick` equals the staged tick, and only when `created_tick` is not the staged tick. Source 1 stores the lowest unconsumed key and producer 1. Every unconsumed record moves into the consumed registry: the lowest as role 1, the others as role 2. The list becomes empty. Source 2 is the due-tick path with an empty unconsumed list. It stores producer 0 and sequence 0 and creates no registry record.

A debit or transform-out must fit in available quantity after earlier legs of the same transaction. If the task has active class-1 reservations on a stock, the planned debit plus transform-out of that stock must equal the sum of those reservation quantities. Class-1 quantity changes only through those legs. Reason-8 release does not debit again. Existing class-2 quantity is immutable. Changing it is `InvalidLeg`. Debit and transform-out require `owner_kind_after` 0 and `owner_id_after` 0.

### 6.2 Credit and transform-in

Credit (op 2) and transform-in (op 4) use the leg's `definition_id`, `class`, `quantity`, `owner_kind_after`, and `owner_id_after`. Quantity is at least 1.

**`stock_id` 0.** Allocate `next_stock_id`. The new stock stores the leg's definition, class, quantity, and owner fields. Owner kind 1 requires that actor. Owner kind 3 requires that site. Owner kinds 2, 4, and 5 require a holder record. Then advance `next_stock_id`.

**`stock_id` ≠ 0.** The stock must already exist. The leg's `definition_id` and `class` must equal the stock's definition and class. `owner_kind_after` and `owner_id_after` must equal the stock's current `owner_kind` and `owner_id`. A credit or transform-in does not change owner. Add `quantity` to the stock with checked `u64` addition. Overflow is `InvalidLeg`. Definition mismatch, class mismatch, owner mismatch, missing stock, or a class-2 quantity change is `InvalidLeg`.

A later-leg failure means an earlier leg in the staged copy would have succeeded and a subsequent leg fails. The whole boundary returns `InvalidLeg`, emits nothing, consumes no transaction id, releases nothing, and restores the pre-call bytes. Transactions committed on earlier ticks remain. Conservation: every quantity change appears in a stored applied leg; a rolled-back boundary changes no on-hand quantity.

On success, each active reservation of that task is released once, reason 8, ascending id. A second release emits nothing. `Done` stores interruption 0 and clears `actor.task_id`.

A repeated terminal release and a matching consumed completion replay change no stock, owner, reservation, task, transaction, digest, or cursor. Identical replay means the same command key and the same submitted `CompleteTask` id.

Nonzero `recipe_id` requires the scenario recipe's input definition and quantity on one debit or transform-out, and its output definition, quantity, and owner on one credit or transform-in.

## 7. Needs

No GOAP search and no floating point. Evaluation is the decision phase, actors in ascending id, only when `next_wake_tick` equals the staged tick, the actor was not enabled from 0 earlier in that tick, the actor has no non-terminal task, and goals that need a site see a site locus. Restore before that wake emits no second kind 6.

For each goal in order 1 through 5, copy the five channels and the relationship term into the trace. The score is the checked `i64` dot product of the channels and that goal's weights, plus that goal's role bonus. The relationship term is not added to the score. Overflow is `CapacityExceeded` for the whole boundary: no trace, no event, no tick advance.

Ineligibility, first match: disabled, relationship below the goal minimum, non-terminal task already present, home required and missing, not on a site when a site or new path is required, path search fails, required stock unavailable. Stock and site choice is the lowest legal id. An ineligible goal keeps its score. The winner is the highest eligible score. A tie keeps the lower goal ordinal. If every goal is ineligible, the winner and task id are 0 and only kind 6 is emitted.

| Goal | Eligible when | Target and stock | Stage at creation |
| --- | --- | --- | --- |
| Eat | Actor inventory holds a class-1 definition-1 stock with available quantity at least 1, and the actor is on a site | Current site. Lowest such stock id. Reserve 1. One debit leg of 1 | `Executing` if no path is required |
| Rest | `home_site_id` names a site and a path exists, or the actor is already there | Home. No stock and no legs | `Travelling` if a path is required, otherwise `Executing` |
| Work | Current site has a class-2 definition-4 stock and a class-2 definition-5 stock, each with available quantity at least 1 | Current site. Lowest stock id with definition 4 (tool). Lowest stock id with definition 5 (workstation). One bundle of those two. No legs. One stock cannot fill both roles because the definition ids differ | `Executing` when both exist |
| Haul | Inventory has a class-1 stock whose definition is not 1, available quantity at least 1, and home is reachable | That lowest stock id. Debit 1. Credit the same definition and class to owner kind 3 at home: if a stock with that definition, class, and owner already exists, credit that `stock_id`; otherwise credit `stock_id` 0 | `Executing` at home, otherwise `Travelling` |
| Return home | Same site rule as rest | Home. No stock and no legs | Same shape as rest |

At task creation by decision, `due_tick = created_tick + travel_ticks + 2`. Decision-created reservation expiry is `due_tick + 1`. Travel ticks are the stored path cost, or 0. When the winner's bundle is reserved in that same commit, §6.1 assigns the same `due_tick` value because `created_tick` equals the staged tick. Scripted `BeginTask` carries its own `due_tick`. Scripted reservation promotion uses §6.1.

A winner creates one task with `created_tick` equal to the staged tick. The winner's bundle, if any, is reserved in that commit under §6.1. `next_wake_tick` becomes the staged tick plus the fixture wake interval whether or not there was a winner. Work does not run that task on its `created_tick`.

## 8. Jobs

Work orders current tasks by `(actor id, task id)`. First match:

| # | Condition | Result |
| --- | --- | --- |
| 1 | `created_tick` equals the staged tick | No change and no event |
| 2 | Stage is `Queued` | No change and no event, including when `due_tick` is below the tick |
| 3 | Stage is not `Queued` and `due_tick` is below the staged tick | `InvalidState` for the boundary |
| 4 | `Reserved`, empty path, at the target | `Executing`, one kind 7 |
| 5 | `Reserved`, path non-empty | `Travelling`, one kind 7 |
| 6 | `Reserved`, empty path, not at the target | `InvalidState` for the boundary |
| 7 | `Travelling`, locus 3, path empty | Spend one cost on the current edge. Do not search |
| 8 | `Travelling`, stored path | One progress step, or arrive and enter `Executing` |
| 9a | `Executing`, goal 0, unconsumed completion list empty | No change and no event |
| 9b | `Executing`, goal 1–5, or goal 0 with a non-empty unconsumed completion list | `Completing`, one kind 7, no legs |
| 10 | `Completing`, unconsumed list non-empty or `due_tick` equals the tick | Legs, one release per own reservation, `Done` |
| 11 | `Completing`, neither requested nor due | No event |

`progress_ticks` increases by 1 on each advancing work-phase transition in the table above. Reservation promotion in §6.1 does not increment `progress_ticks`. Rest and return home have no reservations and no legs. Their successful completion is kind 4 with an empty leg list, then kind 7 `Done`. Disablement or a zero-id cancel on that path is kind 7 `Failed` or `Cancelled`, then kind 5, and no kind 4. Work releases one tool and one workstation without debiting them. A scripted haul completes one hop per task. Actor → store, store → cargo, and cargo → site are three successive current tasks. Autonomous haul is one debit and one credit in a single task.

After task-owned work, residual edge travel runs for every actor with `locus` 3 and `task_id` 0, in ascending actor id. Each spends one edge cost with no event. Arrival places the actor on the destination site, clears `edge_id`, sets `traversed_cost` to 0, and emits no event. Residual travel runs for enabled and disabled actors.

### 8.1 Owner-authorized S1 H10 clarification

Goal 0 is scripted, not an autonomous decision goal. A completion command only
appends provenance in the command phase. For goal 0 in Executing, a non-empty
unconsumed list enables the single work transition to Completing (one kind 7 and
one progress tick); legs never run in that same transition. Requests received in
Queued or Reserved remain on the task until its ordinary stage transitions reach
Completing. Reservation promotion and Reserved-to-Executing behavior are unchanged.

The creation-tick skip and overdue rule precede these rules for every goal. There
is no implicit extension of due_tick. A request reaching Executing at its due tick
can advance it to Completing but cannot complete it that tick. If it survives
invalidation, the following overdue boundary returns InvalidState with full
rollback. A request with an old target still follows §5 admission; a future-target
request cannot override an overdue task. With no request, goal-0 Executing remains
unchanged through its due tick and an overdue attempt fails the same way.

Completing retains rule 10 for every goal: it applies only on a later-than-creation
tick, with a request or at its due tick, after invalidation. An explicitly installed
or BeginTask-created Completing task may therefore use the existing source-2
due-tick path without a request; no new autonomous goal selection is introduced.
If creation and due coincide, creation skip still wins and the next surviving
boundary is overdue. Goals 1–5 keep their unconditional Executing-to-Completing
transition. No transition performs two work steps in one tick.

Disablement, cancellation and expiry still run before work, in that priority.
They release once, move every unconsumed key to role 2, set the terminal stage and
clear actor.task_id, even when work would otherwise be overdue. Successful work
uses the existing kind-4 / ascending reason-8 releases / kind-7 Done order and the
lowest unconsumed key as role 1. Requests and progress persist in existing fields;
restore does not emit prefix events or add a readiness flag. An error restores
every pre-call byte, including any completion admitted during the failed boundary.

## 9. Navigation

The actor record is the only position. Route search is used for a site target. Grid search is used for a cell of the actor's current site. One path does not mix them.

Route successors sort by destination site id, then edge id. Grid successors are north, east, south, west, skipping blocked and out-of-range cells. North increases y. East increases x. Search minimizes the sum of costs. An edge costs its `cost` field. A grid step costs 1. That cost is both the selection weight and the duration in work-phase ticks. Sums use checked `u64` addition. Overflow is `CapacityExceeded`, stores no path, and is not an unreachable result. Equal total cost keeps the lexicographically smaller path: route steps compare `(destination site id, edge id)`, grid steps compare the direction byte. On the fixture 3 by 3 grid from `(0,0)` to `(2,2)` with center blocked, the selected path is north, north, east, east.

A missing path at decision rejects the goal with reason 5 before a task is created. A travelling task that already has an empty path and is standing on a site fails in work with the invalidation event order and reason 5. It transfers nothing. A stored path is authoritative across save and load. Search runs only for an empty path on a travelling task that is on a site, or when a decision selects a goal that needs a path. Search does not run while `locus` is 3. No commit relocates a mid-edge actor to an endpoint to start a search.

Cancel and failure leave the actor on the edge. Residual travel in §8 then spends the remaining cost. Remaining tick cost is `edge.cost - traversed_cost`. That cost is never discarded and never spent as a teleport.

A transfer has one owner before the commit and one owner after it. A failed leg does not leave stock on an intermediate owner.

## 10. Save, replay, and compatibility

The checkpoint holds every future-affecting field in §3, including position, path progress, residual mid-edge state, the current task, the unconsumed completion list, the consumed registry, reservations, the trace, and pending commands. Resume continues at the saved tick. Prefix events are not emitted again. The first new event uses the saved cursor. Suffix plus prefix equals an uninterrupted run. R1 is the requirement. R2 is not claimed.

A failed `drive` leaves the previous domain bytes unchanged. Grouped boundaries match one boundary at a time.

## 11. Commands that must not be added

No decay curve beyond the fixture channels. No planner search. No pathfinding crate. No idle-tick skip. No cohort. No Bevy type in the kernel. No new C0.1 `Outcome` variant. No field added to `SH:C02:STATE` revision 1. No second non-terminal task per actor. No OC-EDGE cost-2 edge. No snap-to-endpoint mid-edge search.

## 12. Fixture configuration

Wake interval 4. Weights: eat `(1000,0,0,0,0)`, rest `(0,1000,0,0,0)`, work `(0,0,1000,0,0)`, haul `(0,0,0,1000,0)`, return home `(0,0,0,0,1000)`. Role bonuses 0. Relationship term 0. Every relationship minimum is `-9223372036854775808`. Channels do not decay. The fixture sets channel values so the winner is determined without a decay table.

Sites 1 and 2. Edges `(1,2,1,cost 1)` and `(2,1,2,cost 1)`. Site 9 has no edge. No edge 4. No edge 5. Site 1 has a 3 by 3 grid, all cells passable except the center. Successor order on the grid is north, east, south, west.

M0-T06 stocks, owner kind 3, owner id 1, on-hand 5 each:

| Stock id | Name | definition_id | class |
| --- | --- | --- | --- |
| 101 | beds | 3 | 2 |
| 102 | tools | 4 | 2 |
| 103 | food | 1 | 1 |
| 104 | workstations | 5 | 2 |

Actors 1 through 20 each have one current task with ids 1 through 20, `due_tick` 0 while `Queued`. Preinstalled scripted tasks use fixture packing fields: `goal` 0, `recipe_id` 0, `interruption` 0, target locus 1, target site 1, target x 0, target y 0, `progress_ticks` 0, empty planned legs. `next_record_id` starts at 21. On a successful bundle at staged tick 1 with an empty path, §6.1 sets `due_tick` to 3. Work then advances each winner once to `Executing` and sets `progress_ticks` to 1. These counts, definition ids, and packing fields are fixture configuration.

M0-T07 holders 20 kind 2 and 30 kind 4 exist at construction. `next_record_id` starts at 401. Recipe 1: input definition 1 quantity 1, output definition 2 quantity 1, owner kind 3, owner id 2. Definition 2 is the recipe output. It is not a tool or workstation definition.
