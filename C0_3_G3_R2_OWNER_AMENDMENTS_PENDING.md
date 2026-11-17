# C0.3 G3 R2.1 Owner Amendments Pending

Version R2.1-final · 17 November 2026  
Status: APPROVED by the project owner on 17 November 2026, except OC-EDGE which is REJECTED. Normative copy: `docs/C0_3_IMPLEMENTATION_SPEC.md` version 1.1-approved. Record: `C0_3_OWNER_DECISION_RECORD.md` version 2.0. This historical proposal text is retained for traceability.

This replaces R2.1 of the same file. OD-01 through OD-16 remain approved as written in proposal v1.2. H0 is unchanged: `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c`.

R2.1-final corrections, relative to R2.1:

| Amendment | Correction |
| --- | --- |
| AM-01 / AM-03 / M0-T07 | Task 405 debits available stock 14, then fails on the invalid credit. It does not transform-out depleted stock 13 |
| AM-02 / AM-05 | The consumed registry stores the submitted `CompleteTask` payload id and the resolved task id. Same key with a different submitted id is `ConflictingIdentity` |
| AM-08 | After invalidation clears `task_id`, residual edge travel continues in the work phase until the destination site. No teleport. Remaining cost is spent one tick at a time |

Already resolved and not reopened: creation-tick skip, one shared `next_record_id`, five goal plans, fixture-2 isolation, approved graph sites and edges, OC-EDGE optional and not recommended.

Detail is in `C0_3_G3_R2_REMEDIATION_PROPOSAL.md`. Sequences are in `C0_3_G3_R2_ACCEPTANCE_RECONCILIATION.md`.

## AM-01 — Creation tick and M0-T07 timing

**Status: APPROVED.**

**Recommended resolution:** keep the OD-10 skip. `BeginTask` and a decision set `created_tick` to the staged tick. Work does not apply that task on that tick, even when a completion key is already stored and even when `due_tick` equals that tick. Each M0-T07 hop is two boundaries. The apply boundary is the next tick. Scripted `BeginTask` carries its own `due_tick`. The acceptance rows set that field to the apply tick, which is `created_tick + 1`. The autonomous formula `created_tick + travel + 2` is not applied to those scripted rows.

Task 405 is the later-leg failure case. After tick 8, stock 13 quantity is 0 and stock 14 holds definition 2 quantity 1. Task 405 therefore uses `recipe_id` 0 and planned legs whose first leg debits stock 14 and whose second leg credits owner id 0. Tick 10 fails on that later leg and rolls the whole boundary back. Ticks 1 through 9 remain.

**Alternative:** a same-tick exception for `CompleteTask`. That amends OD-10. It is not recommended.

**Bytes:** hop timing and the invalid-task legs change from the published acceptance table. H0 does not.

**Approval text:** "I approve AM-01. `created_tick` is the staged tick of creation. Work skips that tick. M0-T07 applies each hop on the following tick. Scripted tasks carry `due_tick` in the command. Task 405 fails on its later leg after a successful first leg against stock 14."

## AM-02 — Completion provenance

**Status: APPROVED.**

**Recommended resolution:** a task stores an unconsumed list of completion records, sorted by command key. Each record is:

- the key `(target_tick, actor_id, producer_id, sequence)`
- `u64 submitted_task_id`, the payload id as admitted (`0` for the same-phase form, or the concrete non-zero id)
- `u64 resolved_task_id`, the non-zero task that form named after resolution

`CompleteTask(0)` is legal only in the same command phase as a `BeginTask` for that actor. It stores `submitted_task_id` 0 and `resolved_task_id` equal to the task created earlier in that phase. `CompleteTask(401)` stores both fields as 401 when that is the actor's current non-terminal task. The command phase appends. It does not apply legs.

When a completion applies, the lowest key is written on the source-1 transaction. Every record in the list is appended to a world consumed registry and the list becomes empty. The lowest record has role 1. The others have role 2. Role 2 does not create a transaction.

If the task reaches `Cancelled` or `Failed` before legs apply, the same move happens with role 2 only. There is no role 1 and no transaction.

A later replay reads the registry in the checkpoint. It does not read a host log. Identical replay means the same key and the same `submitted_task_id`. That is a no-op for both role 1 and role 2. The same key with a different `submitted_task_id` is `ConflictingIdentity`, even when both forms would resolve to the same task id. Histories that accepted different keys, or the same key with different submitted ids, remain different checkpoints after `Done`.

