# C0.3 G3 R2.1 Remediation Proposal

Version R2.1-final · 17 November 2026  
Status: proposal. Amendments are not approved. Normative C0.3 documents were not edited.

Approved OD-01 through OD-16 stay in force. H0 stays `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c`.

R2.1-final changes only: the M0-T07 later-leg oracle uses available stock 14; residual edge travel continues after `task_id` is cleared; the completion registry stores submitted and resolved task ids.

## 1. F01 and the M0-T07 timeline

`BeginTask` sets `created_tick` to the staged tick. `CompleteTask` appends a completion record and does not apply legs. Work's first check is the creation-tick skip. The skip wins over both a non-empty request list and `due_tick == staged tick`.

Initial state, before `drive(0, …)`. Nothing in the later rows is inferred from outside this state.

| Field | Value |
| --- | --- |
| Seed, tick, wake | 100, 0, 100 |
| Actor 1 | Enabled, mode active, channels 0, relationship term 0, home site 1, locus site 1, `task_id` 0 |
| Cursors | `next_actor_id` 2, `next_record_id` 401, `next_stock_id` 11, `next_transaction_id` 1, `next_event_id` 1 |
| Stock 10 | Definition 1, class 1, quantity 1, owner kind 1, owner 1 |
| Holders | 20 kind 2, 30 kind 4 |
| Places | Sites 1, 2, and 9. Edges 1 and 2 only. No reservations. Consumed registry empty |
| Recipe 1 | Input definition 1 quantity 1. Output definition 2 quantity 1, owner kind 3, owner 2 |

Every scripted command uses actor 1 and producer 1. `BeginTask` uses stage `Completing`, an empty path, no reservations, locus site 1, and `due_tick` equal to the staged tick of that command plus 1. Hops 1 through 3 use goal 4 and `recipe_id` 0. Task 404 uses goal 3 and `recipe_id` 1. Task 405 uses goal 4 and `recipe_id` 0. `CompleteTask` on a create tick uses submitted task id 0, which resolves to the task created earlier in that same command phase.

| Staged tick | Submission | Commands, target = staged tick | Work | Events | State after the tick |
| --- | --- | --- | --- | --- | --- |
| 1 | `drive(0, …)` | Begin sequence 1, task 401, hop 1 legs. Complete sequence 2, submitted 0 | Skipped | 1: kind 7, task 401, stage 5 | Request list holds sequence 2, submitted 0, resolved 401. Stock 10 unchanged. `next_record_id` 402 |
| 2 | `drive(1, [])` | None | Apply hop 1 | 2: kind 4, transaction 1. 3: kind 7, stage 6 | `task_id` 0. Stock 10 quantity 0. Stock 11 at holder 20. Registry holds sequence 2 as role 1. `next_stock_id` 12. `next_transaction_id` 2 |
| 3 | `drive(2, …)` | Begin sequence 3, task 402, hop 2. Complete sequence 4, submitted 0 | Skipped | 4: kind 7, stage 5 | Task 402 current |
| 4 | `drive(3, [])` | None | Apply hop 2 | 5: kind 4, transaction 2. 6: kind 7, stage 6 | Stock 11 quantity 0. Stock 12 at cargo 30. `next_stock_id` 13 |
| 5 | `drive(4, …)` | Begin sequence 5, task 403, hop 3. Complete sequence 6, submitted 0 | Skipped | 7: kind 7, stage 5 | Task 403 current |
| 6 | `drive(5, [])` | None | Apply hop 3 | 8: kind 4, transaction 3. 9: kind 7, stage 6 | Stock 12 quantity 0. Stock 13 at site 2. `next_stock_id` 14 |
| 7 | `drive(6, …)` | Begin sequence 7, task 404, recipe 1. Complete sequence 8, submitted 0 | Skipped | 10: kind 7, stage 5 | Task 404 current |
| 8 | `drive(7, [])` | None | Apply recipe | 11: kind 4, transaction 4. 12: kind 7, stage 6 | Stock 13 quantity 0. Stock 14 definition 2 class 1 quantity 1 at site 2. `next_stock_id` 15. `next_transaction_id` 5 |
| 9 | `drive(8, …)` | Begin sequence 9, task 405, `recipe_id` 0, later-leg legs below. Complete sequence 10, submitted 0 | Skipped | 13: kind 7, stage 5 | Task 405 current. `due_tick` 10. Request list holds sequence 10. Stock 14 still quantity 1. `next_event_id` 14. `next_record_id` 406 |
| 10 | `drive(9, [])` | None | Later-leg `InvalidLeg` | None | Pre-call bytes restored. Tick remains 9. `next_event_id` remains 14. Task 405 still `Completing`. Sequence 10 still unconsumed. Stocks 10 through 14 unchanged from tick 9 |

