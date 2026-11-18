# Tri-game experiment results

Prototypes live in `research/tri_game` and link to `sealed_sim` as a library. They do not register a Bevy system and they do not replace `Simulation::drive`.

Tests: `cargo test --offline` in `research/tri_game` — 6 passed.  
Clippy: `cargo clippy --offline --all-targets -- -D warnings` — clean.  
Release experiment command: `.\target\release\tri_game.exe experiments`  
Evidence: `research/tri_game/evidence/experiments.txt`

## Experiment A — daily schedule

Calendar minute is `floor(tick / 2) mod 1440`. Two ticks per minute matches the accepted 2 Hz presentation clock: one real-time second is one in-game minute when the client actually holds 2 Hz. Render frames are not involved.

| Minute of day | Activity | Place |
|---|---|---|
| 00:00–06:00 and 21:00–24:00 | Sleep | Home (site 1). 05:00–06:00 has no named block in the spec, so the NPC stays at the bed. |
| 06:00–07:00 and 12:00–13:00 | Eat | Home |
| 07:00–12:00 and 13:00–17:00 | Work | Field, smithy, pavilion, storehouse, or home, by role |
| 17:00–21:00 | Socialize | Teahouse (site 5) |
| While the NPC is not at that place | Travel | One site step per minute toward the destination |

Roles rotate Farmer, Smith, Disciple, Hauler, Villager from the NPC id. That is an experiment roster, not a kernel goal.

| Check | Result |
|---|---|
| 10,000-tick trace, then restore at tick 4096 and continue | Same SHA-256 as the uninterrupted trace |
| Kernel checkpoint around the roster | Byte-identical before and after; the roster never calls `drive` |
| 20 NPCs, one in-game day | 180 activity changes (9 per NPC) |
| Cost of advancing 20 NPCs for 10,000 ticks | 41 µs in the release binary |
| Visible in the current client | No. The playable scene still shows the kernel goals. The 180 changes are headless observations. |

The kernel, by contrast, wakes each NPC every 4 ticks with no time of day. A player watching the client cannot tell morning from night. The roster makes that distinction trivial to log. It does not yet make it visible.

The energy queue was tested only as a side structure: a speed of 0 returns no actor and is never a divisor; two equal speeds break ties by lower entity id. It still scans the actor list. It was not wired to the tick.

## Experiment B — hierarchical search

The tile search uses the kernel's neighbor order (north, east, south, west) and a FIFO queue. The regional search adds an 8×8 region graph, rejects a query when flood-fill component ids differ, refines inside the region corridor, and falls back to the tile search if that corridor fails. Cache invalidation rebuilds components after blocking 10 percent of open cells, then drops cached paths that now cross a blocked cell.

Seed `0x005EA1ED`. Both searches must reach the same goal or both must fail.

| Map | Queries | Tile mean | Regional mean | Tile p95 | Regional p95 | Mean visits, tile / regional | Agreement | Fallbacks | Component rejects | Rebuild |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 8×8 | 50 | 1.93 µs | 2.03 µs | 3.80 µs | 3.80 µs | 12.66 / 12.66 | 50 | 0 | 0 | 4.5 µs (2 cells) |
| 64×64 | 500 | 210.0 µs | 47.4 µs | 399.9 µs | 85.2 µs | 1659.58 / 370.01 | 500 | 1 | 0 | 355 µs (339 cells) |
| 128×128 | 500 | 930.4 µs | 103.0 µs | 1.723 ms | 190.9 µs | 7225.15 / 838.93 | 500 | 1 | 0 | 1.577 ms (1416 cells) |
| 256×256 | 500 | 2.799 ms | 82.3 µs | 4.056 ms | 276.2 µs | 21177.65 / 654.98 | 500 | 0 | 227 | 6.016 ms (5781 cells) |

On 64 and 128, no query was disconnected, so the speedup is not an artifact of instant rejection. Regional search is about 4.4× and 9.0× faster there, with about 78% and 89% less mean time. On 8×8, which is the scale closest to the current game, regional search is slightly slower and visits the same cells. On 256, 227 of 500 pairs were disconnected; those queries return before the tile search, so that row's mean must not be quoted as a pure search speedup.

A full rebuild after blocking 10% of open cells costs more than one regional query, and almost every cached 256×256 path was dropped (2 of the successful paths remained). A door-sized edit was not measured separately. Dynamic obstacles are affordable only if invalidation stays local. This experiment does not prove that.

Worker counts 1 and 4, partitioning the same 32×32 queries and merging by query index, produce the same digest. The kernel has no worker-count parameter. Parallel kernel execution is **NOT TESTED** and was not added.

The production navigator was not swapped in. The frozen grid remains 3×3.

## Experiment C — storehouse to shrine

One hauler, home site 2 (the shrine), 12 spirit herbs (definition 7) in personal stock, and an empty shrine stock of the same definition. The kernel's existing haul goal moves one herb per task onto the home stock. The desk only records demand 5 and the current kernel task id. It does not store a second quantity.

| Check | Result |
|---|---|
| Herbs at the shrine | 5 |
| Sum of definition-7 quantities on every tick | 12 |
| Distinct kernel tasks recorded | 5 |
| Duplicate observations of an open task | 17, with no second reservation created by the desk |
| Ticks until the fifth delivery and a clear task | 19 |
| Replay from the initial checkpoint | Final checkpoint bytes match |
| `drive` with an unknown actor | Error, and the live checkpoint is unchanged |

Reservation integrity is the kernel's own hold, observed through the canonical checkpoint. The desk does not cancel or complete tasks itself. Save/restore equivalence is the replay above, not a new codec.

## Failures during the spike

The first shrine restore, zero-speed energy case, ledger cursor, and schedule byte restore were covered by the six tests after the compile fixes. No acceptance test in the root crate was weakened or edited. No experimental assertion was relaxed to obtain a pass.
