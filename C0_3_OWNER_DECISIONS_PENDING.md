# C0.3 Owner Decisions Pending

Version 1.2-proposal · 17 November 2026  
Status: waiting for the project owner. Every recommendation in this file is **NOT APPROVED**.

Nothing here modifies the normative C0.3 draft, the C0.2 archive, or a golden vector. Accepting a row is a later, explicit owner action. Refinement 1.2 closes the transfer order, event sequences, task cardinality, and completion-phase gaps. It does not approve the rows and it does not narrow the exit.

## 1. Existing approved requirements

These already bind C0.3. The architect does not re-approve them.

| Topic | Binding text |
| --- | --- |
| Exit | README: C03-T01 and the applicable M0-T06 through M0-T08 fixtures |
| Deliverable | README and Technical Architecture §2: needs, jobs, inventory, resource transactions; Technical Architecture §2 and §10 also name initial navigation |
| Ledger | Core §7 and ADR-013: one owner; reservation is not a second owner; availability is on-hand minus active reservations; interval `[acquired tick, expiry tick)`; same-tick disable, cancel, and expiry invalidate before completion; repeated terminal release is a no-op; one stable identity applies each validated leg once; a failed later leg leaves earlier legs unapplied |
| Keys before code | Core §7 and C03-T01: freeze command priority, reservation keys, transaction IDs, and diagnostics before production behavior |
| Jobs | Core §6: eat, rest, work, haul, return home; stages `Queued → Reserved → Travelling → Executing → Completing → Done`; cancellation, interruption, and failure are explicit branches; a task holds reservations; release once; task and progress survive save and load |
| Needs | Core §6: score on scheduled decision ticks using needs, role, reachability, inventory, reservations, and relationship constraints; DecisionTrace holds candidate names, terms, scores, rejection reasons, winner, task, and next evaluation tick |
| Fairness limit | ADR-021, Core §4, GDD §3: the C0.1 key is not global gameplay fairness. C03-T01 still requires its own ordered fixture |
| Navigation rule | Core §8: the simulation owns position; successor order and equal-cost ties are total when a navigation fixture exists |
| Planner limit | GDD §5: no GOAP framework in C0 |
| Persistence | Core §5 and §11; C0.2 spec: new future-affecting fields need a new schema. Revision 1 of `SH:C02:STATE` stays closed. M0-T11 owns the first migration |
| Fixture 2 | C0.2 spec §8 and the baseline manifest: a disabled actor still completes the installed synthetic task. Published hashes stay |
| Invariant 4 | Core §12: a disabled actor cannot continue an invalid production job. Owning gate is C0.3, not a retrofit of fixture 2 |
| Encoding discipline | Little-endian unsigned integers, exact ASCII tags, counted lists, sorted tables, SHA-256 body trailer, as used by `SH:C02:STATE` |

## 2. Deferred requirements already authorized

These stay out of C0.3 because an approved document names the later gate. Confirming them is not a scope cut.

| Item | Authority |
| --- | --- |
| Production activation without an admitted mode command | LOD §5, ADR-009, M0-T05 |
| Travel approximation | LOD §5, ADR-009 |
| Anonymous cohorts | ADR-010 |
| Day length and the 30-day run | Core §3, ADR-024, M0 §1, M0-T01 |
| Migrating an older save into a newer one | M0-T11, C0.2 spec |
| Typed JSON content | Technical Architecture §3, M0-T09 |
| Bevy and host cadence | V0, I0 |
| R2 and performance budgets | Core §1, M0-T14 |
| Socialize and react-to-threat | Core §6 optional extensions |
| Flow fields and nearest-neighbour acceleration | Core §8, after profiling |
| Combat priorities | GDD §3, M1 |
| I0 producer trust and bridge permissions | I0-T01, I0-T02. Fixture 3 names its own producer rule below |

## 3. Shared encoding rules

These rules apply to every byte layout in this section. They are part of the recommendations and are **NOT APPROVED**.

All integers are unsigned little-endian except `i64` scores, weights, and role bonuses. Tags are exact ASCII with no length prefix. Lists begin with a `u64` count. Records have no padding. Tables are sorted by the key stated in each row. A `u64` id field that is zero means "none" where the row says so. The blob is `body || SHA-256(body)`.

