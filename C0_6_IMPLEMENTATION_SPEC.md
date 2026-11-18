# C0.6 — Readable Living Village

**Base:** `main` `496045841cedaf0f485ebef4c9e123b3354c6760`  
**Accepted C0.5:** `e4a9b9704b1b119b4d03eb2bb70c45c2f1d2468c` (production sources match; docs/evidence only differ)  
**Branch:** `feat/c06-readable-village`

## Research verdict (not adopted as code)

The Tri-Game roster kept its own resident sites. That state can disagree with the kernel after restore, and its Sleep/Eat labels are not kernel goals. C0.6 therefore:

- Derives `minute_of_day = floor(tick / 2) mod 1440` and the day number from the canonical tick only.
- Shows that calendar block as a **schedule**, with an explicit note that it is not the simulation.
- Shows NPC position, needs, task id, and task goal from the existing checkpoint projection (`View`).
- Does not move NPCs, assign work, or write the world from the clock.

Hierarchical navigation and shrine logistics stay out of this milestone. Performance context, unchanged: 20-NPC unpaced compute is about 0.21 ms against the 500 ms presentation tick.

## Scope

Presentation and read models only. Frozen command semantics, checkpoints, golden vectors, occupancy, and the 4096-tick horizon stay as they are.

### A. Readable world

Distinguish the player, NPCs, the herb patch, the shrine, the three canonical NPC sites, and a walkable lane. Flat colored sprites and labels. No new crates.

### B. World clock

Display time, day, and day/night from the formula above. Night is before 06:00 and from 18:00. The tint changes `ClearColor` only.

### C. NPC visibility

On-screen position follows `World::npc_position`. Color and the inspector show the checkpoint task goal and needs. The schedule line is separate.

### D–E. Feedback

Keep the existing rejection and inventory copy. Add the clock, the schedule disclaimer, and the actual goal next to the target the player selected.