Hop legs:

| Hop | Debit or transform-out | Credit or transform-in |
| --- | --- | --- |
| 1 | Stock 10, definition 1, class 1, quantity 1, owner-after 0 | Credit stock 0, definition 1, class 1, quantity 1, owner kind 2, owner 20 |
| 2 | Stock 11, definition 1, class 1, quantity 1 | Credit stock 0, definition 1, class 1, quantity 1, owner kind 4, owner 30 |
| 3 | Stock 12, definition 1, class 1, quantity 1 | Credit stock 0, definition 1, class 1, quantity 1, owner kind 3, owner 2 |
| Recipe task 404 | Transform-out stock 13, definition 1, class 1, quantity 1 | Transform-in stock 0, definition 2, class 1, quantity 1, owner kind 3, owner 2 |
| Invalid task 405 | Debit stock 14, definition 2, class 1, quantity 1, owner-after 0 | Credit stock 0, definition 2, class 1, quantity 1, owner kind 3, owner id 0 |

### Why tick 10 is a later-leg failure

After tick 8, stock 13 quantity is 0. A first leg that transform-out stock 13 would fail immediately for insufficient availability. That is not the required later-leg case.

Task 405 therefore uses available stock 14. On the staged copy for tick 10:

1. The first leg debits stock 14. Available quantity is 1. The staged copy would hold stock 14 quantity 0 and would allocate no credit yet.
2. The second leg credits owner id 0. Owner validation fails. The boundary returns `InvalidLeg`.
3. The staged copy is discarded. Stock 14 remains quantity 1. No transaction id is consumed. No event is emitted. Tick, cursor, digest, task 405, and the unconsumed list match the tick-9 save.

Ticks 1 through 9 are preserved, including the successful recipe that created stock 14. No reservations exist on these hops, so success events have no kind 3.

## 2. F02 provenance

Command key is `(target_tick, actor_id, producer_id, sequence)`. Payload identity for `CompleteTask` is the submitted task id. Resolved identity is the non-zero task after same-phase resolution.

| Admitted form | `submitted_task_id` | `resolved_task_id` |
| --- | --- | --- |
| `CompleteTask(0)` after `BeginTask` created 401 in the same phase | 0 | 401 |
| `CompleteTask(401)` while `actor.task_id` is 401 | 401 | 401 |

Those two forms are different payloads. They may not share one command key in one history. If the same key is later submitted with the other form, AM-05 returns `ConflictingIdentity`.

Unconsumed records live on the task until the task leaves the non-terminal stages. Consumption moves them into the world registry. It does not drop them.

Collision, both runs starting from the same checkpoint. Task 11 is `Executing`, `created_tick` 0, `due_tick` 9, list empty, actor 1, no reservation. Submission tick is 4. The command targets tick 5.

| Run | Command | Checkpoint at the end of staged tick 5 |
| --- | --- | --- |
| A | `CompleteTask` sequence 7, submitted 11 | Stage `Completing`. Unconsumed list contains sequence 7, submitted 11, resolved 11 |
| B | `CompleteTask` sequence 8, submitted 11 | Stage `Completing`. Unconsumed list contains sequence 8, submitted 11, resolved 11 |

Those checkpoints differ. Restore resumes from the stored list. The following empty boundary applies one source-1 transaction. Run A stores sequence 7 as role 1. Run B stores sequence 8 as role 1. The transaction keys differ. Producer 0 is not invented.

Both keys in one batch, sequences 7 and 8, same target, actor, producer, and submitted 11. Both are stored, sorted by key. The apply boundary writes one transaction, role 1 on the lower key, role 2 on the higher key. A second transaction is not created. After that boundary the two checkpoints still differ, because role 2 remains.

