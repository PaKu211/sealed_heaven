# Architecture Decision Register

Version 0.2-draft · 17 November 2026

ACCEPTED entries are binding scope or behavior for the named milestone. PROVISIONAL entries require their stated experiment. DEFERRED entries require an explicit later milestone. REJECTED entries record alternatives. The accepted remediation records below describe the proposed G0 baseline after owner-approved R1–R3 changes. Independent R4 review of this baseline is pending.

| ID | Status | Decision and rationale | Alternative considered | Consequence and affected tests |
| --- | --- | --- | --- | --- |
| ADR-001 | ACCEPTED | Rust domain and Bevy 0.20.1 client baseline. Preserve the original stack with an isolated build boundary | Add Bevy at C0 start | C0.1 is Bevy-free. Select and lock client features at V0. C01-T06, V0-T01/V0-T10, I0-T01 |
| ADR-002 | ACCEPTED | Pure Rust canonical SimWorld gives one simulation authority and supports headless testing | Authoritative Bevy ECS or dual copies | Only the domain commits commands and changes mode. C01-T06, M0-T14, I0-T01 |
| ADR-003 | ACCEPTED | Stable domain IDs outlive presentation | Save Bevy Entity/index handles | Fixture IDs at C0.1, deterministic persistent allocation in C0.2. C01-T01, C02-T01, M0-T03 |
| ADR-004 | ACCEPTED | Ordered validated command commits avoid scheduling-dependent outcomes | Mutating batch preflight or arrival-order priority | Read-only preflight plus live revalidation and atomic effects. C01-T02/C01-T03, C03-T01, M0-T06 |
| ADR-005 | ACCEPTED | Systemic rules drive gameplay and NPCs. AI assistance stays in development | Runtime LLMs/model APIs/image generation | Runtime graph audit and offline tests. C01-T06, M0-T14, V0 asset evidence |
| ADR-006 | ACCEPTED | Eastern Fantasy first over generic pools and actions where consumed | Build a second ruleset in parallel | One technique demonstrates the rule direction. M1-T01 |
| ADR-007 | PROVISIONAL | Pausable real-time is the M1 interaction prototype | Irrevocable turn-based commitment or bounded turn-based comparison | Select interaction after response/playability study. I0-T02 and M1-T03 |
| ADR-008 | PROVISIONAL | Compare exact 10/20/30 host steps per wall second. 10 Hz is the first candidate | Treat 33 ms as exact 30 Hz or couple host speed to game days | Same scenario conversion across cadence tests. Measure CPU, response and state equivalence. M1-T03 |
| ADR-009 | ACCEPTED | Named NPCs stay canonical across LOD. Simulation alone selects processing mode | Camera-triggered modes or entity-to-domain transfer | Pure projection teardown. Deterministic mode triggers and exact initial accounting. C02-T02, I0-T01, M0-T04/M0-T05 |
| ADR-010 | DEFERRED | Anonymous cohorts add complexity beyond the 20-actor M0 workload | Introduce cohorts for M0 | Consider only if a future 200+ actor benchmark warrants it. M0-T05 keeps named actor accounting |
| ADR-011 | PROVISIONAL | JSON definitions plus a typed registry are a content-spike candidate | Custom scripting or unconditional JSON Schema dependency | Sample ten definitions and validate invalid cases/offline behavior. M0-T09 |
| ADR-012 | ACCEPTED | C0 excludes deep inheritance and scripting to limit schema and runtime complexity | Inherited scriptable content | Typed cases required. Shallow cycle fixture is conditional on adoption. M0-T09 |
| ADR-013 | ACCEPTED | The physical ledger is the source of truth before markets | Inventory inferred from market statistics or duplicated in reservations | One owner, atomic transactions and derived availability. C03-T01, M0-T06/M0-T07/M0-T08 |
| ADR-014 | PROVISIONAL | Top-down symbolic 2D with 16×16 candidate tiles is the V0 experiment | Fixed full-screen canvas plus sidebar | Select after physical viewport, keyboard and legibility work. V0-T01/V0-T02/V0-T03/V0-T08/V0-T11 |
| ADR-015 | PROVISIONAL | Native Bevy TilemapChunk is the first candidate | Add third-party tilemap before measuring | Exercise the 256×256 map and dirty chunks. V0-T09 |
| ADR-016 | PROVISIONAL | Compose portraits from authored layers to preserve identity | Runtime image generation or unique hand-authoring for every NPC | Use fixed seeds, clipping/collision review and overlay checks. V0-T06/V0-T07 |
| ADR-017 | ACCEPTED | Versioned domain snapshots plus boundary-indexed command log support authoritative resume | Persist Bevy ECS or replay curated story alone | C0.2 starts external checkpoint I/O and preserves pending commands plus cursor/digest. C02-T01, M0-T02/M0-T03/M0-T10/M0-T11 |
| ADR-018 | ACCEPTED | Tiny authored/seeded region ahead of full worldgen makes causality reviewable | Simulate planet and history first | Review scenario expectations before results. M0-T01/M0-T13/M0-T15 |
| ADR-019 | ACCEPTED | Keep whole-game runtimes out of the graph | Direct Veloren/CDDA/Azgaar runtime adoption | Source provenance review and locked-graph check before adopting isolated dependencies. C01-T06, M0-T14 |
| ADR-020 | ACCEPTED | M0 headless correctness and M1 human playability have separate evidence | Infer playability from a build or replay | M0 matrix and separate M1-T01/T02/T03 |
| ADR-021 | ACCEPTED | C0.1 is a single pure Rust library with one synthetic conflict fixture | Scaffold the target workspace or implement production fairness now | Key is (target tick, actor ID, producer ID, sequence) only for this fixture. C01-T01/C01-T02/C01-T03/C01-T06 |
| ADR-022 | ACCEPTED | Versioned fixture bytes, named ChaCha12 probes and SHA-256 event chaining make replay independently testable | Hash Rust memory, use default hashing/RNG, or save all event bytes in each checkpoint | Canonical future-state inventory and prefix digest tested in C01-T04/C01-T05. C02-T01 verifies loaded prefix continuity |
| ADR-023 | PROVISIONAL | Rust 1.100.0, rand_chacha 0.10.0 and sha2 0.11.1 are implementation candidates with published API support | Custom production cryptographic primitives or selecting a mutable default RNG | Verify crate APIs, lock graph, compilation and published vectors before adopting the pins. C01-T04/C01-T06 |
| ADR-024 | ACCEPTED | Logical ticks are authoritative; scenarios that measure days supply positive ticks_per_game_day | Hardcode one ticks/day constant or infer days from real-time cadence | Checked scenario horizon and pinned replay input. C01-T01, M0-T02/M0-T14, I0-T02, M1-T03 |

