# Art Direction and V0 Visual Production

Version 0.2-draft · 17 November 2026

## 1. Identity and rejected direction

Target: symbolic, information-rich, top-down 2D roguelike pixel art. Reference the readability and restrained asset scope of Of Tales and Deeds and Caves of Qud, while designing original Eastern Fantasy silhouettes and palettes. The prior detailed AI-generated sect-courtyard image is rejected as a production reference; it overstates available art labor and does not imply a reusable tile system.

Priority order: gameplay clarity > asset consistency > procedural variation > decoration. Never copy recognizable game sprites, palette arrangement, UI elements, sounds or layouts.

## 2. Pixel grid and camera

Initial world tile proposal 16x16 pixels, integer scaled, nearest filtering, orthographic top-down. A tall character may use a 16x24 sprite with explicit anchor and overlap. Characters need clear silhouette from terrain and buildings. Local terrain is a 2D grid; multi-z gameplay is deferred.

Fit an integer-scaled world canvas inside the measured map panel, with separate native-resolution UI. The 480×352 canvas (30×22 tiles) at 2×/3× is an exploratory candidate for 1366×768 and 1920×1080. At 3× it uses 1056 vertical pixels before chrome or a HUD. V0 records actual usable pixels and DPI, then validates the selected viewport and tile count. See [V0 layout criteria](V0_ACCEPTANCE_TESTS.md#setup).

## 3. Tile/sprite language

- Terrain first: ground, water, paths, walls, doors, bamboo/trees, rocks, resource nodes, storage and simple roof/overhang silhouettes.
- Use consistent outline/value contrast and reserved accent colors for interaction or spiritual energy.
- Color cannot be the only cue for hostile/friendly, injuries, faction or resource richness; include shape/icon/text.
- Limit early animation to movement (minimal frames), hit/contact flash, one work interaction, one simple aura and UI highlights.
- Ground tiles use authored base tiles plus deterministic variants/edge transitions; worldgen produces terrain state, graphics selects its presentation.
- World view should remain legible even with all particle effects disabled.

## 4. Procedural terrain visuals

Input: canonical tile terrain/biome/season/state + tile coordinates + appearance seed + content pack version. Output: authored atlas tile index, tint and overlay choices. Use a stable coordinate hash or named RNG, not a global random stream consumed each render. A camera pan or reload must not change a tree or stone variant. Tile appearance changes when canonical world state legitimately changes (e.g., harvested resource, season), not when a chunk is re-rendered.

## 5. Actors and portraits

World sprites are small and symbolic; portraits carry individual face detail. Initial portrait candidate 64x64 (also compare 48x48) layered handcrafted base face, eyes, brows, mouth/nose, hair, facial hair, skin palette, collar/robe, headwear and optional scar/mark. Generate layer choices from stable ActorId+appearance seed, cache composed image and invalidate equipment overlay only when equipment changes. Job changes must not regenerate the face. Avoid runtime diffusion or external model APIs.

Start with a carefully drawn small layer library; validate 100 combinations for clipping, similarity and readability before making hundreds of assets. Explicit layer z-order, anchors, palette-index contracts and invalid-combination rules are required.

## 6. UI and interaction

World map receives most screen space. Compact status strip, contextual inspector and event/message log are distinct from debug tools. Character, inventory, jobs, relationships, faction and map screens can open as keyboard-navigable overlays; all core actions accessible without mouse. Rebindable input after basic navigation is implemented. Text uses crisp readable native-resolution font, not necessarily tiny pixel font.

Developer inspector can reveal canonical ActorId, needs, task, utility scores, reservations, position, LOD, RNG stream, resource ledger and state hash. Gameplay must respect information the player could know.

## 7. Bevy implementation proposal

Use built-in Sprite/texture atlases, ImagePlugin nearest sampling, pixel-grid render-to-texture approach and TilemapChunk for initial renderer. Bevy built-in examples: https://bevy.org/examples/2d-rendering/pixel-grid-snap/ and https://bevy.org/examples/2d-rendering/tilemap-chunk/. Compare bevy_ecs_tilemap only if built-in tilemap lacks required feature or worsens measured update complexity.

V0 consumes immutable mock snapshots and deltas. At I0 the same revision and recovery semantics apply to real read-only presentation data. Visual output cannot change a game rule. [Bridge contract](TECHNICAL_ARCHITECTURE.md#5-bevy-materialization-and-presentation-contract)

## 8. V0 asset and workload limit

Prototype pack: 1 coherent limited-palette biome; approximately 12–24 foundational terrain/object tiles, a handful of animated actor silhouettes, a few item overlays, one UI layout, one small portrait-layer set and one effect. The aim is to prove readability and reuse, not complete a commercial art set.

## 9. Acceptance

See [V0 acceptance tests](V0_ACCEPTANCE_TESTS.md). Art direction advances only when real sprite/tile compositing at actual zoom is reviewed. AI-generated illustrations may be used privately to discuss mood, but cannot pass tile consistency or asset-pipeline gates.