The registry record is 41 bytes: four key fields (24), `submitted_task_id` (8), `resolved_task_id` (8), and `u8 role` (1). The count is checked against the unread body before allocation. `u64::MAX`, overflow, or a count that does not fit is `Corrupt`. Duplicate keys, two role-1 records for one task, or `resolved_task_id` 0 are `InvalidState`.

**Alternative:** reject a second unconsumed `CompleteTask` for the same task by failing the boundary. It is not recommended.

**Bytes:** the task record loses the `u8` flag and gains the unconsumed list. The body gains the consumed registry after applied transactions. Event kinds do not change. H0 does not.

**Approval text:** "I approve AM-02. Every accepted completion stores the submitted payload id and the resolved task id. The lowest key is the only source-1 transaction. Identical replay is a no-op. The same key with a different submitted id is `ConflictingIdentity`."

## AM-03 — Legs, holders, recipes, and reservation conservation

**Status: APPROVED.**

**Recommended resolution:** a leg is `u8 op`, `u64 stock_id`, `u64 definition_id`, `u8 class`, `u64 quantity`, `u8 owner_kind_after`, `u64 owner_id_after`. Quantity is at least 1. A task stores `u64 recipe_id`. Zero means the legs are the declaration. Nonzero binds one scenario recipe. Holders `(u64 id, u8 kind)`, sorted by id, validate owner kinds 2, 4, and 5. Kind 1 requires an actor. Kind 3 requires a site.

Available quantity for a task is on-hand minus active reservation quantities held by other tasks. The task's own active reservations are not subtracted from that available quantity. A debit or transform-out must fit in that available quantity after earlier legs of the same transaction. It must not reduce on-hand below the other tasks' active reservations.

If the task has active class-1 reservations on a stock, the planned debit plus transform-out of that stock must equal the sum of those reservation quantities. Any other total is `InvalidLeg` before release. Class-1 quantity changes only through those legs. Reason-8 release does not debit again.

Existing class-2 quantity is immutable. A class-2 debit, transform-out, or transform-in is `InvalidLeg`. Releasing a class-2 reservation does not change quantity.

Legs run on a staged copy. A later-leg failure means an earlier leg in that copy would have succeeded, and a subsequent leg fails. The whole boundary still returns `InvalidLeg`, emits nothing, consumes no transaction id, releases nothing, and restores the pre-call bytes. On success, each active reservation of that task is released once, reason 8, ascending id. A second release emits nothing.

M0-T07 task 405 is that later-leg case: first leg debits available stock 14; second leg credits owner id 0 and fails.

**Alternative:** pre-create every output stock and forbid `stock_id` 0. It is not recommended.

**Bytes:** leg width grows. Holders are inserted after sites. The task gains `recipe_id`. H0 does not.

**Approval text:** "I approve AM-03. Legs carry definition and class. A task cannot debit another task's reservation. Own class-1 reservations match the debit and are released once. A later-leg failure rolls the whole boundary back."

## AM-04 — One record cursor, disjoint ids

**Status: APPROVED.**

**Recommended resolution:** `next_record_id` is the only cursor for tasks and reservations. It is the next unused id and is strictly greater than every task id and every reservation id. Task ids and reservation ids are one namespace. Restore rejects a checkpoint in which any task id equals any reservation id, even when the cursor is above both. Stocks, transactions, and actors keep their own cursors.

Explicit acceptance cursors:

| Fixture | Installed task ids | `next_record_id` | First issued reservation |
| --- | --- | --- | --- |
| C03-T01a | 1 and 2 | 3 | 3 |
| M0-T06 | 1 through 20, one per actor | 21 | 21, then 22 through 40 for the twenty winning bundle rows |
| M0-T07 | none at construction | 401 | none. Scripted tasks use 401 through 405 |

**Alternative:** a second cursor. That contradicts OD-01. It is not recommended.

**Bytes:** reservation ids in the published event examples change to values this cursor can issue. H0 does not.

**Approval text:** "I approve AM-04. Tasks and reservations share `next_record_id` and cannot reuse an id. C03-T01a starts at 3. M0-T06 starts at 21."

## AM-05 — Fixture-3 ingress, first match

