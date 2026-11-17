# V0 Visual Foundation Acceptance Tests

Version 0.2-draft · 17 November 2026

## Setup

Use a coherent pixel tileset, one seeded 64×64 smoke map and a separate 256×256 map for scrolling and edit profiling. Show a handful of actors, two resource states, a keyboard inspector and one authored portrait-layer set. Freeze asset/fixture revisions, seed and workload before collecting results. Feed immutable mock snapshots/deltas using the version/recovery semantics in [Technical Architecture §5](TECHNICAL_ARCHITECTURE.md#5-bevy-materialization-and-presentation-contract). I0 checks those semantics against the real simulation.

For layout, record physical usable client dimensions, display DPI/scale, viewport, UI bounds, zoom and tile area for 1366×768 and 1920×1080 targets. At V0 setup, the visual owner registers the smallest usable visible tile area and native-resolution UI legibility check after an exploratory measurement. Approve them before the acceptance run.

## Visual and UX matrix

| ID | Experiment | Pass criterion |
| --- | --- | --- |
| V0-T01 | Render 16×16 candidate tiles at the two physical target sizes | Integer scaling with nearest sampling. No unintended blur, seams or half-pixel wobble in recorded stills and pans |
| V0-T02 | Pan/zoom across tiles and sprites; check HUD text | Grid alignment holds and native-resolution text satisfies the recorded legibility check |
| V0-T03 | Show map and keyboard inspector together | Recorded usable tile area meets the registered minimum with UI shown and no clipping after display scaling/chrome |
| V0-T04 | Rebuild same tile chunk from the same seed, coordinates and content | Same variants, colors and overlays |
| V0-T05 | Change one authored resource fixture at (8,8). Replace the full resource overlay with its harvested state and the dependent east-edge layer at (9,8) with its depleted variant | The exact changed set is those two tile/layer pairs. All other pairs stay equal. Re-rendering alone changes none |
| V0-T06 | Compose portraits for actor IDs and corresponding appearance seeds 1–100 on pinned assets/palette. Rebuild the set. Check clipping and illegal layer pairs against authored masks/rules | Equal same-input images, zero clipping/illegal layer pairs, and a reviewed fixed contact sheet. Count identical-image collisions and visually confusing pairs using the approved V0 recognition rubric and collision budget recorded before the run |
| V0-T07 | Change equipment and job separately | Face remains byte-identical. Only the declared equipment overlay can change in the equipment case. Job change leaves the face intact |
| V0-T08 | With visible focus on the map, select actor by keyboard, open inspector, traverse one listed field/control and return to the map | Every step is possible without a mouse. Focus is visible after each transition and returns to the actor/map as specified in the recorded script |
| V0-T09 | Scroll the 256×256 map and edit a fixed set of dirty chunks | Record named edit types, p50/p95 frame time and edit cost. Compare a plugin only if the native candidate fails a required behavior or the recorded workload justifies comparison |
| V0-T10 | Drop and rebuild mock projections, deliver a stale delta and skip a revision | Mock input remains byte-identical. Exactly one projection per visible mocked actor, stale output is ignored, and a snapshot restores current mock presentation. I0-T01 owns the real canonical checkpoint/replay test |
| V0-T11 | Review cues on authored sprites and terrain | Required distinctions are visible by silhouette, icon or text as well as any hue |
| V0-T12 | Run the pinned workload on low-end hardware when accessible | Record hardware, driver, resolution, scaling, platform, build, p50/p95 frame time in milliseconds, peak memory and GPU behavior. State whether the 4 GB target runs the workload. Record unavailable hardware as a skip. 60 FPS remains aspirational and 30 FPS remains a provisional floor pending measurement |

## Art review and decision record

Review actual atlas frames, fixed portrait contact sheet, screenshots at native target zoom, collision/occlusion silhouettes and nearest-sampled output. AI-generated concepts supply no tile/compositor evidence. Log asset production time. The V0 owner records the canvas/layout outcome, the measured tilemap decision and portrait recognition rubric/budget before requesting any ADR promotion. V0 establishes visual and mock recovery evidence. The [I0 bridge test](M0_ACCEPTANCE_TESTS.md#6-owned-downstream-fixtures) supplies real canonical evidence.
