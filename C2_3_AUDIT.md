# C2.3 audit

Reviewed against the C2.3 boundaries: no new reputation score, no telepathy, no quest chain, no legal system, no kernel rewrite.

Corrections made before the recorded verification:

- Player reports must not index the NPC table with actor 21. Delivery uses participant helpers.
- Personally examined evidence stays one hop. Relayed gossip becomes two hops and cannot open a dispute.
- Automatic witnessing requires reach and sight, and only on revision 14, so a C2.2 campaign does not gain player knowledge.
- Clippy: collapsed a nested provenance check, replaced `get` plus `is_none` with `contains_key`, and dropped an unused pattern field.

Findings left as limitations, not silent behavior:

- Journal restore replays intents from the initial world. An audit inserted by the test fixture is outside the journal, so `World::restore` correctly returns `StateMismatch` for that fixture. Pending player testimony is still in the digest: two copies that apply the same testimony produce the same checkpoint, and forging knowledge on a clean stepped campaign makes restore fail. This is the same event-sourced rule as C2.2.
- `Rejection::Empty` is shared with gathering. Examining a tile with no finding says the supply is empty.
- Natural contract audits do not appear in a short wait. The divergence test shares one audited failure, then diverges only by player intents.
- No Windows run and no human playtest were performed.

No global leak was added. Hop-2 testimony does not open a dispute. A restored dispute does not delete the failure event. NPC contract, movement, and delivery code still run when the player does nothing. Resource quantities are untouched by testimony.
