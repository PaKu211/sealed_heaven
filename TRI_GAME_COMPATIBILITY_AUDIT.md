# Tri-game compatibility audit

**Branch:** `research/tri-game-integration-spikes`  
**Accepted C0.5 commit:** `e4a9b9704b1b119b4d03eb2bb70c45c2f1d2468c` (`fix(c05): admit each intent once per frame`)  
**Research base:** `8c7ada0fa54b1b402cf0bc54c345cea1d253dd37` (descendant of that commit; working tree was clean before this research)  
**Date:** 2026-11-18  
**Host:** Windows 10.0.26200, 12th Gen Intel Core i7-12700F, 20 logical processors, rustc 1.100.0

This audit compares ideas from Of Tales and Deeds, Dwarf Fortress, and Songs of Syx with the frozen Sealed Heaven kernel. It does not treat the blueprint's formulas, claimed frame rates, or Python suites as evidence. Those suites were not run as acceptance tests. No proprietary game source was copied.

## Baseline that was preserved

| Contract | What this research did |
|---|---|
| 4096-tick prototype horizon | Unchanged. No new horizon was introduced. |
| One-way occupancy | Unchanged. Experiments do not move kernel actors. |
| Canonical save/restore | Unchanged. Logistics uses `Simulation::restore_domain_checkpoint` and `drive`. |
| Deterministic replay | Unchanged. The 20-actor final checkpoint hash matches the published C0.5 row. |
| Player-agency semantics | Unchanged. `src/`, `Cargo.toml`, `Cargo.lock`, specs, and playable sources were not edited. |

`tools/c05_verify.ps1` completed with exit 0 on `8c7ada0` before the research files were added. The generated `C0_5_VERIFICATION_LOG.txt` was restored afterward so this branch does not rewrite that C0.5 log.

## Where the current systems live

| Concern | Location | Research treatment |
|---|---|---|
| Tick, commands, digest | `src/lib.rs` `Simulation::drive` | Called, not replaced. |
| NPC goals, wake interval 4, ledger | `src/fixture3.rs` | Called through public restore/drive. Decision kind `6` is counted from returned events. |
| Site routes and 3×3 grid search | `src/fixture3/navigation.rs` (private) | Not patched. Experiment B is a separate tile search with the same 4-direction order. |
| Player aggregate | `src/player5.rs` `World::settlement` | Timed only for the authored 20 NPCs. |
| Presentation cadence | C0.4/C0.5 reports: nominal 2 Hz | Left in place. The experiment calendar is derived from that tick. |
| Render frame | Bevy client | Not remeasured in this session. |

## Idea-by-idea compatibility

| Source idea | Fits the frozen kernel? | Finding |
|---|---|---|
| 1000 AP energy queue | No, if it becomes the world clock | Safe only as a future combat queue. Speed 0 must not divide. The published "skip the scan" claim is false for the Rust sketch: it still walks every actor. |
| 1440-minute daily routine | Yes, as a derived clock | `minute = floor(tick / 2) mod 1440` at the accepted 2 Hz cadence. It is not a second Bevy tick. |
| 140 Z-level geology | No | Irrelevant to the xianxia village slice and far outside the 3×3 fixture grid. |
| Staged world seeds | Not tested | Conceptually compatible with separate RNG streams. No world generator was built. |
| Hierarchical pathfinding | Not as a drop-in | Production navigation is a site graph plus a 3×3 grid. A regional search helps only on maps this game does not have yet. |
| Odd/even agent updates | No | Splitting actors across frames would change who wakes on a tick. |
| Room push/pull logistics | Yes, beside the ledger | A desk may remember demand and the kernel task id. Quantities stay in the C0.3 checkpoint. |
| Java intrusive pools / zero-GC claims | No | Rust already has no garbage collector. The claim is not an architecture we can adopt. |
| City-size exponent 2.65 | No | An unvalidated curve from another game. Not implemented. |

## Identity check

The prototypes use a village, a storehouse, a shrine, farmers, a smith, a disciple, and spirit herbs. They do not import dwarven geology, a 40,000-citizen city model, or a 59-subsystem life simulation. Sealed Heaven remains one Eastern fantasy sandbox with one deterministic tick.