Fixture 1 and fixture 2 encodings are not extended. `SH:C01:STATE` and `SH:C02:STATE` remain closed. The new decoder returns `UnsupportedFormat` for both. The C0.2 decoder returns `UnsupportedFormat` for `SH:C03:STATE`.

Reason bytes used by events and traces:

| Byte | Meaning |
| --- | --- |
| 0 | None |
| 1 | Disablement |
| 2 | Cancellation |
| 3 | Expiry |
| 4 | Lost contention |
| 5 | Unreachable |
| 6 | Insufficient availability |
| 7 | Invalid leg |
| 8 | Released because the task completed |
| 9 | Actor already has a non-terminal task |
| 10 | Goal has no legal target |
| 11 | Relationship constraint |

## 4. Architect recommendations

### OD-01 — Canonical checkpoint

**Recommendation, NOT APPROVED:** fixture id 3. Tag `SH:C03:STATE`, `u16` revision 1. No migrator.

Body order:

| Order | Bytes |
| --- | --- |
| 1 | ASCII `SH:C03:STATE`, `u16 = 1` |
| 2 | `u32` fixture id 3, then 32-byte content hash |
| 3 | `u64` tick, `u64` seed |
| 4 | `u64 next_actor_id`, `u64 next_record_id`, `u64 next_transaction_id`, `u64 next_stock_id` |
| 5 | Scenario block, whose SHA-256 is the content hash in field 2 |
| 6 | Actors, count then ascending `u32` id |
| 7 | Sites, count then ascending `u64` id |
| 8 | Edges, count then ascending `(from_site, to_site, edge_id)` |
| 9 | One grid descriptor |
| 10 | Stocks, count then ascending `u64` id |
| 11 | Reservations, count then ascending `u64` id |
| 12 | Tasks, count then ascending `u64` id |
| 13 | Transactions, count then ascending `u64` id. Only successfully applied transactions are stored |
| 14 | Pending commands, count then command-key order |
| 15 | Decision traces, count then ascending actor id. At most one trace per actor |
| 16 | `u64 next_event_id`, 32-byte event digest |

Scenario block, in this order: `u64 wake_interval`; five goals in ordinal order, each with five `i64` channel weights; five `i64` relationship minimums, one per goal; `u64 recipe_count`; each recipe is `u64 recipe_id`, `u64 input_definition`, `u64 input_quantity`, `u64 output_definition`, `u64 output_quantity`, `u8 output_owner_kind`, `u64 output_owner_id`.

Actor record:

| Field | Encoding |
| --- | --- |
| Identity | `u32 actor_id`, `u8 enabled` (0 or 1), `u8 mode` (1 active, 2 scheduled) |
| Needs schedule | `u64 next_wake_tick`, five `u32` channels in `0..=1000` (hunger, fatigue, duty, haul, homesickness) |
| Role | five `i64` bonuses, one per goal ordinal |
| Relationship | one `i64 relationship_term` |
| Home | `u64 home_site_id` |
| Current task | `u64 task_id`, or 0. This is the actor's only non-terminal task |
| Position | `u8 locus`, `u64 site_id`, `u32 grid_x`, `u32 grid_y`, `u64 edge_id`, `u64 traversed_cost` |

Locus 1 is a site. Locus 2 is a grid cell of that site. Locus 3 is on a route edge, with `traversed_cost` ticks already spent on that edge. The unused coordinates are zero. Any other locus, a non-zero unused field, or a `traversed_cost` greater than the edge cost is `Corrupt`.

Site record: `u64 site_id`, `u8 has_grid` (0 or 1). Edges: `u64 from_site`, `u64 to_site`, `u64 edge_id`, `u64 cost`. Cost is a positive `u64`. A zero cost is `Corrupt`. Grid descriptor: `u64 site_id` or 0 when absent, `u32 width`, `u32 height`, then `width * height` bytes in row-major order, 1 passable and 0 blocked. Width and height are both zero when `site_id` is zero. If either dimension is above 4096, either is zero while the other is not, their product overflows `u32`, or the product is larger than the unread body, the blob is `Corrupt`. No grid buffer is allocated from an unchecked product.

Every `u64` count is checked before allocation. If `count * minimum_record_size` overflows, or the product is larger than the unread body, the blob is `Corrupt`. A count of `u64::MAX` is `Corrupt`. These rules follow the C0.2 bounded-parse class and do not change the C0.2 decoder.