Replay after that save:

| Replay | Result |
| --- | --- |
| Same key, submitted 11, role 1 or role 2 | No-op. Tick advances. No second transaction |
| Same key, submitted 0 | `ConflictingIdentity`. Tick unchanged |
| Same key, submitted 99 | `ConflictingIdentity`. Tick unchanged |

A create-tick history that stored submitted 0 and resolved 401 replays as a no-op only when the resubmitted payload is again `CompleteTask(0)`. Resubmitting `CompleteTask(401)` under that key is `ConflictingIdentity`.

A cancel or failure before apply moves the unconsumed keys to role 2 only. Replay of those keys with the matching submitted id is a no-op and still creates no transaction.

Registry decode rejects a count that does not fit in the unread body, a duplicate key, `resolved_task_id` 0, and two role-1 records for one task.

## 3. F03 conservation

Validation uses a staged copy, legs in order.

`other_reserved(stock, task)` is the sum of active reservation quantities on that stock whose task id is not this task. Available quantity is staged on-hand minus `other_reserved`. A debit or transform-out greater than that available quantity is `InvalidLeg`. Earlier legs in the copy already reduce staged on-hand. Other tasks' reservations stay in the subtraction.

Own class-1 reservations are the quantity this task may consume. If any exist on a stock, the planned debit plus transform-out of that stock must equal their sum. The release that follows a successful debit does not subtract again. A class-1 task with no reservation may debit available quantity directly. That is the M0-T07 shape.

Existing class-2 on-hand does not change. Changing it is `InvalidLeg`. Reason-8 release of a class-2 reservation changes only the reservation state.

Success releases each active reservation of the completing task once, ascending id, reason 8. Failure releases none of them. A later repeated release emits nothing.

### Two-task oracle

Initial bytes, tick 0, seed 100, wake 100. Actors 1 and 2 enabled, locus site 1. Stock 10, definition 1, class 1, quantity 2, owner kind 1, owner 1. `next_record_id` 5. `next_stock_id` 11. `next_transaction_id` 1. `next_event_id` 1. Consumed registry empty.

| Record | Contents |
| --- | --- |
| Task 1 | Actor 1, `Completing`, `created_tick` 0, `due_tick` 1, `recipe_id` 0, one debit of stock 10 quantity 1, unconsumed key target 1, actor 1, producer 1, sequence 1, submitted 1, resolved 1 |
| Reservation 3 | Stock 10, claimant 1, quantity 1, task 1, active, acquired 0, expiry 2 |
| Task 2 | Actor 2, `Queued`, `created_tick` 0, `due_tick` 0, no legs and no path. Work does not advance `Queued`, so this boundary emits nothing for actor 2 |
| Reservation 4 | Stock 10, claimant 2, quantity 1, task 2, active, acquired 0, expiry 6 |

`drive(0, [])` stages tick 1. Available to task 1 is `2 - 1 = 1`. The debit of 1 fits.

| Oracle | Result |
| --- | --- |
| Success | Transaction 1, source 1, sequence 1. Stock 10 quantity 1. Reservation 3 released, reason 8. Reservation 4 still active. Task 1 `Done`, actor 1 `task_id` 0. Task 2 stays `Queued`. Events: kind 4 transaction 1, kind 3 reservation 3 reason 8, kind 7 task 1 `Done`. A second `drive` does not emit those events again and does not create transaction 2 |
| Same world, debit quantity 2 | Available 1 is below 2. `InvalidLeg`. Tick stays 0. Quantity stays 2. Both reservations stay active. No transaction. No event |
| Same world, task 1 has no reservation and on-hand is 1 with reservation 4 still quantity 1 | Available 0. `InvalidLeg`. Reservation 4 remains |

Class 2 check, separate world: on-hand 1, one active reservation of quantity 1 owned by the completing task, no legs. Success leaves quantity 1, releases that reservation once, and does not create a second release on replay.

## 4. F04 namespaces and restore

| Cursor | Issues | Rule |
| --- | --- | --- |
| `next_actor_id` | Actors | Greater than every actor id, and not zero |
| `next_record_id` | Tasks and reservations | Greater than every id in either table, and not zero |
| `next_stock_id` | Stocks | Greater than every stock id, and not zero |
| `next_transaction_id` | Applied transactions | Greater than every transaction id, and not zero |

