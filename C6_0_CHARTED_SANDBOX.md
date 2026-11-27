# C6.0 - Integrated charted sandbox

Baseline: C5.9 7aeac8ccb6a391d29156a221efecafe3196b0638. Revision 47 (ChartedSandbox), separate checkpoint.c60. The existing campaign simulation and content are retained. No new dependencies, forced story path, resource grants, or migration of older save files.

## Canonical geography

The 54 x 15 campaign map uses a fixed 13-word chart bitset plus the last observed position. Living players learn visible static cells at the initial observation and after player/NPC movement barriers. Opaque edges may be charted from a visible cardinal neighbor, matching the existing map observation rule. Re-observation at the same position is skipped because terrain is immutable. The bitset and observation position are included in the canonical state digest and reconstructed by replay. Charted landmarks are shrine, clinic, patch, market, garden, hall, depot, and grove. Unobserved landmarks cannot be selected as destinations. Remembered terrain does not retain mobile NPCs, corpses, caches, local supplies, or current political conditions.

Route guidance is a deterministic shortest path (N/E/S/W tie order) over charted, walkable cells, to within one tile of a known landmark. A route does not know hidden occupancy, reserve a cell, cross unobserved terrain, or guarantee an unobstructed step. Each follow action delegates to one ordinary movement command. Arrival or no known path is read-only. A chosen waypoint is presentation state, cleared on load, reset, or seed change; the chart persists in the checkpoint.

## Field guide

Six tabs: local actions, travel, training, commitments, journal, system. Entries are based on own state, known records and current visible/reachable actors and sites. They delegate to existing commands, not a second economy, quest engine, or action validator. Attempts can still fail for lack of stock, cost, schedules, refusal, or other authoritative constraints. Explicit actor IDs and menu selection prevent accidental default targeting among multiple adjacent NPCs. Known requests offer their existing accept/decline and other available commands, as well as a route to the discovered request site. Ordinary typed commands remain available. The guide does not promise a menu item for every historical diagnostic or advanced command.

ASCII: field [local|travel|training|commitments|journal|system]; choose N; map; route SITE; route clear; go; close. Page views do not commit simulation turns. Gameplay entries have their ordinary intent consequences; save/load are file operations.

Pixel: fixed 1100 x 720 window for C6.0, world viewport with current sight and dim remembered terrain, contextual opportunity list, route guidance and status. Enter opens local menu; Left/Right switch tabs; Up/Down select; PageUp/PageDown jump menu pages; Enter executes the explicitly selected entry; Esc closes. Journal reader supports pagination, PageDown opens the full last notice. Modal navigation consumes no game turns and blocks movement/combat shortcuts. Tab switches EN/ID. Right Shift+Space attempts one charted movement step; arrows move normally outside the menu. Previous F8, F5 save and F9 load bindings remain. The atlas shows static geography only. Legacy revisions keep their old interface.

## Integration evidence

The seed-3 vertical slice uses real background choice, request acceptance, gathering, shrine delivery, breath practice, Willowford trade, a one-herb accord with the Stonebank envoy, enrollment at Reed Hall, Pineford exploration and gathering, and a mapped return. The fixture includes initial and other career states. It was constructed with developer observations to select a valid test path, then replayed in normal EN/ID clients. No scripted rewards or debug mutations are used. The route is a reproducible test, not a claim that every seed or unguided player follows the same experience.

Acceptance: 5 domain and 3 integration tests (12,000-tick equivalence, 12,000 mixed ticks, complete 65,536-tick horizon and checkpoint restore), journal/modal controls, localization validation, 12 bilingual golden states, legacy compatibility, pinned offline build/test gates, packaged executable verification and native window smoke. Evidence files record the actual acceptance results. Automated validation is not a human playtest, independent audit, or broad hardware/resolution certification.
