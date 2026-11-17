# Core Simulation Specification

Version 0.2-draft · 17 November 2026

This document defines cross-milestone rules. The only initial kernel requirements are in the [C0.1 specification](C0_1_IMPLEMENTATION_SPEC.md) and its [acceptance matrix](C0_1_ACCEPTANCE_TESTS.md). C0.2 introduces persistence and domain LOD. C0.3 introduces production actor needs, tasks and physical resources. The records and invariants below apply when their owning gate introduces them.

## 1. Objectives and non-goals

The authoritative game domain must run headlessly without GPU, window, animation or Bevy ECS. Given pinned build/content, initial state, seed and external command log, it must reproduce canonical state and consequential event sequence on the same platform. Cross-platform bitwise stability and massive-scale simulation are NOT C0 requirements.

## 2. Canonical data ownership

SimWorld owns domain actors, positions, needs, injuries, tasks, inventories, resources, faction relations, clocks, events, scheduling and RNG state. Render entities are projections. No Bevy Transform, sprite, particle or UI widget may authoritatively determine gameplay state.

Minimal installed domain records by C0.3/M0:

| Record | Essential fields |
| --- | --- |
| SimWorld | schema, content manifest, seed, tick, scenario time parameters, regions/sites, actor table, inventory ledger, faction table, pending admitted commands, schedule, RNG state, allocation cursors, event cursor and digest |
| ActorRecord | ActorId, site/position, appearance seed, needs, health/injuries, resources/progression, inventory ref, faction, relationships, task state, next wake tick |
| SiteRecord | SiteId, location/route graph, resource nodes, storage owners, jobs, faction presence |
| ItemStack/Instance | unique identity where necessary, definition ID, quantity and owner/location. Available quantity derives from active reservation records |
| Reservation | reservation ID, subject/resource, claimant, quantity, expiration, task ID |
| TaskState | stage, goal, target references, progress ticks, held reservations, interruption metadata |

Strongly typed IDs (ActorId, SiteId, FactionId, ItemInstanceId, TaskId, EventId) have stable serialized values and deterministic allocation. Indexes, slotmap handles and Bevy Entity handles are not persistent identity. ID reuse is prohibited until an explicit future migration policy exists.

## 3. Time model

A logical tick is a canonical unsigned 64-bit count. C0.1 advances it through explicit boundary inputs. Starting with scenarios that measure days, `ticks_per_game_day` is a positive `u64` scenario parameter. Record it in scenario identity, preserve it on reload and include it in replay evidence. At tick T, the elapsed whole days are floor(T / ticks_per_game_day), and the within-day tick is T modulo that parameter. Starting at tick zero, a 30-day run ends at exactly 30 × that parameter. Reject zero or an overflowing scenario horizon before running. Durations and scheduled triggers remain integral ticks. The fixture author pins this value per scenario.

Real-time host cadence is measured as steps per wall-clock second. Pause invokes zero steps, and speed changes cadence without changing any rule within a step. Exact 10/20/30 Hz are M1 host experiments. Long jumps must process every due consequential transition at its logical tick, in specified order. C0.2 uses a one-step reference and may preserve it as the only driver. Skipping idle segments is optional after equivalence and profiling.

## 4. Canonical command pipeline

At each boundary, the simulation receives an immutable batch and performs read-only preflight on input structure and timing. It admits the entire batch or returns a typed ingress error with no mutation. It orders due commands, rechecks each action against committed state at its turn, and commits each complete outcome including its event. Later rules may add explicit phases with their own total ordering, eligibility and conflict fixtures. No order is inferred from frame time, producer emission order or a Bevy system schedule.

