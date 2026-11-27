# C4.1 - Continuous regional expansion

## Decision and scope

Standalone Rust/Bevy game. Baseline C4.0 commit 3da46a11e35c8a368eabadebe642a6984a3d17ae. C4.1 is the next incremental GDD Stage D milestone selected during implementation. Pineford and its two NPCs are new authored provisional content, not an assertion that the complete source lore has been imported.

The canonical scenario is 54 x 15 (810 tile slots). The existing 30 x 9 generated region is copied exactly, including all four tile fields. The unused six western rows are closed; expansion is eastward, not an unrestricted rectangle or a new simulation clock. The second river is a continuous north-south barrier, with two crossings separated by at least five rows, feeder roads, and connected eastern settlement cells. Both original and additional geography use stable deterministic integer streams. The generated map validates all actor anchors and connected walkable cells. New settlement terrain is deliberately simple; no claim of full urban architecture.

## Authored overlay and identity

The original v1 content pack and its SHA-256 are unchanged. The new content/expansion/v1/pineford.json is strictly deserialized, typed, validated against the base actor identities, bounds and economy, and compiled into static data. Canonical overlay fingerprint: b768996f73e24a6a41c227fb7c27f9377e60df724994c71b817dc202bcc8af5a. Build rejects any changed canonical overlay fingerprint. No runtime JSON parsing in the turn loop. Revision-aware content identity exposes both the base registry and the additional overlay. Five extra definitions (settlement, grove, depot, two actors) are resolved only for revision 28. The old --content-info manifest intentionally remains byte-identical.

Li Wen (wire ID 37) is a freight carrier; Han Yu (38) is a grower. Both are actual entries in the existing world NPC namespace and are initialized through the same social, martial, meridian, injury, character and traffic systems. Their livelihood goals enter the existing deterministic plan/commit loop. Carrier rests in place after 24 fatigue rather than returning home every time; more urgent threat and recovery goals may interrupt. No new actor clock or hidden duplicate actor. Names and roles are authored, not procedurally invented per encounter.

## Physical economy

Pineford grove initial/capacity 12 herbs; depot initial 6, target 8, freight reserve 2, initial treasury 20 coins. Local Qi controls grove regrowth cadence (10, 14, 18 or 22 turns). Han Yu harvests at most two per trip and physically delivers to depot. Li Wen takes at most two from surplus depot stock while Willowford has demand (market target + 2 buffer), walks the actual map and unloads on arrival. Loaded shipments remain deliverable if the destination stock changes en route. The institutional supply transfer is in kind, not a paid NPC trade; player buy/sell uses the depot's finite treasury and physical inventory. Existing downstream courier and Qinghe demand run on the same tick. No claim of item-by-item source provenance after fungible herbs mix in the market.

Player uses existing gather/buy/sell at the local grove/depot. Normal buy/sell 2/1 coins; scarce buy/sell 4/3, based on stock at/below reserve. Market closes at night. No free materials, money creation, or long-distance trades. Cargo is Npc::herbs; no separate duplicated freight inventory. Dead or wrong-role actors and stale load proposals cannot commit freight. Corpse cargo uses the existing take/recovery system. Stock, coin and delivery counters are covered by the canonical state digest.

## Persistence and play

Revision 28 has separate checkpoint.c41 slot. Rules 1-27 keep their original scenario, wire record, state digests and launch options. The scenario fingerprint binds generator version, both content hashes, authored scenario and expanded geography. Save reconstructs the map from seed before replaying all committed intents; old saves are never automatically migrated.

    sealed_agency.exe --c41 --seed 42
    sealed_agency.exe --c41 --pixel --seed 42
    sealed_agency.exe --c41 --debug-world --seed 42
    sealed_agency.exe --expansion-info 42

ASCII: help, survey, seed N before the first turn, gather, buy, sell, save/load; Pixel: F2 survey, Tab EN/ID, WASD, Space wait, F5/F9. Follow the road east from Willowford and cross the second river. Camera follows both axes, with a 17 x 9 visible viewport. Nearby market/grove stock is shown only when visible (developer view is explicitly exempt). The terrain projection reads the actual canonical collision map, not the base scenario's template floor. Diagnostic full-map output is developer-only and read-only.

## Acceptance design and limits

- 2,051 seeds including u32 boundaries, exact base-tile preservation, connected topology, varied eastern rivers, crossings and cadences.
- Six seeds x 2,000 passive turns: every live position valid and unique; actor displacement <= one tile; loaded carrier physically crosses river; actual depot stock and market delivery; herbs/coins conserved; exact save/restore.
- Four round trips through both rivers: local buy/sell/gather and Qinghe delivery, restore/resume and full fresh replay.
- Targeted stale/dead carrier, reserve, reach, insufficient coins and duplicate loot tests; pack schema, role and ID collision rejection; revision admission matrix and tampered map/seed/state/revision.
- Golden CSV captures first successful implementation's 12 map and 12 checkpoint hashes; subsequent changes checked against these. Separate Python reference reproduces map/overlay identity; not an independent human audit.
- ASCII executable tests exercise real trips in both languages; native Pixel projection/viewport tests, legacy replay, full Windows debug/release quality gates and extracted distribution smoke.

Remaining scope: larger geographical regions, multiple climates, procedural history, dynamic terrain/site generation, LOD, large-population benchmark, full content import and human playtest/independent audit. C4.1 does not claim these. Windows only as requested; no Linux or hosted CI and no GitHub push.