## Rejected alternatives

| ID | Status | Alternative and consequence | Affected tests |
| --- | --- | --- | --- |
| ALT-01 | REJECTED | Bevy ECS ownership or two canonical copies would create competing state and break neutral projection rebuilds | C01-T06, I0-T01 |
| ALT-02 | REJECTED | Frame-time-driven mutation would tie rule outcomes to refresh schedules | I0-T02, M0-T12 |
| ALT-03 | REJECTED | A permanent turn-based choice before playtest would preempt ADR-007. A bounded prototype remains available for comparison | M1-T03 |
| ALT-04 | REJECTED | Full procedural planet/history and a many-crate framework ahead of the first fixture would exceed early scope | C01-T06, M0-T15 |
| ALT-05 | REJECTED | Runtime LLM NPCs and model-generated runtime visual assets violate the runtime constraint | C01-T06, M0-T14, V0-T06 |
| ALT-06 | REJECTED | Raw narrative JSON has no executable rule contract. A future typed content conversion would need diagnostics | M0-T09, C01-T06 |

## Amendments recorded on 17 November 2026

| Record | Prior wording and change | Compatibility and evidence |
| --- | --- | --- |
| ADR-001 scope | The accepted stack remains. Bevy is a client dependency at V0, while C0.1 locks its own pure Rust graph | No client import or version migration. C01-T06 and V0 client smoke |
| ADR-002/004/009 ownership | Clarify read-only preflight, canonical admitted queue, and simulation-only LOD mode triggers. Projection teardown no longer flushes commands | Earlier flush wording was unspecified. C01-T02/T03, C02-T02 and I0-T01 cover the corrected contract |
| ADR-008 timing | Clarify exact host rates and remove the rounded 33 ms description. Preserve the provisional status | M1-T03 measures cadence with constant scenario time |
| ADR-017 replay | Make queued inputs and prefix digest plus cursor mandatory in the first saved domain schema | No existing save files to migrate. C02-T01 and M0-T02/T03/T11 own compatibility |
| ADR-021 through ADR-024 | Add the approved C0.1, encoding and time amendments. Record dependency versions as provisional | No code or manifest changes. C01 and downstream tests above provide future evidence |

For later changes, record date, owner, choice, considered alternatives, evidence, migration consequences, affected documents and affected tests. A successful compile alone cannot promote a provisional entry.