Stock record: `u64 id`, `u64 definition_id`, `u8 class` (1 consumable, 2 durable capacity), `u64 quantity`, `u8 owner_kind`, `u64 owner_id`. Owner kinds are 1 actor inventory, 2 container, 3 site stockpile, 4 in-transit cargo, 5 explicit reserve. Quantity is the on-hand amount. Availability is that quantity minus the sum of active reservation quantities for the same stock id.

Reservation record: `u64 id`, `u64 stock_id`, `u32 claimant`, `u64 quantity`, `u64 acquired_tick`, `u64 expiry_tick`, `u64 task_id`, `u8 state` (1 active, 2 released).

Task record:

| Field | Encoding |
| --- | --- |
| Identity | `u64 id`, `u32 actor_id`, `u8 goal`, `u8 stage`, `u8 interruption` |
| Target | `u8 target_locus`, `u64 target_site`, `u32 target_x`, `u32 target_y` |
| Progress | `u64 progress_ticks`, `u64 created_tick`, `u64 due_tick`, `u8 completion_requested` (0 or 1) |
| Path | `u64 path_index`, `u64 step_count`, then that many steps |
| Holds | `u64 reservation_count`, then that many `u64` reservation ids, ascending |
| Planned legs | `u64 planned_leg_count`, then that many legs in the transaction-leg encoding |

Stages: 1 Queued, 2 Reserved, 3 Travelling, 4 Executing, 5 Completing, 6 Done, 7 Cancelled, 8 Failed. Goals: 1 eat, 2 rest, 3 work, 4 haul, 5 return home.

An actor owns at most one non-terminal task. `actor.task_id` is that task, or 0 when every task of that actor is terminal or absent. A second Queued, Reserved, Travelling, Executing, or Completing task for the same actor is `InvalidState` on load and is rejected if a command would create it. Terminal tasks stay in the table so their transaction ids remain stable. A reservation command whose `task_id` is not `actor.task_id` fails preflight and does not mutate. A non-current task therefore cannot acquire a reservation. A path step is `u8 kind`. Kind 1 is a route edge and is followed by `u64 edge_id`. Kind 2 is a grid step and is followed by `u8 direction` (1 north, 2 east, 3 south, 4 west). `path_index` is the step currently being travelled. `progress_ticks` counts work-phase ticks already applied to this task.

Transaction record: `u64 id`, `u64 task_id`, `u8 source` (1 command, 2 generated), the completion command key (`u64 target_tick`, `u32 actor_id`, `u32 producer_id`, `u64 sequence`), `u8 applied` (always 1 in the stored list), `u64 leg_count`, then legs. A leg is `u8 op`, `u64 stock_id`, `u64 quantity`, `u8 owner_kind_after`, `u64 owner_id_after`. Ops are 1 debit, 2 credit, 3 transform out, 4 transform in. A failed transaction is not appended and consumes no transaction id.

Pending command: `u64 target_tick`, `u32 actor_id`, `u32 producer_id`, `u64 sequence`, then one payload.

| Tag | Payload |
| --- | --- |
| 1 `SetMode` | `u8 mode` |
| 2 `CancelReservation` | `u64 reservation_id` |
| 3 `SetEnabled` | `u8 enabled` |
| 4 `ReserveStock` | `u64 stock_id`, `u64 quantity`, `u64 expiry_tick`, `u64 task_id` |
| 5 `ReserveBundle` | `u64 task_id`, `u64 expiry_tick`, `u64 pair_count`, then pairs `(u64 stock_id, u64 quantity)` sorted by stock id |
| 6 `CompleteTask` | `u64 task_id`. Zero means the task created earlier in this same command phase for this actor |
| 7 `BeginTask` | `u8 goal`, `u8 stage` (`Queued` or `Completing` only), target locus and coordinates, `u64 due_tick`, then the planned-leg list |

`BeginTask` is legal only when `actor.task_id` is 0. It allocates the next record id, stores that id in `actor.task_id`, and leaves `completion_requested` at 0. A disabled subject does not receive a task. The command phase emits one kind 2, stock 0, quantity 0, reason 1, and the boundary still commits.

