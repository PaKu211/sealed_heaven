# C1.0 Architecture Decision

The current rules, boundary contract and checkpoint compatibility are documented in [the GDD implementation pass](C1_1_GDD_IMPLEMENTATION.md).

## Decision

Build the living-world domain alongside C0.9 and use it as the default ASCII campaign. Keep the frozen C0.1–C0.9 domains and their checkpoint readers available through their existing clients.

| Approach | Assessment |
| --- | --- |
| A. Extend the existing world | C0.3 has deterministic goals, travel, and resource transactions. C0.5 wraps those NPCs, C0.9 fixes the witness and recipient to specific identities, and the C0.7 host advances its own hauler world and save file. Adding dynamic witnesses and stock-aware goals to this arrangement would leave canonical decisions split across boundaries. |
| B. Refactor the existing internals | Unifying the old fixture, player wrapper, and host supply would require changing the behavior that validates historical checkpoint bytes and digests. It would add risk to compatibility work that C1.0 does not need to touch. |
| C. Add a living-world domain | One world owns identities, positions, activities, stocks, combat, and knowledge. Reuse the actor ID, position and direction types, SHA-256 encoder, terminal clock, and atomic file operations. Port the playable gathering and shrine haul into the new timeline. Keep old campaigns available independently. |

A turn delivers due reports and regrows herbs, resolves one player action and the witnesses at that moment, computes all NPC goals from the resulting snapshot, commits NPC actions in actor-ID order, then queues conversations. Commit checks protect contested cells, medicine, and cargo. A reporter who cannot see the recipient travels to their usual post or home, and sends the report only after making contact. The engine has no presentation clock or external service. Qinghe map, staff, routes and social contacts live in a typed scenario with no per-name branches in the rules.

Revision 1 of `SH:C10:WORLD` stores a scenario fingerprint, one fixed-width intent per tick, a digest of the complete reconstructed state, and an envelope checksum. Restore replays the normal turn pipeline. The digest covers activity timing and progress, needs, health, the stock ledger, incidents, individual receipts, directed relationships, pending reports, and the last player action result. Receipts record applied regard changes. Altered history cannot be accepted as the original state merely by updating the checksum.

C1.0 saves to `checkpoint.c10`. The older domain can still open `checkpoint.c05` and its separate `shrine-supply.c03` through `--c09` or `--c08`, as appropriate. Legacy saves have no cross-domain conversion. Their events and witness records lack the C1.0 positions, custody, and contacts needed for a faithful migration. Start a C1.0 campaign independently or continue the prior campaign with its original rules.
