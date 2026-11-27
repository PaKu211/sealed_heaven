# C4.3 — Exact named-actor scheduling and navigation

## Decision and scope

Baseline C4.2 commit d5216c50de7fa29c81d398e8070257d6f42f0d76. Implements the production L0/L1 foundation under docs/SIMULATION_LOD_SPEC.md, ADR-009 and M0-T05. No approximation is enabled. The same authored 12 NPCs, expanded map, economy, social/knowledge systems and single canonical clock are retained. Revision 30 is distinct because mode and wake metadata become canonical. No content or dependency changes.

## Schedule and execution

Each named NPC has a persistent scheduler entry keyed by its existing ActorId, with mode Active, Scheduled or Dead, next full wake tick, deterministic evaluation counters and mode transition count. At the immutable pre-NPC-plan snapshot, healthy nonlocal livelihood roles (supplier, traveler, grower, courier, frontier carrier/grower) may qualify for L1. The policy uses Manhattan distance greater than 10 from the canonical player and excludes hall teachers, perceived players, injury, HP deficits, suspended ambitions, companion work, martial/meridian proposals, reporting/testimony, disputed/claimed lots, duties, contracts, audits and any other exceptional proposal. Watch and Medic roles are always active; all other unsupported roles are active by default. The certificate computes the exact remaining routine/rest/camp-rest decision from current stock knowledge, fatigue and time, using the same livelihood selector, score, reason and priority tie-break as the full planner. If the certificate fails or the goal changes, full evaluation happens immediately. A full evaluation also occurs at least every four ticks for a continuously scheduled actor. Thus the scheduler reduces calls to the general planner; it does not claim to skip all computations for three turns.

All plans are taken before movement and effects. Apply mode/wake updates before ID-ordered consequential commits; preserve the existing traffic revalidation and one-move rule. No deferred physical action, resource reservation, death, care, injury, quest, event, RNG or information delivery; the ordinary domain tick still executes every consequential transition. Dead actors retain their original identity and cargo records, but their schedule is retired with next_wake=0. State is bounded by number of named actors. Scheduler modes describe the latest planning snapshot; after movement a new local activation is evaluated at the next planning boundary.

## Exact route occupancy

Dynamic path queries for revision 30 construct an ephemeral occupancy bitmap from current living player/NPC positions, then use the existing NESW breadth-first search with identical walkability, reach, shortest path, and tie-break. Every query rebuilds the snapshot from current authoritative state, so prior movement, swaps or death cannot leave stale occupancy. Static fallback and courtesy remain unchanged. The bitmap is not persisted. This replaces per-visited-cell linear actor scanning with O(actors + visited cells) occupancy work rather than O(actors * visited cells). Actual wall time depends on workload. Synthetic scale tests use 64x64 grid occupancy populations; they are not a large-world or 2,000-NPC social simulation.

## Persistence and projection

New explicit --c43 launch, checkpoint.c43, rules/state/scenario digest domains and fixed scheduler payload ordered by ActorId. Scheduler is reconstructed from seed and canonical journal, then verified through the state digest. Revision 1–29 historical hashes and content packs remain unchanged. No automatic migration. The presentation never chooses mode or wake. Debug-only ASCII 'lod' prints the canonical entries and evaluation totals without advancing time. Normal players see the same gameplay as C4.2, including F2 survey and F3 dated news. Run sealed_agency.exe --c43 [--seed N] [--pixel | --debug-world].

## Acceptance criteria

- Exact reference: compare complete World equality and state digest each turn after projecting the new rules/metadata back to revision 29; six seeds x 2,000 passive turns, four seeds x 1,000 mixed commands, plus player journeys between regions and interactions. Verify actual mode transitions, conserved inventories, in-flight freight, histories, outcomes, save/load and deterministic counters. Historical C4.2 goldens independently protect the reference from shared-source regressions.
- Certificate fatigue boundary and interruption cases; schedule retirement idempotent; altered future wake state changes digest and cannot pass journal restoration.
- Fixed v1 golden CSV: 6 initial checkpoints, 6 tick-1000 checkpoints and 6 terminal state digests with scheduler counts. Capture from the reviewed candidate; freeze before later phase-order cleanup.
- Dense synthetic occupancy: exact path results against linear scan for 20/100/500/1000/2000 occupied positions. Measured wall-clock timing is descriptive; no timing-sensitive test assertion.
- 1,000 Bevy ECS projection rebuilds/teardowns with exact checkpoint invariance, followed by identical future turns. Separate native GUI startup smoke. These are specific proofs, not a complete I0/M0 acceptance claim.
- Strict launch, isolated save slots and bilingual developer inspection; all historical fixtures, Rust debug/release and Windows executable replay gates. Evidence outside the frozen source records actual results.

## Explicit limits

Only L0/L1 exact planning foundation is implemented. No L2/L3/L4 aggregates, anonymous cohorts, coarse travel, time fast-forward, dynamic population expansion, larger map, longer horizon, procedural prehistory or new lore import. The current 65,536-turn cap is unchanged. L1 intentionally still evaluates a lightweight exact guard every tick, and worst-case actors may remain fully active. This is not a claim of full-scale M0 or commercial readiness. Windows-only as requested, no GitHub push, no independent human audit or human playtest.