C03-T01a installs tasks 1 and 2 and `next_record_id` 3. The first reservation is 3. M0-T06 installs tasks 1 through 20 and `next_record_id` 21. Winning reservations are 21 through 40, four per winning actor, actor order, and stock-id order inside each bundle. M0-T07 starts at `next_record_id` 401 and issues 401 through 405.

Restore after the trailer matches. The first failing check wins. `InvalidState` unless the parse already returned `Corrupt`:

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
12. At most one trace per actor. Holder ids are unique. Edge cost is positive. Locus 3 with `task_id` 0 is legal residual travel.
13. The body ends at the trailer.

A checkpoint with task 3 and reservation 3 fails check 2 even if `next_record_id` is 4.

## 5. Five goal plans

Decision runs for an enabled actor whose `next_wake_tick` equals the staged tick, who was not changed from disabled to enabled earlier in that tick, who has no non-terminal task, and whose `locus` is a site when the goal needs a site. Actors are visited in ascending id. The relationship term gates eligibility. It is not added to the score. The score is the checked `i64` dot product of the five channels and that goal's weights, plus that goal's role bonus. Overflow is `CapacityExceeded` for the boundary. The winner is the highest eligible score, then the lower goal ordinal. `next_wake_tick` becomes the staged tick plus the wake interval whether or not there is a winner.

Ineligibility, first match: disabled, relationship below the goal minimum, non-terminal task already present, home required and missing, not on a site when a site or new path is required, path search fails, required stock unavailable. Stock and site choice is the lowest legal id. A new reservation expires at `due_tick + 1`. `due_tick = created_tick + travel_ticks + 2`. Travel ticks are the stored path cost, or 0. The creation tick stores the path and does not spend it.

| Goal | Eligible when | Target and stock | State effect | Terminal path | Save and replay |
| --- | --- | --- | --- | --- | --- |
| Eat | Actor inventory holds a class-1 definition-1 stock with available quantity at least 1, and the actor is on a site | Current site. Lowest such stock id. Reserve 1. One debit leg of 1 | `Executing`. No path | Skip creation tick. Next tick `Completing`. Following tick kind 4, kind 3 reason 8, kind 7 `Done` | Save after the decision. Resume emits no second kind 6 before the new wake. Replay of the generated source-2 key is not a command |
| Rest | `home_site_id` names a site and a path exists, or the actor is already there | Home. No stock | `Travelling` with the stored path, or `Executing` if the path is empty | Empty kind 4, then kind 7 `Done`. No kind 3. Disable or cancel emits kind 7 and kind 5 and no kind 4 | Save on the path. Restore keeps the path and does not search |
| Work | Current site has a class-2 tool and a class-2 workstation, each with available quantity at least 1 | Current site. Lowest tool id and lowest workstation id. One bundle. No legs | `Executing` | Kind 4 with an empty leg list, two kind 3 reason 8, kind 7 `Done`. Quantities stay | Save after the bundle and before `Done`. The two reservation ids are in the blob |
| Haul | Inventory has a class-1 stock whose definition is not 1, available quantity at least 1, and home is reachable | That lowest stock. Debit 1. Credit the same definition and class to owner kind 3 at home | `Executing` at home, otherwise `Travelling` | One transaction. Not the three-hop M0-T07 script | Save with the unconsumed list empty and the path stored, when travel is required |
| Return home | Same site rule as rest | Home. No stock and no legs | Same stages as rest | Same terminal events as rest | Same path save as rest. Goal ordinal 5 distinguishes the trace |

Eat with hunger 1000 and the other channels 0 scores 1000000 for eat and 0 for the other four goals under the approved fixture weights. The decision events are kind 1, kind 7 `Executing`, kind 6. Work emits nothing on that tick.

## 6. Lifecycle order

Work, first match, for each current task in actor-id then task-id order:

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
| 9 | `Executing` | `Completing`, one kind 7, no legs |
| 10 | `Completing`, request list non-empty or `due_tick` equals the tick | Legs, one release per own reservation, `Done` |
| 11 | `Completing`, neither requested nor due | No event |

