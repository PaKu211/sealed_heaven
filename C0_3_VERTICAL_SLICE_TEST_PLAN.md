# C0.3 Vertical Slice Test Plan

Version R2.1-final-approved · 17 November 2026  
Status: plan. AM-01 through AM-08 are approved and normative. Tests become required only after an independent G3 re-review passes and the owner authorizes the named slice. No gameplay hash is an oracle yet.

Canonical sources: `docs/C0_3_IMPLEMENTATION_SPEC.md` version 1.1-approved, `docs/C0_3_ACCEPTANCE_TESTS.md` version 1.1-approved, and the C0.1 and C0.2 acceptance files for regression.

Pyramid, bottom to top: codec and schema, state machine, reservation and transaction invariants, save and restore, replay, boundary permutation, multi-actor integration, C0.1 and C0.2 regression. A golden is packed from the approved contract by a packer that is not the Rust encoder. The Rust tests compare against that pack. Implementation output is not promoted to an oracle inside the same change.

These slices are not C0.3 exits. The milestone exit is S4 plus the frozen regression suites.

## S1 — Resource core

**Contracts:** OD-01 through OD-05, OD-08, OD-10, OD-14, OD-15, OD-16, plus AM-01, AM-02, AM-03, AM-04, AM-05, and AM-07. All approved. **BLOCKED** on independent G3 PASS and an owner authorization that names S1.

**Modules, after authorization:** new fixture-3 world, ledger, and task files. Frozen fixture 1 and 2 paths stay untouched.

**API:** `drive` only. `step` returns `UseDrive` and does not mutate. Producer 1. Errors follow the AM-05 table.

**Tests:** C03-T01a through C03-T01f and C03-T01h. Reservation id 3. Work emits `Executing` after the rejection. The two-task oracle debits 1 beside another task's reservation of 1 and rejects a debit of 2. Cancel, disable, and expiry each emit kind 3, kind 7, then kind 5. A second release emits nothing. Replay of a role-1 or role-2 key with the same submitted task id emits nothing. The same key with a different submitted id, including `CompleteTask(0)` versus `CompleteTask(N)`, is `ConflictingIdentity`.

**Vectors:** event bytes for those sequences, packed independently after the schema is frozen. Checkpoint bytes include the 41-byte consumed registry records. Hashes wait for AM-02 and AM-03.

**Save and restore:** save with a non-empty unconsumed list that stores submitted and resolved ids. Resume moves every key into the registry and creates one transaction.

**Rollback:** the quantity-2 debit, and a two-leg transaction whose first leg would succeed and whose later owner id is 0. Tick, cursor, registry, and stock match the pre-call bytes.

**Regression:** full C0.1 and C0.2 suites, including fixture 2 disabled synthetic completion.

**Entry:** G3 PASS on the amended specification, then an owner authorization that names S1.

**Exit:** the S1 tests pass in debug and release. This is not the C0.3 milestone exit.

**Incomplete afterward:** needs, travel, the four-location chain, and 20 actors.

## S2 — First living NPC

**Contracts:** S1 plus OD-09 and AM-06. All approved. **BLOCKED** on S1 exit and an owner authorization that names S2.

**Modules:** needs and decision trace beside the ledger.

**API:** decision runs only when `next_wake_tick` equals the tick and the actor was not enabled from 0 earlier in that tick.

**Tests:** hunger 1000 and the other channels 0. Kind 1, kind 7 `Executing`, kind 6 with eat score 1000000 and four zeros. Wake becomes tick + 4. Work does not run on the creation tick. Later ticks emit `Completing`, then kind 4, kind 3 reason 8, and kind 7 `Done`. Restore before the new wake emits no second kind 6.

**Vectors:** that decision boundary and the completion pair. The trace contains five candidates.

**Save and restore:** split after the decision and before `Done`.

**Rollback:** an `i64` score overflow returns `CapacityExceeded` and emits nothing.

**Regression:** S1 plus C0.1 and C0.2.

**Entry:** S1 exit and an owner authorization that names S2.

**Exit:** one actor can eat and resume. The milestone stays open.

**Incomplete afterward:** rest, work, haul, return home, navigation, M0-T06, and M0-T07.

## S3 — Movement and production

**Contracts:** S2 plus OD-11 and AM-08. OC-EDGE is rejected and not required. **BLOCKED** on S2 exit and an owner authorization that names S3.

**Modules:** graph and grid search in fixture 3. No pathfinding crate.

**API:** position remains the actor record. A stored path is not recomputed. Locus 3 is not relocated inside a search.

**Tests:** edge 1 arrives on the work tick after it is stored. Site 9 fails. The grid path is north, north, east, east. Cancel mid-edge clears `task_id`, leaves locus 3, and residual travel arrives next tick with no event, including when the actor is disabled. Rest and return home have no reservations. Work reserves the lowest tool and workstation and does not debit them. Haul is one debit and one credit at home. M0-T07 uses the ten-tick table: task 405 debits stock 14 then credits owner 0; tick 10 is later-leg `InvalidLeg`.

**Vectors:** those event sequences. No cost-2 checkpoint unless the owner approves OC-EDGE.

**Save and restore:** mid-path grid index, edge-1 start, and post-cancel residual mid-edge with `task_id` 0. Restore does not search and does not finish the edge early.

**Rollback:** M0-T07 tick 10 matches the tick-9 bytes, including stock 14 quantity 1.

**Regression:** S1 and S2 plus C0.1 and C0.2.

**Entry:** S2 exit and an owner authorization that names S3.

**Exit:** the five goals and M0-T07 run. The 20-actor world is still absent. This is not the milestone exit.

**Incomplete afterward:** M0-T06 and the single combined regression.

## S4 — Multi-NPC integration

**Contracts:** all approved OD rows and all approved AM rows. **BLOCKED** on S3 exit and an owner authorization that names S4.

**Modules:** the same fixture 3, with 20 actors. No new package.

**API:** command-key order remains the contention order.

**Tests:** M0-T06 from `next_record_id` 21. Reservations 21 through 40. Winners 1 through 5 then enter `Executing`. The twenty `Queued` tasks with `due_tick` 0 do not fail the boundary. C03-T01a through C03-T01i. M0-T08 uses reason 1 when all three causes exist. Grouped `drive` matches single boundaries.

**Vectors:** the 20-actor event stream and the final checkpoint, packed independently.

**Save and restore:** one split with a future command and an in-flight reservation, matching Core §11.

**Rollback:** one over-reserve inside the 20-actor batch fails the boundary with no partial reservations.

**Regression:** the whole pyramid, including C0.1 and C0.2 goldens.

**Entry:** S3 exit and an owner authorization that names S4.

**Exit:** this is the first slice that can satisfy the C0.3 milestone exit. A passing S4 is evidence for the owner. It is not itself the independent sign-off of the implementation.

**Incomplete afterward:** nothing inside the C0.3 exit. Day length, migration, Bevy, and C0.4 stay out.
