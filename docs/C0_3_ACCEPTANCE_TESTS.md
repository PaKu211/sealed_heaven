# C0.3 Acceptance Tests

Version 1.3-approved + RT-01 · 18 November 2026
Status: owner-authorized documentation after NF-05 packing-field remediation and cancellation-only RT-01. RT-01 authorizes its targeted documentation/tests and branch commit/push, not independent C0.3 acceptance or unrelated implementation.

The procedures below are the exit evidence for the [implementation specification](C0_3_IMPLEMENTATION_SPEC.md) version 1.3-approved. They keep C03-T01 and the written M0-T06, M0-T07, and M0-T08 sentences. C0.1 and C0.2 golden literals are not replaced. H0, the M0-T07 initial blob, and the C03-T01a and M0-T06 post-boundary checkpoint blobs are stated where independently derived from this contract alone.

## 1. Oracle

A fresh fixture starts the cursor at 1 and the digest at H0.

`H0 = SHA-256("SH:C03:EVENT-INIT:v1")` = `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c`.

That value was computed from the ASCII string. It was not copied from a Rust run.

Each later digest is `SHA-256("SH:C03:EVENT:v1" || previous || canonical_bytes)` with the bytes in the implementation specification §4. An independent packer, separate from the future `sealed_sim` encoder, packs checkpoint bodies from §3. A hash copied from the implementation is not that oracle.

A failed preflight or a failed later leg leaves the previous bytes, cursor, and digest unchanged. Replay repeats the same commands and the same bytes.

Shared constructor, except where a row replaces a field: seed 100, wake 4, the approved weights and relationship minimums, recipe 1, holders 20 and 30, sites 1, 2, and 9, edges 1 and 2 only. Actors used by a test are enabled, mode active, channels 0 except where a row sets hunger, relationship term 0, `home_site_id` 1, `next_wake_tick` 100, locus site 1, unused position fields 0, empty path unless stated.

**Preinstalled task packing fields**, except where a row replaces a field: `goal` 0 (scripted carrier; not decision-created), `recipe_id` 0, `interruption` 0 while non-terminal, target locus 1, target site 1, target x 0, target y 0, `progress_ticks` 0 before any work-phase advance, empty unconsumed list, empty path, empty planned legs. Spec §8: only work-phase advancing transitions increment `progress_ticks`; §6.1 reservation promotion does not.

## 2. Regression

From the frozen C0.2 tree, and again from any later tree that claims to preserve it:

```text
cargo fmt --all --check
cargo check --locked --offline
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --locked --offline
cargo test --release --locked --offline
```

C0.1 literals in `src/tests/golden.rs` and the C0.2 blobs named in `C0_2_BASELINE_MANIFEST.md` stay byte-identical. Event 3 digest stays `1a1cc22bf1041c19845bceb790e6761dad0edaf41cb3eca1f90b0ce06f1875b1`. Witness tick 1 stays `35b5104ca23dc7723baffbe76948a044fdda583eb8374604149133986e2f4ac2`. No Bevy import enters `sealed_sim`. Direct dependencies stay the frozen pins unless the owner later approves a crate. This specification approves none.

## 3. C03-T01a — two claimants