After task-owned work, residual edge travel runs for every actor with `locus` 3 and `task_id` 0, in ascending actor id. Each spends one edge cost with no event. Arrival places the actor on the destination site with no event.

Invalidation, before decision and work, walks actors and then tasks in ascending id. It releases active reservations once, writes the interruption, emits kind 3 then kind 7 then kind 5, and then sets `actor.task_id` to 0. Reason priority is disablement, then cancellation, then expiry. `Done` writes interruption 0. Mid-edge locus is not cleared by invalidation.

Completion events are kind 4, then kind 3 reason 8 in reservation-id order, then kind 7 `Done`. Kind 3 is omitted when the task has no reservations. Rest and return home therefore emit kind 4 with an empty leg list and kind 7.

C03-T01a initial tasks are 1 and 2, `created_tick` 0, `due_tick` 0, stage `Queued`, both actors at the stock site, empty paths. Tick 1 accepts reservation 3 for task 1 and rejects actor 2. Promotion sets task 1's `due_tick` to 3 because travel is 0 and the formula adds 2. The event order is kind 1 reservation 3, kind 7 task 1 `Reserved`, kind 2, kind 7 task 1 `Executing`. `next_event_id` ends at 5. This is proposed, not a golden hash.

M0-T06 uses the same promotion. Tasks 1 through 20 start `Queued` with `due_tick` 0. That is legal because they are `Queued`. Winners 1 through 5 then receive the work event into `Executing` on that same tick. Their new `due_tick` is the staged tick plus 2.

## 7. Navigation and residual edge travel

The approved graph is unchanged: sites 1, 2, and 9, edge 1 from 1 to 2 at cost 1, edge 2 from 2 to 1 at cost 1. The grid tie is start `(0,0)`, center `(1,1)` blocked, target `(2,2)`. The smaller direction sequence is north, north, east, east.

Site 1 to site 2 uses edge 1. The creation tick stores that path and does not spend it. The next work tick spends the one cost and arrives. Site 9 has no path. A decision rejects that goal with reason 5 before a task is created. A travelling task that already has an empty path and is standing on a site fails in work with kind 7 `Failed` and kind 5, plus kind 3 when it holds a reservation.

### Residual travel after `task_id` is cleared

| Concern | Rule |
| --- | --- |
| Phase ownership | Work phase, after task-owned work, actors with `locus` 3 and `task_id` 0, ascending actor id |
| Tick cost | One edge cost per residual actor per tick. Remaining cost is `edge.cost - traversed_cost` |
| Position changes | Progress increments `traversed_cost` by 1. Arrival sets `locus` 1, `site_id` to the edge destination, `edge_id` 0, `traversed_cost` 0 |
| Events | None for residual progress or residual arrival |
| Save and restore | Persist `locus`, `edge_id`, and `traversed_cost`. Restore does not search and does not finish the edge early |
| Enable and disable | Residual travel runs for enabled and disabled actors. Disablement does not discard remaining cost and does not invent a task |
| Decision | While `locus` is 3, goals that need a site are ineligible. No current task means residual travel is the only motion |

### Concrete acceptance case

Initial: actor 1 enabled, locus 3, edge 1, `traversed_cost` 0, `task_id` 50. Task 50 is `Travelling`, empty path, no reservation, `created_tick` 0, `due_tick` 9.

1. `drive` cancels task 50. Invalidation emits kind 7 `Cancelled` and kind 5. `task_id` becomes 0. Locus remains 3, edge 1, `traversed_cost` 0.
2. Save. Restore. Bytes still show locus 3 and `task_id` 0.
3. Empty `drive` for the next boundary. Residual work spends the one remaining cost. Actor arrives at site 2. No event. `task_id` stays 0.
4. Repeat the cancel, then `SetEnabled` 0 before the residual tick. The residual arrival still occurs. No task is created.

The R2 snap rule is withdrawn. OC-EDGE remains optional and not recommended.

## 8. What this version does not do

The five-phase order, the creation-tick skip, fixture 2 synthetic work, and H0 are unchanged. No gameplay checkpoint hash is published. The normative specification is not edited. Implementation is not authorized. G3 is not declared passed.