**Status: APPROVED.** Fixture 1 and fixture 2 keep their frozen precedence. This table is fixture 3 `drive` only. `step` on fixture 3 returns `UseDrive` and does not enter the table.

Rules are tested in order. The first match is the result. An error rolls the whole boundary back. A no-op does not emit and does not admit the command. A boundary whose commands are all no-ops still advances the tick.

| # | Condition | Result |
| --- | --- | --- |
| 1 | `submission_tick` is not the current tick | `WrongBoundary` |
| 2 | Any producer is not 1 | `InvalidProducer`. Producer 0 never replays a generated transaction |
| 3 | Any actor id is unknown | `UnknownActor` |
| 4 | The same key appears twice in the batch, or the key is already pending | `ConflictingIdentity` |
| 5 | The key is in the consumed registry and the submitted `CompleteTask` id equals the stored `submitted_task_id` | No-op, including when the target tick is old. Role 1 and role 2 both match this way |
| 6 | The key is in the consumed registry and the submitted `CompleteTask` id differs from the stored `submitted_task_id` | `ConflictingIdentity`, even if both ids would resolve to the same task |
| 7 | The payload is a cancel, the reservation exists, and the command actor is not the stored claimant | `InvalidPayload` |
| 8 | The payload is a cancel, the reservation exists, the actor is the claimant, and that task is already terminal | No-op, including when the target tick is old |
| 9 | `target_tick` is less than or equal to the current tick, and no earlier rule matched | `LateTarget` |
| 10 | Bad tag, unsorted or repeated bundle, non-positive quantity, an empty reservation interval, or `BeginTask` while the actor `locus` is 3 | `InvalidPayload` |
| 11 | `CompleteTask` does not name the actor's current non-terminal task, and it is not the same-phase zero form that resolves to the task created earlier in this phase | `InvalidPayload` |
| 12 | Non-zero cancel whose reservation task is non-terminal and is not `actor.task_id` | `InvalidPayload` |
| 13 | Unknown cancel id, or cancel 0 when the actor has no current task | No-op |
| 14 | Otherwise | Admit |

Pending keys and consumed keys use the same key fields. A pending duplicate is a conflict. A consumed record matches only when the submitted payload id matches. `CompleteTask(0)` and `CompleteTask(401)` are different submitted payloads. Generated source-2 transactions are recognized only by the work phase finding that transaction already applied. They are not commands.

Restore, after a complete body and a matching trailer: `Truncated`, `UnsupportedFormat`, `UnsupportedRevision`, `ChecksumMismatch`, trailing bytes `Corrupt`, then the semantic matrix. Count and grid bounds stay `Corrupt` during the parse.

**Alternative:** treat every old target as `LateTarget`, including a matching consumed key. Identical replay would then be an error. It is not recommended.

**Bytes:** no new event kind. H0 does not.

**Approval text:** "I approve AM-05. A consumed completion key replays as a no-op only when the submitted payload id matches. A different submitted id is `ConflictingIdentity`. Terminal cancel uses the stored claimant. Producer 0 is not a command."

## AM-06 — Five goal plans

**Status: APPROVED.**

**Recommended resolution:** the five plans in the remediation proposal §5. Ineligibility order is disabled, relationship, existing non-terminal task, missing home, not on a site when a search would be required, unreachable, insufficient stock. Selection uses the lowest legal stock id and the lowest legal site id. `due_tick = created_tick + travel_ticks + 2`. Reservation expiry is `due_tick + 1`. Enabling an actor from 0 to 1 suppresses that actor's decision on that tick and, when `next_wake_tick` equals the tick, moves the wake forward by the wake interval.

While `locus` is 3, every goal that requires a site or a new path is ineligible under the existing "not on a site" reason. Residual edge travel in AM-08 is not a goal.

**Alternative:** script every goal and leave the planner blank. That drops the needs exit. It is not recommended.

**Bytes:** traces and task fields become determined. No new event kind. H0 does not.

**Approval text:** "I approve AM-06. The five goal plans, the reason order, and same-tick enable rescheduling are fixture rules. Weights and the wake interval stay the approved fixture configuration."

## AM-07 — Lifecycle

**Status: APPROVED.**