Decision trace, one per actor: `u32 actor_id`, `u64 evaluated_tick`, `u64 next_evaluation_tick`, `u8 winner_goal` (0 if none), `u64 task_id` (0 if none), then five candidates in goal order. Each candidate is `u8 goal`, `u8 eligible`, `u8 rejection_reason`, `i64 score`, five `i64` channel terms, and one `i64` relationship term.

Alternative B, `SH:C02:STATE` revision 2, remains **NOT APPROVED** and is not recommended. Alternative C, new fields inside revision 1, is not offered.

### OD-02 — Events

**Recommendation, NOT APPROVED:** `H0 = SHA-256("SH:C03:EVENT-INIT:v1")`, then `SHA-256("SH:C03:EVENT:v1" || previous || canonical_bytes)`. Canonical bytes are `u64 id`, `u64 tick`, `u32 actor_id`, `u8 kind`, then the payload.

| Kind | Payload |
| --- | --- |
| 1 ReservationAccepted | `u64 reservation_id`, `u64 stock_id`, `u64 quantity` |
| 2 ReservationRejected | `u64 stock_id`, `u64 quantity`, `u8 reason` |
| 3 ReservationReleased | `u64 reservation_id`, `u8 reason` |
| 4 LegsApplied | `u64 transaction_id`, `u64 task_id` |
| 5 TaskFailed | `u64 task_id`, `u8 reason` |
| 6 DecisionMade | `u8 winner_goal`, `u64 task_id`, `u64 next_wake_tick`, five `i64` scores in goal order |
| 7 TaskStage | `u64 task_id`, `u8 stage` |

Kind 7 is emitted for every stage change, including creation into `Queued` or `Completing` and every later transition. A silent no-op emits nothing. There is one legal order for each commit:

| Commit | Event order |
| --- | --- |
| Reservation accepted and the stage changes | Kind 1 in ascending new reservation id, then one kind 7 |
| Reservation or bundle rejected | One kind 2 |
| Invalidation | Kind 3 in ascending reservation id, then kind 7 for `Failed` or `Cancelled`, then kind 5. Zero reservations omit kind 3 and still emit kind 7 and kind 5 |
| Decision with a winner | Kind 1 in stock-id order, then kind 7, then kind 6 |
| Decision with no eligible goal | Kind 6 only |
| Work changes stage and does not finish | One kind 7 |
| Work completes | Kind 4, then kind 3 reason 8 in ascending reservation id, then kind 7 `Done`. Zero reservations omit kind 3 |

A new fixture starts `next_event_id` at 1 and the digest at `H0`. Each emitted event uses the current cursor, then the cursor increases by 1, then the digest becomes `SHA-256("SH:C03:EVENT:v1" || previous || those canonical bytes)`. A rolled-back boundary changes neither cursor nor digest. No test accepts a second order. Fixture 1 and fixture 2 keep their digest domains. Production events do not enter the C0.1 `Outcome` enum.

### OD-03 — Contention order

**Recommendation, NOT APPROVED:** due commands sort by `(target tick, actor ID, producer ID, sequence)`, ascending. This order is the C03 fixture order, not global gameplay fairness.

`ReserveStock` and each pair inside `ReserveBundle` are all-or-nothing. Walk due reservation commands in key order. A request is accepted only when every named stock has available quantity at least the requested amount. Acceptance allocates the next record ids, emits kind 1 once per new reservation, and reduces availability immediately for later commands in the same phase. A request that does not fit emits one kind 2 with reason 4, or reason 6 when the stock's on-hand quantity itself is below the request. It allocates nothing. Both submission orders of the same keys produce the same winners.

### OD-04 — Reservations and bundles

**Recommendation, NOT APPROVED:** a task may hold zero or more reservations. The task stores their ids. Core §6 names held reservations as a collection.

A bundle is one `ReserveBundle` command. The pairs are checked together. Either every pair becomes an active reservation, with ids allocated in ascending stock-id order, or none do. There is no partial bundle.

Stock class 1, consumable, is food and recipe input. Completion debits on-hand quantity and then releases the reservation with reason 8. Stock class 2, durable capacity, is a bed, tool, or workstation. Completion does not change on-hand quantity. It releases the reservation with reason 8 so the capacity can be reserved again. A reservation never changes owner and never creates quantity.

