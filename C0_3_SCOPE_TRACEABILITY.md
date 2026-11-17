# C0.3 Scope Traceability

Version 1.1-proposal · 17 November 2026  
Status: architect reconciliation. This file does not amend the README, Core specification, Technical Architecture, or the C0.3 draft. It does not authorize implementation. Refinement 1.2 corrects transfer order, event sequences, and task cardinality inside the same exit. It does not remove a retained row. Refinement 1.2 corrects transfer order, event sequences, and task cardinality inside the same exit. It does not remove a retained row.

Internal subphases may order the work. A subphase does not satisfy the C0.3 exit.

## 1. How ownership was decided

A requirement stays in C0.3 when an approved document assigns it to C0.3 or places it in the C0.3 exit column. A joint label `C0.3/M0` keeps the mechanism in C0.3 and leaves the 30-day scenario proof on M0. A feature leaves C0.3 only when an approved document already names a later gate, or when the project owner records a scope amendment. This reconciliation does not record that amendment.

GDD §5 says to avoid a GOAP framework in C0. That limits the planner. It does not remove needs or jobs from the C0.3 deliverable.

## 2. Milestone ownership

| Capability | Approved assignment | C0.3 ownership after reconciliation | Exit evidence that remains |
| --- | --- | --- | --- |
| Inventory and one-owner ledger | Core §7, ADR-013, README deliverable | Retained in full | C03-T01 conservation and rollback |
| Resource transactions and recipes | Core §7, M0-T07, C03-T01 | Retained in full | C03-T01 failed later leg, and the M0-T07 trace |
| Reservation lifecycle | Core §7, C03-T01 | Retained in full | Acquire, expiry, cancel, disable, one release |
| Production jobs eat, rest, work, haul, return home | Core §6 `C0.3/M0`; README "jobs"; GDD §5 names the same list as M0 minimal goals | Mechanism retained in C0.3. The 30-day emergent proof stays M0-T13 | A scheduled selection among those five goals, with the Core stage list |
| Socialize and react-to-threat | Core §6 optional extensions | Not an exit item | None at C0.3 |
| Actor needs and DecisionTrace | README deliverable; Core preamble; Core §6; TA §2 | Retained. Numeric weights are not in the approved text and cannot ship until the owner accepts a table | Decision on a scheduled tick, persisted trace fields named by Core §6 |
| Navigation fixture | TA §2 and §10; M0-T08 unreachable targets; Core §8 requires successor order when the first fixture exists | Retained. Core §8's two-site region remains the M0 world. C0.3 still owns the first fixture | Reachable move, equal-cost tie, unreachable failure |
| Local 2D grid | Core §8, as the initial local search method | Retained with the navigation fixture. Flow fields and nearest-neighbour acceleration stay behind profiling, as Core §8 already says | One grid search with a fixed successor order |
| M0-T06 | README exit "applicable"; C03-T01 "corresponding"; the row itself is a 20-NPC contention that includes the C03-T01 case | Retained as written. This is not a silent reduction to two claimants | 20 actors, fewer beds, tools, food, and workstations, plus the one-unit order case |
| M0-T07 | Same exit language | Retained as written | Actor → store → cargo → site, declared recipe, then an invalid later leg |
| M0-T08 | Same exit language | Retained as written | Cancel, disable, expiry, unreachable, repeat release, replayed completion key |
| C03-T01 | Core §7 and M0 §6 | Retained in full | The ledger fixture whose keys the owner must freeze |
| Automatic mode policy | LOD §5, ADR-009, M0-T05 | Deferred by those documents | Not a C0.3 exit item |
| Coarse travel approximation | LOD §5, ADR-009 | Deferred by those documents | Exact ticks only |
| Anonymous cohorts | ADR-010 | Deferred | None |
| `ticks_per_game_day` and 30 days | Core §3 for scenarios that measure days; ADR-024; M0 §1 | Deferred for this gate. No retained C0.3 row measures days | M0-T01, M0-T02 |
| First save migration | C0.2 spec; M0-T11 | Deferred. A new schema is not a migration of revision 1 | Old blobs keep their current errors |
| Typed content JSON | TA §3; M0-T09 | Deferred | None at C0.3 |
| Bevy, host cadence, presentation | TA §5; V0; I0 | Deferred | None in `sealed_sim` |
| R2 and timing budgets | Core §1; M0-T14 | Deferred | R1 only |

## 3. What "applicable" and "corresponding" contain

README exit text:

> C03-T01 and the applicable M0-T06 through M0-T08 fixtures

C03-T01 text:

> Run the corresponding M0-T06/T07/T08 cases before promoting C0.3.

The approved rows are:

| ID | Procedure that the exit still includes |
| --- | --- |
| M0-T06 | 20 NPCs compete for fewer beds, tools, food, and workstations. The C03-T01 one-unit ordering case is included. Expected winner and typed rejection. No double reservation or consumption |
| M0-T07 | Transfer actor → store → cargo → site. Execute declared recipes. A later invalid leg leaves balances, reservations, and success events unchanged |
| M0-T08 | Cancel, disable, or expire on the completion boundary. Unreachable targets fail. A repeated release and a replayed completion key happen at most once, with a deterministic reason |

No approved sentence removes the 20-NPC procedure, the four-location trace, or unreachable targets from that exit. Dropping any of them is an owner scope amendment. This proposal does not make that amendment.

The 30-day horizon, two factions, and the relation delta of −1 belong to M0 §1, M0-T01, and M0 §5. They are not sentences of M0-T06, M0-T07, or M0-T08. They stay on M0 without an amendment.

## 4. Frozen behavior that C0.3 work must not enter

| Surface | Required treatment |
| --- | --- |
| Fixture 1 commands, `SH:C01:EVENT:v1`, C0.1 goldens | Unchanged |
| `SH:C02:STATE` revision 1 and every published blob | Unchanged. No new field and no new revision of that tag |
| Fixture 2 disabled synthetic completion | Unchanged. Production disablement does not rewrite it |
| C0.2 enabled flags | Remain immutable after construction |
| C0.1 `Outcome` | Gains no production variant |
| `Cargo.lock` baseline | Unchanged until the owner separately approves a crate. This proposal adds none |

The C0.2 sentence "C0.3 replaces this synthetic completion" is reconciled as forward production behavior in a new domain. It is not authority to edit fixture 2. The owner confirms that reading in `C0_3_OWNER_DECISIONS_PENDING.md`. The archive stays frozen either way.

## 5. Proposed internal order

These stages are sequencing only. The gate is the whole table in §2.

| Stage | Work | Why it is not an exit |
| --- | --- | --- |
| S1 | Schema, ledger, contention, cancel, disable, expiry, exactly-once | Needs, jobs, navigation, and the three M0 rows would still be open |
| S2 | Needs, five goals, stage machine | Navigation and M0-T06 through M0-T08 would still be open |
| S3 | Graph, grid, unreachable failure, four-location trace, 20-actor contention | This is the first stage that can meet the written exit |

## 6. Amendment boundary

The owner may narrow the exit. Until that decision is recorded, the specification revision that follows this proposal has to include every retained row. An implementer does not get to treat S1 as C0.3 complete.

## 7. Refinement 1.2

R1.2 keeps every retained exit row. M0-T07 remains Actor → Store → Cargo → Site. Each actor still has one current task in the 20-NPC case, and that task holds the contended reservations. Those rules are recommendations in the pending-decision file and are not approved.
