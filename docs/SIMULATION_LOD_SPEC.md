# Simulation LOD and Identity Contract

Version 0.2-draft · 17 November 2026

## 1. Ownership

`sealed_sim` owns a named actor's identity, records, processing mode and pending work for its entire lifetime. `sealed_app` owns visibility and disposable Bevy projections. A visible actor can use either simulation mode. An offscreen actor can remain active. A camera or projection operation alone cannot change a canonical field, select a mode, or admit or execute a command.

## 2. Domain processing modes

| Mode | Authoritative state | Update policy |
| --- | --- | --- |
| Active local | Same canonical ActorRecord, position, tasks and ownership | Due work eligible at its declared logical ticks |
| Scheduled named actor | Same record and pending work, with processing mode and next eligible wake | Process each due consequential transition once at its logical tick |
| Future aggregates | Site processes may reference named actors. Anonymous cohorts require a later ADR | Outside the C0.2 proof |

The active/scheduled modes first enter the C0.2 domain fixture. A mode change occurs within an ordered tick phase owned by the simulation, triggered by deterministic domain state or an explicitly admitted replayable control. Choose the production activation policy only with a deterministic fixture. Visibility never serves as the trigger. Scheduled travel approximation is optional after exact reference evidence and may affect only specified fields with a bound.

## 3. Projection operations

Materialization reads the current projection snapshot, installs or reuses a runtime mapping and is idempotent. Dematerialization removes the mapping and its client entities and is idempotent. Both operations are read-only with respect to `sealed_sim`. Neither drains an admitted command, evaluates an actor, releases a reservation or changes a logical tick. A rebuild reads the latest immutable projection snapshot. Canonically queued work remains pending until the relevant logical tick. I0-T01 exercises this behavior with the simulation paused and a future command queued.

## 4. Scheduled work and accounting

C02-T02 in the [M0 test document](M0_ACCEPTANCE_TESTS.md#6-owned-downstream-fixtures) defines explicit fixture transitions and due work. Process the fixture's mode triggers before due work in the same tick, then process due work by (due tick, actor ID, fixture work ID), ascending. Any promoted skip/fast-forward preserves due tick, order and observable outcomes relative to repeated single steps. Reject unsupported overdue/input state during preflight or load validation. The C0.2 oracle enables no approximate fields. An interrupted job, reserved cargo or already consumed material cannot be manufactured, duplicated or lost on transition.

Each physical item or quantity has one canonical owner/location. A reservation refers to existing stock. In-transit cargo is a canonical owner. Coarse site production consumes and produces through declared transactions, with no duplicate accounting across actor, cargo or site stores.

## 5. Continuity and evidence

A visibility-only change preserves actor ID, appearance seed, outfit, injuries, relationships, positions, processing mode, task progress, schedule, ledger, RNG ordinals, pending commands, event cursor and digest. A domain mode change preserves actor identity and exact fixture accounting as work progresses on tick. Check C02-T01/T02 for checkpoint and processing outcomes, V0-T10 for mock-only recovery, I0-T01 for actual pause/rebuild invariance, and M0-T04/T05/T12 for M0's linked evidence. Test 1,000 seeded projection operations against an unchanged complete canonical checkpoint in I0-T01.

Production activation policy and coarse travel remain open. C0.2 uses an explicit domain control schedule. Keep exact progression until M0-T05 defines an independently reviewable comparison and ADR-009 records any proposed field-specific approximation. Aggregate cohorts remain deferred under ADR-010.
