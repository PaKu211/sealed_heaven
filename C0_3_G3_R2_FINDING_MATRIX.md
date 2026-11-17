# C0.3 G3 R2.1 Finding Matrix

Version R2.1-final · 17 November 2026  
Status: reconciliation. No finding is closed in the normative specification by this file.

Source audit: `C0_3_G3_INDEPENDENT_AUDIT.md`, verdict FAIL. Approved decisions remain OD-01 through OD-16. H0 stays `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c`.

R2.1-final supersedes R2.1 only for the later-leg oracle, residual edge travel, and submitted-payload identity. Creation-tick skip, one record cursor, five goals, and the approved graph are not reopened.

| ID | Severity | Root cause | R2.1-final correction | Approval | Remaining risk |
| --- | --- | --- | --- | --- | --- |
| F01 | BLOCKER | Acceptance applied legs on `created_tick` | Keep the skip. M0-T07 is the ten-tick table. Task 405 fails on its later leg after debiting stock 14 | AM-01 BLOCKED | Published acceptance still has the old same-tick table |
| F02 | BLOCKER | One flag collapsed sequences. R2.1 stored only resolved id, so `CompleteTask(0)` and `CompleteTask(401)` could falsely match | Store submitted and resolved ids. Identical submitted replay is a no-op. Different submitted id is `ConflictingIdentity` | AM-02 BLOCKED | Registry width is 41 bytes. No gameplay hash yet |
| F03 | BLOCKER | Legs lacked definition/class. Stock 13 was reused after depletion | Available quantity excludes other tasks' reservations. Task 405 debits stock 14, then fails the invalid credit | AM-03 BLOCKED | A first-leg-only failure would miss the later-leg requirement |
| F04 | HIGH | Impossible reservation ids under one cursor | One cursor. Cross-table collisions fail restore. C03-T01a starts at 3. M0-T06 starts at 21 | AM-04 BLOCKED | Published reservation-1 payloads change |
| F05 | HIGH | Replay, payload conflict, and late target unordered | First-match table matches on submitted payload id. Producer 0 stays illegal | AM-05 BLOCKED | Fixture 1 and 2 keep their frozen precedence |
| F06 | HIGH | Planner under-specified | Five plans with eligibility, selection, due, expiry, terminal path, and save point | AM-06 BLOCKED | Weights stay fixture configuration |
| F07 | HIGH | `Queued`, overdue, and tick-10 interaction incomplete | `Queued` exempt from overdue. Tick 9 creates task 405. Tick 10 later-leg rolls back | AM-07 BLOCKED | C03-T01a includes the `Executing` event |
| F08 | HIGH | Mid-edge progress and cleared `task_id` left no recovery rule | Residual work-phase travel spends remaining cost. No teleport. Approved graph unchanged | AM-08 BLOCKED. OC-EDGE optional | `traversed_cost > 0` needs OC-EDGE |
| F09 | HIGH | Incomplete oracles | Acceptance reconciliation names initial state, stock 14 later-leg, residual travel, and submitted-payload replay | Follows AM-01–AM-08 | Not normative until copied after approval |

Fixture 2 synthetic work stays. OD-16 reading of fixture phases is unchanged.
