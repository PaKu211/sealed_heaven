# Technical Architecture — Rust + Bevy

Version 0.2-draft · 17 November 2026

## 1. Boundary

The pure Rust `sealed_sim` library owns all authoritative domain state, including pending commands, actor processing mode, identity and consequential events. Explicit input producers submit immutable tick batches to the simulation. The application drives steps and consumes read-only outputs. Presentation actions and camera state have no write path into the domain.

The later `sealed_app` client owns Bevy input collection, 2D renderer, camera, UI, audio and disposable projections. Later `sealed_tools` hosts headless scenarios and diagnostics once the first workload needs a separate runner. World simulation is a dependency of that runner and client. The dependency direction never reverses.

## 2. Target workspace and actual C0.1 scope

C0.1 creates exactly one library package, `sealed_sim`, with fixture tests in the same package. The [C0.1 specification](C0_1_IMPLEMENTATION_SPEC.md) defines its allowed contents and interfaces. Cargo setup, core code and tests begin after the project owner authorizes implementation.

Later gates introduce:

| Gate | Additional owner or data |
| --- | --- |
| C0.2 | Domain checkpoint I/O, allocation, scheduled actor fixture |
| C0.3 | Needs, jobs, physical inventory/ledger and initial navigation |
| V0 | `sealed_app` with mock data, authored tiles, UI and portrait assets |
| I0 | The real read-only domain projection and host integration |
| M0 | `sealed_tools` when needed for the headless long run, scenario fixtures and typed content diagnostics |

Add loaders, content registries, path helpers or tooling only when the first named fixture needs them. A separate content crate, generalized plugin interfaces and parallel domain execution remain deferred.

## 3. Dependencies and provenance

ADR-001 retains Bevy 0.20.1 for `sealed_app` beginning at V0. Client features and the locked platform build are V0 evidence. C0.1 dependency candidates and their verification are in the [implementation specification](C0_1_IMPLEMENTATION_SPEC.md#replay-and-dependencies) and C01-T06.

Serde/JSON may enter the checkpoint or typed-content milestone. Route-search dependencies require equal-cost and successor ordering tests when a route fixture needs them. An optional JSON Schema tool belongs in offline diagnostics only. If selected, disable default remote resolution, permit only explicitly registered local schemas, and run the no-I/O remote-reference case in M0-T09. Direct whole-game dependencies remain excluded under ADR-019. [Bevy release](https://github.com/bevyengine/bevy/releases/tag/v0.20.1) · [Serde](https://serde.rs/) · [jsonschema offline configuration](https://docs.rs/jsonschema/latest/jsonschema/#offline-validation)

## 4. Simulation host

An explicit driver submits one closed batch for the current logical boundary and invokes one canonical step. For replay, use the recorded boundary and target ticks. Any grouped driving preserves the sequence of explicit boundaries. The headless M0 driver reads the scenario time and report parameters independently from the client scheduler.

At I0, a real-time accumulator requests steps without passing frame delta to domain rules. Live input is buffered in the host in collection order, then stamped at the first eligible step boundary, with the earliest target at boundary + 1. Assign deterministic producer-local sequences before sealing that batch. Accepted input is recorded for replay. Input captured while paused may remain in the host buffer until resume. Pause takes effect between steps, resets the host's accumulated fractional time and runs zero steps. Resume accrues time from that boundary without catch-up for time spent paused. No projection action can request a step. I0-T02 compares pre-stamped equivalent inputs across varying frame cadence and exercises the live input seam independently.

## 5. Bevy materialization and presentation contract

Read-only snapshots are labelled by their canonical tick. I0 applies each output delta from one committed tick to its immediate successor, including empty changes as needed to account for the revision. A delta whose target is already applied is discarded. When its base does not match the current projection revision and the target is newer, reacquire the latest full snapshot and rebuild. Apply subsequent deltas from the rebuilt snapshot's revision. V0 mocks use the same version/recovery semantics without importing canonical state.

A runtime projection maps stable domain identity, such as ActorId, to zero or one Bevy entity per named actor. Materializing and dematerializing map and dispose of client-side presentation only, are idempotent, and never submit, flush or execute gameplay commands. Pending admitted commands stay in the domain. Camera visibility may change client entities and caches. The domain alone decides active/scheduled modes. Animation may interpolate render positions and drop optional FX. Committed rules determine logical movement, collision, damage and transactions. See the [LOD contract](SIMULATION_LOD_SPEC.md) and I0-T01.

## 6. Scheduling and thread safety

Canonical mutation is serial and ordered. A later parallel read-only producer experiment must preserve the accepted batch and event sequence under the corresponding replay test. Bevy schedules are free to schedule UI, render and FX without determining domain event order.

## 7. Asset, content and world generation pipeline

Later typed-content validation parses definitions, resolves namespaced IDs and references, and validates semantics before constructing an immutable registry and deterministic content identity. Bounded composition is conditional on approval. The initial world fixture is authored or seeded in stages when requested by M0. Full climate/history remains deferred.

Visual variation uses authored layers plus canonical tile state and stable appearance seeds. Rendering consumes its own deterministic choices without consuming domain RNG draws. Named NPC faces remain stable when a job changes.

## 8. Save and compatibility

C0.1 checkpoints are in-memory canonical observations. Beginning in C0.2, a versioned domain data transfer format stores every future-affecting field in [Core §11](CORE_SIMULATION_SPEC.md#11-events-histories-and-persistence). Retain pending commands and the event prefix digest/cursor, and resume at the loaded boundary. Already emitted events keep their IDs and are not emitted again. Keep explicit prior-format fixtures for migration, return actionable errors for invalid or unsupported saves, and exclude Bevy handles and caches. C02-T01 and M0-T02/T03/T10/T11 own the evidence.

## 9. Build, QA and performance

C0.1 runs C01-T01 through C01-T06. Later gates add acceptance tests only with their dependencies. V0 records graphics observations on available hardware. I0 adds real bridge checks. The M0-T14 executable and dependency graph remain headless even when separately collected I0 evidence is required for M0 closure. Performance reports record exact scenarios, hardware/target, profile and metrics. Gate verdict and executed evidence are recorded separately.

## 10. Planned spikes and owners

- C0.1 owns the explicit kernel and replay fixture.
- C0.2 owns checkpoint I/O and scheduled progression with exact reference accounting.
- C0.3 owns the resource transaction and navigation fixtures.
- V0 owns the canvas, tilemap, portrait and input/readability experiments.
- I0 owns the projection and host integration.
- M1 owns the interaction-cadence spike before choosing a final host rate.
