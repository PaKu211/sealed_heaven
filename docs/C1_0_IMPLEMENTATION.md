# C1.0 Living World Foundation

For the post-audit build, see [the GDD implementation pass](C1_1_GDD_IMPLEMENTATION.md).

**IMPLEMENTED — PENDING COMPILATION AND RUNTIME VERIFICATION**

- Qinghe has four autonomous NPCs. Each turn they select from fatigue, care, work, patrol, retaliation, retreat, and reporting goals. Their positions, activity start times, route progress, health, and cargo persist. Waiting advances the settlement while the player is alive or dead.
- Player strikes create immutable incidents. Witnesses need line of sight to both participants. Trusted NPC contacts can pass dated reports in person. Each recipient learns and reacts once per incident. Report provenance is retained and limited to two hops.
- The herbalist carries herbs from the patch to the shrine. The healer consumes shrine stock to treat injuries and can refuse care based on learned violence. Combat, medicine use, player deliveries, herb regrowth, and NPC duties share one turn clock.
- Tests cover the authored Qinghe route, both witness outcomes, goal preemption, guard damage, report provenance and deduplication, a pending-report save, stock conservation, state equality on replay, forged checkpoints, player death, and terminal save/load.

## Terminal entry point

Run outside the cloud implementation environment.

```sh
cargo run --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --bin sealed_agency
```

In an opening C1.0 campaign, move east eight times. The authored regression test compares two attacks on the outrider with the same two attacks after three additional waits. The herb carrier's position and wall visibility differ at strike time. Wait for reports to travel and approach the healer for care. The terminal accepts `f 22` to attack the outrider, `e 1` to ask the healer, and `v` and `o` to save and restore. The test encodes the timing, which still needs a native run.

## Verification and limits

- Static review covered command admission, phase ordering, interaction revalidation, bounded counters, canonical wire layout, and the legacy/new CLI routes. Rust compilation, tests, performance, and playability remain unverified.
- C1.0's authored roster is separate from the full C0.3 fixture. The older graphical client still uses the legacy domain and is available as the `sealed_playable` binary. C1.0 uses the terminal and English presentation.
- Revision 1 supports 65,536 turns. Restoring replays from turn zero. Worst-case load time, prolonged congestion at the gate, and balancing across fatigue, report deadlines and clinic demand need runtime measurements.
- Positions and identity persist after death. The player can continue advancing time through waits. A successor or reincarnation mechanic is outside this milestone.