**Recommended resolution:** work considers the creation-tick skip before any due-tick or request rule. `Queued` does not advance and is not an overdue failure. `Reserved`, `Travelling`, `Executing`, and `Completing` with `due_tick` below the staged tick fail the boundary as `InvalidState`. `Completing` applies only when invalidation left it `Completing` and the request list is non-empty or `due_tick` equals the staged tick. `Done` stores interruption 0, clears `actor.task_id`, and moves the request list into the consumed registry. Invalidation walks actors, then tasks, in ascending id, and then clears `actor.task_id`. Same-boundary work events are kept.

The tick-9 and tick-10 M0-T07 rows in the acceptance reconciliation are the executable form of this rule. Tick 10 fails because the second planned leg is invalid after a successful first-leg check against stock 14.

**Alternative:** suppress work on a reservation tick. That contradicts the phase list. It is not recommended.

**Bytes:** C03-T01a and M0-T06 include the work-stage events. H0 does not.

**Approval text:** "I approve AM-07. `Queued` may be overdue without failing the boundary. Other non-terminal stages may not. M0-T07 tick 10 is a later-leg `InvalidLeg` from the tick-9 task, and the creation tick does not apply legs."

## AM-08 — Navigation without a mid-edge teleport

**Status: APPROVED.**

**Recommended resolution:** keep sites 1, 2, and 9 and edges 1 and 2. Do not add edges. The equal-cost demonstration is the approved 3 by 3 grid: north, north, east, east.

An actor on an edge keeps `edge_id` and `traversed_cost`. Restore does not search and does not change that locus. No commit moves the actor to the source or the destination in order to start a search.

While a non-terminal travelling task owns the actor, work spends one cost on the current edge or stored path as already specified.

When invalidation or completion clears `actor.task_id` while `locus` is still 3, residual edge travel begins. It is owned by the work phase, not by decision, command, or a synthetic task. Actors with `locus` 3 and `task_id` 0 are visited in ascending actor id after task-owned work. Each such actor spends one cost on the current edge. No event is emitted for residual progress or for residual arrival. When `traversed_cost` reaches the edge cost, the actor is placed on the destination site, `locus` becomes 1, `site_id` becomes that destination, `edge_id` becomes 0, and `traversed_cost` becomes 0. Remaining tick cost before that arrival is `cost - traversed_cost`. The remaining cost is never discarded and never spent in one commit as a teleport.

Residual travel runs whether the actor is enabled or disabled. Disablement does not strand the actor mid-edge and does not create a task. While `locus` is 3, decision does not select a goal that requires a site. A new `BeginTask` while `locus` is 3 is `InvalidPayload` under AM-05 rule 10. Save and restore keep `locus`, `edge_id`, and `traversed_cost` and do not search.

`traversed_cost` greater than 0 cannot appear on the approved cost-1 edges. It is not an acceptance case unless the owner approves OC-EDGE.

**Optional choice OC-EDGE, not part of the recommended approval:** add edge 5, site 1 to site 2, cost 2, so a save can hold `traversed_cost` 1. Edge 4 is not requested. Rejecting OC-EDGE does not reject AM-08.

**Withdrawn:** the R2 snap rule that moved a mid-edge actor to an endpoint when `traversed_cost * 2 >= cost`.

**Alternative:** freeze mid-edge actors until a later scripted task resumes them. That strands disabled actors and invents a resume command. It is not recommended.

**Bytes:** the approved edge list is unchanged under the recommended resolution. H0 does not.

**Approval text:** "I approve AM-08. Mid-edge progress is preserved. After `task_id` is cleared, residual work-phase travel spends the remaining edge cost one tick at a time until the destination site. The approved graph is unchanged. I do not approve OC-EDGE."

If the owner wants the cost-2 save, a separate sentence is required: "I approve OC-EDGE. Edge 5, site 1 to site 2, cost 2, is added. Edge 1 remains the shortest path. Site 9 remains unreachable."

## Owner choices (recorded)

| Choice | Status |
| --- | --- |
| AM-01 through AM-08 | APPROVED |
| OC-EDGE | REJECTED |
| Same-tick completion | Rejected by retention of the creation-tick skip |
| Second record cursor | Rejected by retention of one `next_record_id` |
| Snap-to-endpoint replan | Withdrawn and not approved |

Normative documents are revised. Implementation remains unauthorized. This file does not declare G3 PASS.