Cancellation, disablement, expiry, unreachable failure, and an invalid goal release every still-active reservation of that task, in ascending reservation id, one kind 3 event each, the same reason byte, and each release happens once. A second release of any of those ids is the OD-08 no-op.

Quantity zero, an expiry less than or equal to the acquired tick, a pair list that is not sorted or that repeats a stock id, and a sum that would overflow `u64` fail in preflight. The boundary does not mutate.

### OD-05 — Transactions

**Recommendation, NOT APPROVED:** legs are copied from the task's planned-leg list. The stored transaction id is `next_transaction_id`, allocated only after every leg has been validated. Legs apply in list order inside the work-phase commit. A credit or transform-in leg with `stock_id` 0 allocates `next_stock_id`. A later invalid leg rolls the whole boundary back: no transaction is stored, no event is emitted, the tick does not advance, and every earlier committed transaction remains.

Source 1 means an admitted `CompleteTask`. Its stored key uses that command's target tick, actor id, producer 1, and sequence. Source 2 means the work phase completed a task whose `due_tick` equals the staged tick and whose `completion_requested` was still 0 after invalidation. The stored key uses the staged tick, the task's actor id, producer 0, and sequence 0. Producer 0 is provenance only. A command that carries producer 0 fails admission as an invalid producer and does not mutate. Replay of either stored key is the OD-08 no-op.

### OD-06 — Who may cancel

**Recommendation, NOT APPROVED:** fixture 3 admits only `producer_id = 1`. Every other producer fails preflight with the existing invalid-producer class and no mutation. This rule is fixture data. It does not consult I0.

A non-zero `reservation_id` is valid only when the command's actor id equals that reservation's claimant and the reservation's task is `actor.task_id`. Any other actor id or task id fails preflight and does not mutate. Reservation id 0 marks `actor.task_id` for cancellation. That form is the cancel input for Rest and Return Home, which hold no reservation. Producer 1 may submit either form.

The command phase records the cancel. It does not release and it does not clear `completion_requested`. The invalidation phase of that same tick releases every active reservation of that task, in ascending id, reason 2, sets the stage to `Cancelled`, clears `completion_requested`, and emits kind 3 events, kind 7, and kind 5. An unknown id, or a target whose task is already terminal, is a no-op.

### OD-07 — Who may disable or enable

**Recommendation, NOT APPROVED:** `SetEnabled` uses the same producer rule. The command's actor id is the subject. Producer 1 may set that subject to 0 or 1. The subject may be disabled when the command is applied. No other producer is accepted. An enabled byte other than 0 or 1 fails preflight.

The command phase writes the bit. If the bit becomes 0, invalidation then fails that actor's current non-terminal task, including a task with no reservations, releases every active reservation with reason 1, clears `completion_requested`, and emits the invalidation event order. Work does not apply legs. If the bit becomes 1, no reservation is released and no task is created. A following decision can run only on a later wake.

Fixture 1 and fixture 2 reject this payload. Their enabled flags stay immutable after construction.

### OD-08 — Exactly-once release

**Recommendation, NOT APPROVED:** a repeated terminal release, a second cancel of an already terminal task, and a replayed applied completion key change no stock, owner, reservation, task, transaction, digest, or event cursor. They emit no event. A valid boundary that contains only that no-op still advances the tick.

### OD-09 — Needs evaluation

**Recommendation, NOT APPROVED:** no GOAP search and no floating point. The five channels and the weight table are the whole scorer.

Evaluation runs in the decision phase, after invalidation and before work. Actors are considered in ascending actor id. An actor is evaluated only when `next_wake_tick` equals the staged tick. Any other actor is left unchanged, including after restore.

For each goal in ordinal order 1 through 5, the five channel terms are copied into the trace and the relationship term is copied beside them. The score is the `i64` dot product of the channels with that goal's weights, plus that goal's role bonus. The relationship term is not added to the score. Each multiplication and addition uses checked `i64` arithmetic. An overflow fails the boundary with `CapacityExceeded`, writes no trace, emits no event, and does not advance the tick. Eligibility is then applied, and an ineligible goal keeps its computed score and gains a rejection reason:

| Condition | Reason |
| --- | --- |
| Actor is disabled | 1, for every goal |
| Actor's current task is non-terminal | 9, for every goal. The current task is kept |
| Eat, and no consumable food has available quantity at least 1 in the actor inventory | 6 |
| Rest or return home, and `home_site_id` is 0 | 10 |
| Work, and a tool plus a workstation cannot both be reserved for quantity 1 | 6 |
| Haul, and the actor has no cargo stock with available quantity at least 1 | 6 |
| The goal's target has no stored or computable path, and the actor is not already there | 5 |
| `relationship_term` is less than that goal's relationship minimum | 11 |

The winner is the eligible goal with the highest score. A tie keeps the lower goal ordinal. The decision commit reserves that goal's bundle, if it has one, by the same all-or-nothing rule as OD-04. Later actors in the same phase see the reduced availability.

If every goal is ineligible, `winner_goal` and `task_id` are 0, no reservation is created, and kind 6 is still emitted.

A winner creates one task with `created_tick` equal to the staged tick. The bundle, when required and accepted, moves the task to `Reserved` in that same commit. If a path is required, it is stored and the stage becomes `Travelling`. If the actor is already at the target, the stage becomes `Executing`. `next_wake_tick` becomes the staged tick plus `wake_interval` whether or not there was a winner. The trace replaces the actor's previous trace.

The task is not eligible for the work phase of its `created_tick`. It becomes eligible on the next tick. That prevents the decision and the first movement from happening in one phase, and it gives invalidation a later tick on which it can still precede completion.

Proposed numbers, **NOT APPROVED:** `wake_interval = 4`. Eat weights `(1000,0,0,0,0)`, rest `(0,1000,0,0,0)`, work `(0,0,1000,0,0)`, haul `(0,0,0,1000,0)`, return home `(0,0,0,0,1000)`. Role bonuses are 0. Every relationship minimum is the minimum `i64` value, and every actor's `relationship_term` is 0. That is the neutral gate: no goal is rejected for reason 11, and the term is still stored in the trace. Channels do not decay. The fixture sets the channel values, so the winner is determined without a decay table. The owner may replace the table. An implementer may not invent a different one.

Rest and Return Home reserve nothing. Their completion still passes invalidation. Disablement or a zero-id cancel on that tick emits kind 7 and kind 5 and does not emit kind 4. A completion that survives invalidation emits kind 4 with an empty leg list, then kind 7 `Done`.

Restore uses the persisted `next_wake_tick` and the persisted trace. If the restored tick is earlier than `next_wake_tick`, the decision phase does not emit another kind 6 for that actor. The same channels, weights, and tick therefore cannot produce a second trace before the stored wake.

### OD-10 — Job timing

**Recommendation, NOT APPROVED:** work-phase tasks are ordered by `(actor id, task id)`. A task with `created_tick` equal to the staged tick is skipped. Each other non-terminal task takes one transition:

| Stage at the start of the transition | Result at the end |
| --- | --- |
| `Reserved`, path empty, actor at target | `Executing` |
| `Reserved`, path non-empty | `Travelling` |
| `Travelling`, and one more tick remains on the current step | `traversed_cost` or grid progress increases by 1. Stage stays `Travelling` |
| `Travelling`, and the current step completes with steps remaining | Position becomes the step destination, `path_index` increases, step progress returns to 0 |
| `Travelling`, and the last step completes | Position is the target and the stage is `Executing` |
| `Executing` | Stage becomes `Completing`. No legs apply on this tick. `completion_requested` is left unchanged |
| `Completing`, and invalidation left the stage unchanged, and either `completion_requested` is 1 or `due_tick` equals the staged tick | Apply the planned legs, then release reservations with reason 8, then stage `Done`, then clear `actor.task_id` and `completion_requested` |
| `Completing`, and invalidation already changed the stage | No legs. The invalidation events were already emitted |

`progress_ticks` increases by 1 on every transition above. Eat debits one food unit. Rest and return home have no planned legs and no reservations. Work releases one tool and one workstation without debiting them. A scripted haul uses one planned transfer per completion, so Actor → Store, Store → Cargo, and Cargo → Site are three successive current tasks, never one actor holding three tasks. A terminal task clears `actor.task_id` only after that commit's events.

`CompleteTask` never applies legs in admission or in the command phase. Admission rejects producer values other than 1, a task id that is neither 0 nor `actor.task_id`, and a duplicate key in the same batch. The batch then does not mutate. The command phase sets `completion_requested` to 1. Invalidation still runs before work. If invalidation terminates the task, work sees a terminal stage and applies nothing. A second boundary that repeats an applied completion key is the OD-08 no-op. A failed later leg rolls the boundary back under OD-05.