C0.1 uses the numeric key (target tick, actor ID, producer ID, sequence), also its command identity. Its exact admission, duplicate, late-input and conflict rules are in the [C0.1 specification](C0_1_IMPLEMENTATION_SPEC.md#command-input-and-tick-boundary). Actor-ID ordering is a fixture policy. Active domain tasks and future fairness rules require separate milestone decisions. Projection and UI code consume outputs. Explicit input producers can propose future commands, while the simulation owns all admitted work.

## 5. RNG and reproducibility

Use named deterministic streams with a pinned algorithm and version. Scope decision ordinals by subsystem and subject, and record them wherever needed to continue a run. The [C0.1 encoding](C0_1_IMPLEMENTATION_SPEC.md#canonical-encoding-revision-1) is exhaustive for its fixture. Its published seed and event vectors are normative for revision 1.

As later gates add state, their versioned canonical schemas extend the inventory to include every future-affecting field. These include pending commands, scheduler state and ties, task/LOD state, reservations/ledger, deterministic ID cursors, RNG ordinals, content identity and time parameters. Encode in deterministic semantic order. Exclude render and debug artifacts. The event stream contributes through its ordered incremental digest and next ID. Compare emitted events separately. Compute checkpoint hashes when requested by the driver or test. Consequential records advance the digest as they occur.

Reproducibility tiers are R0 same-seed initial fixture, R1 same-build/platform replay with identical inputs, and R2 cross-platform/version replay (deferred). These are replay tiers, separate from the G0 remediation steps.

## 6. Autonomous actors and tasks

C0.3/M0 goals: eat, rest, work, haul, return home; socialize and react-to-threat are limited optional extensions. Utility scoring considers needs, role/job priority, reachability, inventory, reservations and relationship constraints. Scoring happens on scheduled decision ticks, not every render frame. Record DecisionTrace: candidate names, terms, scores, rejection reasons, winner, task and next evaluation tick.

Task lifecycle: Queued → Reserved → Travelling → Executing → Completing → Done. Cancellation, interruption and failure are explicit branches. Release resources once on terminal failure or cancellation. Current task and progress survive save/load and LOD transitions.

## 7. Inventory, resources, and economy

Every item/resource quantity has exactly one canonical ownership location: actor inventory, container, site stockpile, in-transit cargo or explicitly modeled reserve. Reserving never creates a second owner. Transfers are validated atomic transactions; source balance decreases and target increases within the same commit.

Recipes consume declared inputs and create declared outputs. A per-resource balance is not necessarily constant across transformations: accepted sources, sinks and transformations must be traceable in a ledger. No unexplained creation, destruction, negative stock or simultaneous ownership is allowed. Physical stock is the M0 authority.

C0.3 owns the production reservation lifecycle in [C03-T01](M0_ACCEPTANCE_TESTS.md#6-owned-downstream-fixtures). The ledger derives available quantity from on-hand stock minus active reservation records. The interval is [acquired tick, expiry tick). At a tie, actor disable/cancel and expiry invalidate a reservation before task completion. A repeated terminal release is a no-op. A completion has a stable task/transaction identity and applies every validated leg exactly once. The C0.3 spec will define keys and diagnostics before implementation against that fixture.

## 8. Spatial model and pathfinding

M0 has one small region with two sites, an abstract route graph and a local 2D grid. Simulation validates passability and actor position; Bevy camera/Transform is non-authoritative. Deterministic successor and equal-cost tie ordering are required when route search enters the first navigation fixture. Initial local spatial search uses a simple grid; nearest-neighbour acceleration and flow fields require profiling first.

## 9. Combat/progression extension points

Generic ability is an action with eligibility conditions, tick durations, resource costs, target and typed effects. Combat intent -> windup -> commit -> recovery is proposed for M1. Health changes, injury and resources are canonical. Qi and cultivation are named content/ruleset definitions layered over generic pools/tracks; no Qi enum belongs to the kernel.

## 10. LOD and presentation

The [LOD contract](SIMULATION_LOD_SPEC.md) governs C0.2 scheduled actors and I0 disposable projections. The simulation owns processing mode and all pending commands. Camera visibility and projection teardown operate on presentation state. Pause, teardown and rebuild never drain commands. I0 consumes read-only output at specified revisions.

## 11. Events, histories and persistence

Keep the boundary-indexed external command log, consequential domain events and curated player history distinct. C0.1 has an in-memory checkpoint only. Starting at C0.2, persist an explicit schema version and all future-affecting state, including any pending admitted commands, clock/scenario/rules identity, allocation and RNG cursors, and the consequential-event prefix digest plus next ID. Persist available history according to its own contract, without re-emitting it. Runtime entity handles are excluded.

For resume at tick N, the checkpoint already contains commands admitted at earlier boundaries for future targets. Continue the input log at boundary N. Retain the pre-save event prefix for comparison, append only newly emitted suffix, and compare prefix plus suffix with the uninterrupted event stream. Compare complete state and cursor/digest at the split and end. See C02-T01 and M0-T02/T03 in the [M0 tests](M0_ACCEPTANCE_TESTS.md). M0-T11 specifies explicit schema migration.

## 12. Testable invariants

Apply the first owning milestone for each invariant. C0.1 tests its fixture invariants separately.

1. Every live reference resolves to a valid persistent ID.
2. One item quantity has one owner/location; reservations never exceed availability.
3. All inventory creation/transformation/destruction is explainable by declared transactions.
4. A dead/disabled actor cannot continue an invalid job.
5. Task cancellation releases reservations exactly once.
6. Same seed/content/build/platform/input commands yields same canonical hash/event sequence.
7. Saving mid-task and reloading cannot change the task's next eligible action.
8. Named actor identity/attributes/relationships/inventory persist across any representation transition.
9. Bevy presentation rebuild leaves canonical state unchanged.
10. No event is backdated before its triggering canonical tick.

## 13. Observability and gates

C0.1 reports the evidence in C01-T06. M0 records scenario/build/content/seed, simulated days and ticks, due evaluations, path queries, consequential events, ledger deltas, checkpoint hashes, wall time, memory and phase timing. Record trace sampling separately and retain all traces used by the causal oracle. Tie any performance claim to fixed input, hardware, platform and build profile. See the [M0 acceptance criteria](M0_ACCEPTANCE_TESTS.md).
