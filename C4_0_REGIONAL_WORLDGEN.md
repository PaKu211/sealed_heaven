# C4.0 — Seeded regional world generation, generator v1

## Scope and decision record
Standalone Rust/Bevy game. C3.7 baseline ab9f2fc7e6e62466797fb9a14c0f251d6f120f9a. The master GDD sections 7 and 12.4 and RAG reconciliation sections 19–20 call for deterministic, versioned regional generation, ecology, authored sites, roads, causal trade and stable actor identity. No numbered C4.0 specification existed, so this milestone deliberately chooses a bounded first playable generator, not the whole Stage D or the full lore import.

The playable Riverlands grid remains 30 by 9. Qinghe and Willowford use their authored v1 settlement footprints, stable sites and actor definitions (10 NPCs). The corridor between x=11 and x=21 is procedurally generated. Revision 27 inherits revision 26's living systems, including institutional contracts, autonomous sect progression, injury, social memory and material-backed trade. The superseded relief/faction-agency experiment states remain disabled just as in revision 26. No new clock, actor duplicates, off-screen approximation or arbitrary scripting.

## Deterministic stages
1. Geography: quantized elevation 0–100; bilinear integer lattice interpolation. A SplitMix64 finalizer hashes coordinate and seed, with explicit wrapping and separate domain constants.
2. Hydrology: bounded meandering north–south river, x=15..17; consecutive rows differ by at most one. Water blocks movement and line of sight.
3. Ecology: quantized moisture and Qi 0–100. Humidity and Qi have separate coordinate domains, moisture is biased toward the river. Forest/meadow are cosmetic passable terrain; elevation produces rock obstacles.
4. Roads: two crossing rows separated by at least three rows; roads carve a continuous walkable connection between settlement gates and bridge river cells. Roads do not have a movement speed modifier in v1. A bounded reachability repair closes disconnected non-authored pockets. Validate every site, NPC spawn/home/route and relic lot; all walkable tiles must share the entrance component.
5. World digest: explicit byte tags and integer LE encoding, authored content SHA-256, fields, river, crossings, road distance, regrowth cadence and stage digests. Each stage digest has a distinct domain and binds generator version plus seed.

Qi changes real production: the west herb patch regrows every 8/12/16/20 turns; eastern garden every 12/16/20/24, depending on local density. The existing stock caps and material conservation ledger remain unchanged. One regrowth unit is credited only when the stock is below capacity on the scheduled tick. NPC pathfinding, traffic courtesy, line of sight, player collision and both renderers consume the generated map.

## Save and version contract
Revisions 1–26 retain authored maps and exact legacy checkpoints. Revision 27 is opt-in and has its own checkpoint.c40 slot; no automatic legacy migration. The legacy 5-byte intent record and bounded envelope remain unchanged. Revision 27's scenario fingerprint binds generator version 1, exact content v1 SHA-256, authored scenario/rules digest and generated geography digest. The seed is already in the envelope; restore reconstructs the immutable map and rejects mismatched identity before replay. Mutable resource changes are recovered by the existing canonical journal, not by resetting an already played world. Future terrain alteration, broader content versions and generator changes require explicit versioning/migration.

## Play and inspect
    sealed_agency.exe --c40
    sealed_agency.exe --c40 --seed 42
    sealed_agency.exe --c40 --pixel --seed 42
    sealed_agency.exe --c40 --debug-world --seed 42
    sealed_agency.exe --worldgen-info 42

ASCII: seed N before first turn, survey for current tile's terrain/elevation/moisture/Qi; herb cadence is visible only near the patch or garden. Survey is presentation-only. Standard controls still apply. Pixel: F2 survey; Tab language; WASD movement; Space wait; F5 save; F9 load. The full map is developer-only. ~ river, % bridge, : road, ; woodland, , meadow, # wall/rock. The original launch flags (no C4.0 flag) intentionally retain revision 26; distribution launchers select the new campaign.

## Acceptance
- Generate 4099 seeds including boundary u32 values, validate topology, immutable authored anchors and deterministic identity; test stream isolation.
- Freeze six seeds' five stage digests plus checkpoint hashes at 0/1000 in tests/fixtures/living/c40_generator_v1.csv. Golden capture was performed using this first implementation; independently coded Python reference verifies all 30 stage hashes (and executable map data when supplied). It is not an independent human audit.
- Four playable journeys to Willowford and back, actual purchase and delivery; 9000 passive turns across six seeds with physical courier crossings, correct scheduled regrowth, stock conservation, no overlapping living actors, and restore equality.
- Recompute checksum after hostile seed/scenario/revision edits and require rejection; old saves remain readable. Expand public admission matrix to revision 27.
- New localized ASCII/GUI survey and executable slot/manifest tests; retain 294 C3.6 golden checkpoint comparisons and all historic tests. Validate and package on pinned Windows x64 Rust 1.100.0. Report actual results outside frozen source.

## Explicit limits
No claim of a large procedural continent, physical hydrology, multiple climates, dynamic terrain changes, authored lore conversion, region transitions, prehistory, LOD, simulation scale, human playtest or Linux/hosted CI validation. The first generator is a compact playable bridge from the existing vertical slice toward those future systems.