Interruption is disablement or cancellation. Both are explicit branches to `Failed` or `Cancelled` and use the release rule in OD-04.

### OD-11 — Navigation

**Recommendation, NOT APPROVED:** the actor record in OD-01 is the only position. A site locus is a route-graph node. A grid locus is a cell of that site's grid. An edge locus is travel between two sites. An actor is in one locus at a time.

Route search is used when the target locus is a site. Grid search is used when the target is a cell of the actor's current site. The two searches are not mixed in one path. No pathfinding crate is added.

Route successors from a site are the outgoing edges sorted by destination site id, then edge id. Grid successors are north, east, south, west, skipping blocked cells and cells outside the grid. Search minimizes the sum of integer costs. A route edge costs its `cost` field. A grid step costs 1.

That cost is both the selection weight and the travel duration. An edge of cost 2 adds 2 to the path total and takes 2 work-phase ticks to traverse. `traversed_cost` advances by 1 per tick. Path sums use checked `u64` addition. An overflow fails the boundary with `CapacityExceeded`, stores no path, emits no event, and does not advance the tick. It is not an unreachable result. Equal total cost keeps the lexicographically smaller path. Each route step compares as `(destination site id, edge id)`. Each grid step compares as its direction byte. Expansion follows the successor order above, and an equal-cost candidate replaces the stored path only when its sequence is smaller.

A missing path fails the task with reason 5, releases every active reservation once, and emits the invalidation order: kind 3 events, kind 7 `Failed`, then kind 5. It transfers nothing. A path already stored in the task is authoritative across save and load. Restore does not search again. Search runs only when a travelling task has an empty path, or when a decision selects a goal that needs a path.

Proposed map, **NOT APPROVED:** sites 1 and 2, edges `(1,2,edge 1,cost 1)` and `(2,1,edge 2,cost 1)`, site 9 with no edge. Site 1 has a 3 by 3 grid, all cells passable except the center. A target of site 9 fails. A target of site 2 from site 1 succeeds in one movement tick.

Inventory transfer across locations is a transaction in OD-05. The stock has one owner before the commit and one owner after it. An in-transit cargo owner may be the destination of one committed transaction and the source of a later one. A failed leg does not leave the stock on the cargo owner.

### OD-12 — Automatic scheduling

**Recommendation, NOT APPROVED as a new policy:** do not add one. Mode changes only through admitted `SetMode` from producer 1. Active and scheduled actors use the same decision, reservation, and work rules. Visibility is not an input.

### OD-13 — Twenty actors on this reservation model

**Recommendation, NOT APPROVED:** M0-T06 stays in the exit. The 20-actor case is explicit bundle commands, not a needs wake, so the result does not depend on the weight table.

Actors 1 through 20 are enabled and stand on site 1. Each has exactly one current `Queued` task, ids 1 through 20, and `actor.task_id` equals that id. No actor has a second non-terminal task. Four stocks have on-hand quantity 5 and no active reservation: stock 101 beds, class 2; stock 102 tools, class 2; stock 103 food, class 1; stock 104 workstations, class 2.

Each actor submits one `ReserveBundle` at target tick 1, producer 1, sequence 1, naming that actor's current task and the four stocks in id order, quantity 1, expiry 10. Key order is actor 1 through actor 20.

Actors 1 through 5 each receive four kind 1 events, stock order 101, 102, 103, 104, then one kind 7 `Reserved`. Actors 6 through 20 each receive one kind 2 for stock 101, quantity 1, reason 4, and their tasks stay `Queued` with no reservations. On-hand quantities stay 5. Available quantities become 0. Food is not consumed. Each winner's single task holds all four reservation ids.

A later completion of actor 1's task debits food from 5 to 4 and releases every held reservation with reason 8. Bed, tool, and workstation on-hand quantities stay 5.

The one-unit, two-claimant case remains a separate script. Two actors, two current tasks, one consumable stock of quantity 1, and two `ReserveStock` commands.

A bundle whose tool or workstation is unavailable emits one kind 2 and creates neither reservation. That script uses one current task.