**Initial state.** Tick 0, seed 100, cursor 1, digest H0. `next_actor_id` 3, `next_record_id` 3, `next_transaction_id` 1, `next_stock_id` 2. Stock 1, definition 1, class 1, quantity 1, owner kind 3, owner id 1. Actor 1 task 1. Actor 2 task 2. Both actors at site 1 (the stock's site). Shared constructor otherwise. Consumed registry empty. Applied transactions empty. Pending empty. Traces empty.

| Task | actor | goal | recipe_id | stage | interruption | target (locus, site, x, y) | progress_ticks | created_tick | due_tick | path | reservations | planned legs |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 1 | 0 | 0 | Queued (1) | 0 | 1, 1, 0, 0 | 0 | 0 | 0 | empty | none | none |
| 2 | 2 | 0 | 0 | Queued (1) | 0 | 1, 1, 0, 0 | 0 | 0 | 0 | empty | none | none |

**Commands.** `drive(0, …)` with two `ReserveStock`, producer 1, sequence 1 each, target tick 1, quantity 1, expiry 5, task id equal to that actor's current task. Either batch order.

**Promotion.** Actor 1's accept sets stage `Reserved` and, under spec §6.1, `due_tick = 1 + 0 + 2 = 3`. `created_tick` stays 0. `progress_ticks` stays 0 through promotion. Actor 2 is rejected. Work then sees task 1 `Reserved`, `due_tick` 3 which is not below staged tick 1, empty path, at target → `Executing`, and increments `progress_ticks` to 1.

| Cursor | Kind | Result |
| --- | --- | --- |
| 1 | 1 | reservation 3, stock 1, quantity 1, actor 1 |
| 2 | 7 | task 1, stage Reserved (2) |
| 3 | 2 | stock 1, quantity 1, reason 4, actor 2 |
| 4 | 7 | task 1, stage Executing (4) |

**Final state.** End cursor 5. Tick 1. Digest after event 4. On-hand 1. Available 0 for other claimants. Reservation 3 active: stock 1, claimant 1, quantity 1, acquired 1, expiry 5, task 1, state 1. `next_record_id` 4. Both command orders match.

| Task | stage | due_tick | created_tick | progress_ticks | reservations | other packing fields |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Executing (4) | 3 | 0 | 1 | id 3 | unchanged from the initial packing fields |
| 2 | Queued (1) | 0 | 0 | 0 | none | unchanged from the initial packing fields |

**Replay checkpoint.** Save after this boundary. Restore. Prefix events are not emitted again. Next event id is 5.

Independently packed post-boundary blob SHA-256 (body‖trailer) from this contract: `11f9aae46d7f0f04b6c3efead0a25c3086bc56904877d237c0a79ae032a293d9`.

**Error.** None on the success path. A whole-boundary failure before publication leaves tick 0 and `due_tick` 0.

## 4. C03-T01b — expiry

**Initial.** Reservation 3, acquired 1, expiry 3, task 1 Completing, `due_tick` 3, `created_tick` 0, packing fields from §1 (`goal` 0, `recipe_id` 0, `interruption` 0, target site 1, preinstalled `progress_ticks` 2), enabled, unconsumed list empty. Cursor 1. Tick 2 so that staged tick becomes 3. This is an explicitly installed snapshot, not an assertion of an unrequested goal-0 Executing→Completing history; see the owner-authorized H10 clarification in specification §8.1. Its initial fields and expected events are unchanged.

**Commands.** `drive(2, [])`.

| Cursor | Kind | Result |
| --- | --- | --- |
| 1 | 3 | reservation 3, reason 3 |
| 2 | 7 | task 1, stage Failed (8) |
| 3 | 5 | task 1, reason 3 |

**Final.** End cursor 4. No kind 4. `actor.task_id` 0. A later release emits nothing.

## 5. C03-T01c — cancel

**Initial.** Same Completing task shape, expiry 9, reservation 3 active, claimant actor 1.

**Commands.** `CancelReservation`, producer 1, actor 1, reservation 3, target 3.

| Cursor | Kind | Result |
| --- | --- | --- |
| 1 | 3 | reservation 3, reason 2 |
| 2 | 7 | task 1, stage Cancelled (7) |
| 3 | 5 | task 1, reason 2 |

**Errors.** Non-claimant actor id is `InvalidPayload` before any no-op. A later cancel of that terminal reservation by the claimant is a no-op.

## 6. C03-T01d — disable after acquire

**Initial.** Actor 1 enabled. Task 1 uses the §3 packing table (`Queued`, `created_tick` 0, `due_tick` 0, `goal` 0, target site 1, `progress_ticks` 0). Stock and reservation shape as §3 with expiry 4.

| Tick | Commands | Events | End cursor | Notes |
| --- | --- | --- | --- | --- |
| 1 | Reserve as §3 | Kind 1 reservation 3. Kind 7 Reserved. Kind 7 Executing | 4 | §6.1 sets `due_tick` 3. `created_tick` stays 0 |
| 2 | Admit `SetEnabled(actor 1, 0)` for target 3 only | None | 4 | Bit not applied yet |
| 3 | Empty due set-enabled applies | Kind 3 reason 1. Kind 7 Failed. Kind 5 reason 1 | 7 | No kind 4 |

If `CompleteTask` shares tick 3, the command phase appends a completion record; invalidation still emits those three events and moves unconsumed keys to role 2. Fixture 2 disabled synthetic completion hashes stay.

## 7. C03-T01e — completion replay

After one applied completion, the consumed registry holds the role-1 record with the submitted task id.

| Replay | Result |
| --- | --- |
| Same key, same submitted `CompleteTask` id | No-op. No event. Cursor and digest stay. The tick may advance |
| Same key, different submitted id, including `CompleteTask(0)` versus `CompleteTask(N)` | `ConflictingIdentity`. Tick unchanged |
| Same key, different command type or non-matching payload | `ConflictingIdentity`. Tick unchanged |
| Role-2 key with matching submitted id | No-op. No second transaction |

A repeated terminal release emits nothing.

## 8. C03-T01f — invalid later leg, reservation protection, existing-stock credit

### Later-leg failure

Stock 14 quantity 1, definition 2, class 1, owner kind 3, owner id 2. Planned legs: debit stock 14 quantity 1, then credit stock 0 definition 2 class 1 quantity 1 owner kind 3 owner id 0. First leg succeeds on the staged copy. Second leg fails. `InvalidLeg`. No event. No transaction id. Tick stays. Stock 14 remains quantity 1.

### Other-task reservation protection

Actors 1 and 2. Stock 10 quantity 2. Task 1 Completing with debit 1 and reservation 3 quantity 1. Task 2 Queued with reservation 4 quantity 1. Available to task 1 is 1. Debit 1 succeeds and releases reservation 3 once. Debit 2 returns `InvalidLeg` and leaves both reservations active.

### Credit to existing stock

Stock 20, definition 7, class 1, quantity 1, owner kind 3, owner id 1. Planned legs: debit an inventory stock of definition 7 class 1 quantity 1, then credit `stock_id` 20, definition 7, class 1, quantity 1, owner kind 3, owner id 1. Success leaves stock 20 quantity 2. A credit naming stock 20 with definition 8, or owner id 2, or class 2, returns `InvalidLeg` and restores both quantities. A credit with `stock_id` 0 allocates a new stock instead.

## 9. C03-T01g and C03-T01i

Save while the unconsumed completion list is non-empty and the stage is not `Done`. The blob contains submitted and resolved task ids. Restore. Prefix events are not emitted again. Resume consumes the lowest key and retains every key in the registry. One-boundary `drive` and grouped `drive` match after every boundary.

## 9A. H09 — applied non-completion replay

**Initial.** Shared two-claimant state from §3: tick 0, seed 100, cursor 1, digest H0. Actor 1 is already enabled and task 1 is non-terminal.

**Applied no-op.** Boundary 0 admits actor 1, producer 1, sequence 1, target 1, `SetEnabled(1)`. It emits nothing. Tick is 1, cursor 1, digest H0. The checkpoint body contains no applied-command history for this key.

**Positive old-target replay.**

| Step | Input | Expected |
| --- | --- | --- |
| 1 | Same key, `CancelReservation(0)`, at boundary 1 | `LateTarget`. Tick, cursor, digest, and full checkpoint bytes unchanged |
| 2 | Save after step 0, restore, repeat step 1 | Identical `LateTarget` and unchanged bytes |
| 3 | Empty boundary at tick 0 instead of step 0 | Byte-identical tick-1 checkpoint; step 1 produces the same `LateTarget` and unchanged bytes |

Step 3 is the no-hidden-cache assertion. A no-op applied command and an empty boundary are indistinguishable in canonical state, and they must remain indistinguishable after restore.

**Negative structural cases.** From the same tick-1 image, every case fails with no mutation:

| Case | Expected |
| --- | --- |
| Old-target replay key with producer 0 | `InvalidProducer` (row 2) |
| Old-target replay key with an unknown actor id | `UnknownActor` (row 3) |
| The same key twice in one admission | `ConflictingIdentity` (row 4) |
| A pending future key submitted again before its due tick | `ConflictingIdentity` (row 4) |

**Future target.** At boundary 1, the same key with target 2 and `SetMode(Scheduled)` is an ordinary new command, not a historical replay. It is admitted, emits nothing, and applies at tick 2 with mode Scheduled. Re-submitting that key at tick 2 returns `LateTarget` and leaves the bytes unchanged. Save/restore before any case reproduces the same result and the same bytes.

**Durable completion identities.** Consumed `CompleteTask` identities are unaffected: §7 still governs the 41-byte registry, including no-op replay for the same submitted id and `ConflictingIdentity` for a different payload or type.

## 10. C03-T01h — rollback and hostile saves

Over-reserve, quantity 0, an empty interval, and a reservation sum that would overflow `u64` fail before publication. An `i64` score overflow and a `u64` route-cost overflow return `CapacityExceeded` and emit nothing. Cursor overflow returns `CapacityExceeded`. An unbounded count, an illegal grid, or a registry count that does not fit returns `Corrupt`. A task id equal to a reservation id, a dangling task id, or `resolved_task_id` 0 is `InvalidState`. Truncation, wrong tag, wrong revision, bad checksum, unknown fixture, and content mismatch are distinct. A C0.2 blob is `UnsupportedFormat`. The following valid boundary matches a run that never attempted the failed call.

## 11. M0-T06

**Initial.** Tick 0, seed 100, cursor 1, digest H0. `next_actor_id` 21, `next_record_id` 21, `next_transaction_id` 1, `next_stock_id` 105. Actors 1 through 20 at site 1, each with current task id equal to the actor id. Shared constructor otherwise. Consumed registry empty. Applied transactions empty. Pending empty. Traces empty.

Stocks:

| Stock | definition_id | class | quantity | owner |
| --- | --- | --- | --- | --- |
| 101 beds | 3 | 2 | 5 | kind 3, id 1 |
| 102 tools | 4 | 2 | 5 | kind 3, id 1 |
| 103 food | 1 | 1 | 5 | kind 3, id 1 |
| 104 workstations | 5 | 2 | 5 | kind 3, id 1 |

Preinstalled tasks 1 through 20 (identical packing fields; `actor_id` equals task id):

| Field | Value |
| --- | --- |
| goal | 0 |
| recipe_id | 0 |
| stage | Queued (1) |
| interruption | 0 |
| target | locus 1, site 1, x 0, y 0 |
| progress_ticks | 0 |
| created_tick | 0 |
| due_tick | 0 |
| path | empty |
| reservations | none |
| planned legs | none |

**Commands.** One `ReserveBundle` per actor, producer 1, sequence 1, target 1, stocks 101–104 in id order, quantity 1, expiry 10. Command-key order admits actors 1 through 20 ascending.

**Promotion.** Actors 1 through 5 accept. Each accept sets `due_tick = 3` under §6.1. `created_tick` stays 0. `progress_ticks` stays 0 through promotion. Reservations 21 through 40 in ascending actor then stock id: actor `a` receives ids `21 + 4*(a-1)` through `24 + 4*(a-1)` for stocks 101, 102, 103, 104. Work emits `Executing` for each winner because `due_tick` 3 is not below staged tick 1, and sets each winner's `progress_ticks` to 1. Actors 6 through 20 each receive one kind 2 for stock 101, quantity 1, reason 4, and remain `Queued` with `due_tick` 0 and `progress_ticks` 0.

**Events.** For each winner in actor order: four kind 1 (ascending new reservation id / stock id), then one kind 7 `Reserved`. Then each loser in actor order: one kind 2 stock 101 quantity 1 reason 4. Then work: one kind 7 `Executing` for each winner in actor order. End cursor 46.

**Final.** Tick 1. Digest after event 45. `next_record_id` 41. On-hand stays 5. Available 0. Food is not consumed. Queued losers with `due_tick` 0 do not fail the boundary. Scheduled mode does not change the winners. §3 is the included one-unit case as a separate script.

| Tasks | stage | due_tick | progress_ticks | reservations |
| --- | --- | --- | --- | --- |
| 1–5 | Executing (4) | 3 | 1 | four active ids as above |
| 6–20 | Queued (1) | 0 | 0 | none |

**Replay.** Save after the contention boundary. Restore. Next event follows the saved cursor. Winners keep `due_tick` 3 and `progress_ticks` 1.

Independently packed post-boundary blob SHA-256 (body‖trailer) from this contract: `528bc334bc1f081ccbf06c3247ec46909abd6f1073594dfb13ebd7ae16d640c9`. Body SHA-256 `bccf8e1adf8bc22623202a58fd5d84b9911d8d1e42959331c692fcc55e3f1ed0`.

## 12. M0-T07

**Initial,** before `drive(0, …)`: seed 100, tick 0, wake 100. Cursor 1. `next_record_id` 401. `next_stock_id` 11. `next_transaction_id` 1. Actor 1 enabled, `task_id` 0, locus site 1. Stock 10, definition 1, class 1, quantity 1, owner kind 1, owner 1. Holders 20 kind 2 and 30 kind 4. Recipe 1 as in the implementation specification §12. Consumed registry empty.

Independently packed initial blob SHA-256 (body‖trailer) from this contract: `bd803a2c4ae4320ce2a1a5ceb49a2941aec48921c4cbc0a60b15c607b62148c3`. Body SHA-256 `787a40d5eabadfeb9294851ee967dd0a81122fac44f91b04b961f1ced59f6f78`.

Every scripted command uses actor 1 and producer 1. `BeginTask` uses stage `Completing`, empty path, no reservations, locus site 1, and `due_tick` equal to the staged tick of that command plus 1. Hops 1 through 3 use goal 4 and `recipe_id` 0. Task 404 uses goal 3 and `recipe_id` 1. Task 405 uses goal 4 and `recipe_id` 0. `CompleteTask` on a create tick uses submitted task id 0.

| Staged tick | Submission | Work | Events | State after the tick |
| --- | --- | --- | --- | --- |
| 1 | Begin sequence 1, hop 1. Complete sequence 2, submitted 0 | Skipped | 1: kind 7, task 401, stage 5 | Request list holds sequence 2. Stock 10 unchanged |
| 2 | Empty | Apply hop 1 | 2: kind 4, transaction 1. 3: kind 7, stage 6 | Stock 10 quantity 0. Stock 11 at holder 20 |
| 3 | Begin task 402, hop 2. Complete sequence 4, submitted 0 | Skipped | 4: kind 7, stage 5 | Task 402 current |
| 4 | Empty | Apply hop 2 | 5: kind 4, transaction 2. 6: kind 7, stage 6 | Stock 11 quantity 0. Stock 12 at cargo 30 |
| 5 | Begin task 403, hop 3. Complete sequence 6, submitted 0 | Skipped | 7: kind 7, stage 5 | Task 403 current |
| 6 | Empty | Apply hop 3 | 8: kind 4, transaction 3. 9: kind 7, stage 6 | Stock 12 quantity 0. Stock 13 at site 2 |
| 7 | Begin task 404, recipe 1. Complete sequence 8, submitted 0 | Skipped | 10: kind 7, stage 5 | Task 404 current |
| 8 | Empty | Apply recipe | 11: kind 4, transaction 4. 12: kind 7, stage 6 | Stock 13 quantity 0. Stock 14 definition 2 class 1 quantity 1 at site 2 |
| 9 | Begin task 405, debit 14 then credit owner 0. Complete sequence 10, submitted 0 | Skipped | 13: kind 7, stage 5 | Task 405 current. `due_tick` 10. Stock 14 still quantity 1. `next_event_id` 14 |
| 10 | Empty | Later-leg `InvalidLeg` | None | Tick remains 9. Cursor remains 14. Task 405 still Completing. Stocks 10 through 14 unchanged from tick 9 |

Hop legs:

| Hop | Debit or transform-out | Credit or transform-in |
| --- | --- | --- |
| 1 | Stock 10, definition 1, class 1, quantity 1 | Credit stock 0, definition 1, class 1, quantity 1, owner kind 2, owner 20 |
| 2 | Stock 11, definition 1, class 1, quantity 1 | Credit stock 0, definition 1, class 1, quantity 1, owner kind 4, owner 30 |
| 3 | Stock 12, definition 1, class 1, quantity 1 | Credit stock 0, definition 1, class 1, quantity 1, owner kind 3, owner 2 |
| Recipe | Transform-out stock 13, definition 1, class 1, quantity 1 | Transform-in stock 0, definition 2, class 1, quantity 1, owner kind 3, owner 2 |
| Invalid | Debit stock 14, definition 2, class 1, quantity 1 | Credit stock 0, definition 2, class 1, quantity 1, owner kind 3, owner id 0 |

On tick 10 the first leg succeeds on the staged copy and the second leg fails. The whole boundary rolls back. Ticks 1 through 9 remain. Creation-tick skip held on every create tick. No reservations on these hops, so success events have no kind 3.

## 13. M0-T08

Cancel, disable, and expiry are §5, §6, and §4. When all three are true, the reason byte is 1. The order is kind 3, kind 7 Failed, kind 5.

Unreachable site 9 at decision rejects the goal with reason 5 before a task is created. A travelling task that already has an empty path on a site fails in work with kind 7 Failed and kind 5, plus kind 3 when a reservation is held. The stock owner does not change.

Rest or Return Home, no reservation, Completing, unconsumed list non-empty, actor enabled: kind 4 with an empty leg list, then kind 7 Done. The same task with disablement on that tick: kind 7 Failed, then kind 5 reason 1. No kind 4.

Repeat release and matching completion replay are §7.

## 14. Needs, jobs, and movement

**Eat.** Cursor 1, hunger 1000, other channels 0, relationship term 0, neutral minimums, wake equal to the tick, no current task, one class-1 definition-1 food unit in actor inventory, actor already there.

| Cursor | Kind | Result |
| --- | --- | --- |
| 1 | 1 | the food reservation |
| 2 | 7 | the new task, Executing |
| 3 | 6 | winner eat, that task, next wake = tick + 4, eat score 1000000, other scores 0 |

`due_tick = staged + 2`. Work emits nothing on the creation tick. The next tick emits one kind 7 Completing. The following tick emits kind 4, kind 3 reason 8, and kind 7 Done if the reservation survived. Restore before the stored wake emits no second kind 6.

**Work.** Tool = lowest class-2 definition-4 stock at the site. Workstation = lowest class-2 definition-5 stock at the site. Bundle both. Quantities unchanged on completion.

**Haul.** Lowest class-1 inventory stock with definition ≠ 1. Credit home stockpile: existing matching stock if present, else `stock_id` 0.

No eligible goal: one kind 6, winner 0, task 0.

### Movement

Site 1 to site 2 uses edge 1 and costs one later work tick. Grid path `(0,0)` to `(2,2)` is north, north, east, east. Site 9 is unreachable. OC-EDGE is rejected.

### Residual edge travel

Actor 1 on edge 1, `traversed_cost` 0, travelling task 50, empty path.

1. Cancel task 50. Events: kind 7 Cancelled, kind 5. `task_id` 0. Locus still 3.
2. Save and restore. Locus 3 and `traversed_cost` 0 remain.
3. Empty `drive`. Actor arrives at site 2. No event.
4. Repeat from the post-cancel save, disable the actor, then empty `drive`. Arrival still occurs. No task is created.

**RT-01 timing clarification (owner approved, cancellation only).** Use initial tick 0, task 50 due 4, cursor 1 and H0. Submit `CancelReservation(0)` with key `(target 1, actor 1, producer 1, sequence 1)` through `drive(0, ...)`. The committed tick is `T = 1`: task 50 is Cancelled with reason 2, `task_id` is 0, and locus 3 / edge 1 / traversed 0 are preserved. The only events are id 1 kind 7 `(50, Cancelled)` and id 2 kind 5 `(50, reason 2)`; cursor is 3. Save/restore before and after cancellation must reproduce complete bytes. The next successful `drive(1, [])` commits tick 2 and spends the remaining cost 1, arriving at site 2 with edge 0 and traversed 0, silently. In step 4, submit `SetEnabled(0)` with key `(2,1,1,2)` at boundary 1: that same tick-2 `drive` also performs residual arrival despite disablement; a subsequent empty `drive` is silent and does not repeat arrival or create a task. Both branches preserve the post-cancel event cursor/digest across residual movement. An invalid `SetEnabled(2)` at boundary 1 must return `InvalidPayload` without advancing the tick or changing checkpoint bytes; a subsequent valid tick-2 retry still arrives. This qualification preserves steps 1–4 and does not alter other interruption/residual cases or any published golden value.

## 15. Coverage

| Exit item | Section |
| --- | --- |
| C03-T01 | §§3–10 |
| H09 applied non-completion replay | §9A |
| M0-T06, including the one-unit case | §§11 and 3 |
| M0-T07 Actor → Store → Cargo → Site, recipe, later-leg rollback | §12 |
| M0-T08 cancel, disable, expiry, unreachable, exactly once | §13 |
| Needs, jobs, movement, residual travel | §14 |
| C0.1 and C0.2 regression | §2 |

No mandatory exit sentence was removed. OC-EDGE is rejected and is not an acceptance case.