### OD-14 — Simultaneous reasons

**Recommendation, NOT APPROVED:** one task transition and one reason. Priority is disablement, then cancellation, then expiry. Each active reservation of the task is released once with that reason. Work does not apply legs.

### OD-15 — Phases

**Recommendation, NOT APPROVED:** after fixture 3 admits a boundary and increments the tick:

1. Mode. Apply due `SetMode` commands in key order. Emit nothing.
2. Command. Apply due `SetEnabled`, `BeginTask`, reservation, cancel marks, and `CompleteTask` flags in key order. `CompleteTask` sets `completion_requested` and does not apply legs.
3. Invalidation. Apply OD-14 to tasks whose claimant is disabled, whose cancel was marked, or whose active reservation has expiry less than or equal to the staged tick.
4. Decision. Run OD-09.
5. Work. Run OD-10 and OD-11.

Fixtures 1 and 2 do not enter phases 3, 4, or 5. Their published bytes stay the regression oracle.

### OD-16 — Fixture 2

**Recommendation, NOT APPROVED as an edit:** production rules exist only in fixture 3. Fixture 2 still completes a due synthetic task while its actor is disabled. No archive byte changes.

## 5. Worked bundle

This example is **NOT APPROVED** fixture data. It is the cardinality check for OD-04 and OD-10.

Actor 7 is enabled at site 1. Stock 102, a tool, on-hand 1. Stock 104, a workstation, on-hand 1. Task 70 is `Queued` for goal work. One `ReserveBundle` names both stocks, quantity 1, expiry 9.

The command phase allocates reservation 1 for stock 102 and reservation 2 for stock 104, in that stock order. Task 70 is actor 7's only current task. It stores both ids and becomes `Reserved`. The events are kind 1 for reservation 1, kind 1 for reservation 2, then kind 7 stage `Reserved`. Available quantity of each stock is 0. A second bundle emits one kind 2 and allocates nothing.

On the completion tick, `CompleteTask` sets `completion_requested` in the command phase. Invalidation does not fire. Work emits kind 4, then kind 3 for reservation 1, kind 3 for reservation 2, both reason 8, then kind 7 `Done`. Both on-hand quantities remain 1. The leg list is empty, and kind 4 is still emitted so the completion key exists.

If `SetEnabled(actor 7, false)` has the lower command key on that same tick, the command phase disables the actor and still sets `completion_requested`. Invalidation then emits kind 3 for reservation 1, kind 3 for reservation 2, reason 1, kind 7 `Failed`, and kind 5. Work applies no legs and emits nothing further. The quantities remain 1.

## 6. Decisions the owner must record

| ID | Decision | Approval |
| --- | --- | --- |
| OD-01 | Accept the fixture-3 layout, including one current task, `completion_requested`, planned legs, position, path, traces, and `BeginTask`, or replace that layout | Required |
| OD-02 | Accept the event dialect and the single event order, including kind 7 on every stage change | Required |
| OD-03 | Accept fixture-local command-key order and all-or-nothing requests | Required |
| OD-04 | Accept multiple reservations per task, atomic bundles, and the consumable versus durable rule | Required |
| OD-05 | Accept transaction ids allocated only after every leg validates | Required |
| OD-06 | Accept producer 1 and claimant-only cancellation | Required |
| OD-07 | Accept producer 1 as the only enable and disable source | Required |
| OD-08 | Accept the silent no-op | Required |
| OD-09 | Accept the decision phase, the neutral relationship gate, checked `i64` scoring, the weight table, and the wake interval, or substitute another integer table | Required. Needs stay in the exit |
| OD-10 | Accept the one-transition work table | Required. The five goals stay in the exit |
| OD-11 | Accept cost as both route weight and duration, the graph, and the grid, or substitute another authored map | Required. Navigation stays in the exit |
| OD-12 | Confirm the existing deferral of automatic mode selection | Confirmation |
| OD-13 | Accept one current task per actor, the four-stock bundle, and the supply count of 5, or replace the counts without dropping the row | Required |
| OD-14 | Accept the reason priority | Required |
| OD-15 | Accept the five phases | Required |
| OD-16 | Confirm fixture 2 is not rewritten | Confirmation |

Until those rows are recorded, the normative C0.3 specification stays the current draft and the G3 verdict stays FAIL.
