# Tri-game integration decisions

None of these experiments is accepted gameplay. They are evidence for a later review. The frozen C0.1–C0.5 contracts stay as they are.

## ADOPT

Nothing from this spike is adopted into the kernel, the client, or the golden vectors.

## ADAPT

### Derived daily calendar, not a second tick

**Justification.** The accepted client already has one 2 Hz tick and a separate render frame. A calendar minute is just `floor(tick / 2) mod 1440`. Twenty NPCs produce 180 observable phase changes per in-game day. Replaying 10,000 ticks through a restore at tick 4,096 matches the uninterrupted trace. The roster does not call `drive`.

**Evidence.** `evidence/experiments.txt`: 41 µs to advance 20 NPCs for 10,000 ticks. Kernel checkpoint bytes stay put.

**Integration.** Small if it remains a presentation read model: map the existing tick onto Sleep, Eat, Work, Socialize, and Travel, then point the current NPCs at places the client can draw. Large if someone tries to replace goal selection.

**Compatibility.** None while it stays outside the kernel. Wiring it into goal selection would change golden decisions and is out of scope.

**Gameplay.** This is the change a player could actually see. The kernel wake every 4 ticks has no morning or evening.

**Risk.** 05:00–06:00 is not specified by the source note; this spike leaves the NPC in bed. Roles are a temporary rotation, not authored biographies. Travel is one site per minute on a five-site line, not the frozen navigator.

### Demand desk on the existing ledger

**Justification.** Storehouse to shrine completes inside the C0.3 haul: 12 herbs stay 12, the shrine receives 5, five kernel tasks are recorded, and a repeated observation does not create a second reservation. An unknown-actor command leaves the checkpoint unchanged. Replay matches.

**Evidence.** Experiment C, 19 ticks, `logistics_checkpoint_equal: true`.

**Integration.** Moderate. The desk should sit beside task selection and keep using haul completion on the canonical checkpoint. It must not grow its own stock map.

**Compatibility.** The tested world is an authored checkpoint with the frozen scenario block. It does not alter `demo.c03`.

**Gameplay.** A shrine that asks for herbs, and a hauler who is not duplicated, is readable sandbox behavior.

**Risk.** The kernel haul moves one unit per task and only actor-owned goods. A true room inventory, partial loads, and interrupted trips still need a reviewed design. This spike does not prove multi-hauler contention.

## DEFER

### Hierarchical navigation

**Justification.** On the 8×8 map nearest the current game, regional search is slower (2.03 µs versus 1.93 µs) and visits the same cells. The selective-adoption rule says to keep C0.3 navigation when the demo-scale gain is under 20%. On 64×64 and 128×128, with zero disconnected queries, regional search is about 4.4× and 9.0× faster. That benefit belongs to a future map, not to the 3×3 fixture.

**Evidence.** Experiment B table. Agreement is 500/500 on every large map. The 256×256 mean is partly 227 instant component rejects and is not the headline number. Rebuilding after a 10% obstacle edit costs milliseconds and drops almost all cached paths.

**Integration.** High. The real navigator is private, deterministic, and covered by golden movement. A replacement has to match those paths or version the save format.

**Compatibility.** A different corridor would change travel time and therefore task due ticks.

**Gameplay.** None on the current village. Real once regions are larger than a single screen of tiles.

**Risk.** Adopting it now adds cache invalidation bugs without a map that needs them.

### Energy queue for combat

**Justification.** The safe version refuses speed 0 and breaks ties deterministically. It is still a linear scan, contrary to the blueprint's wording. Combat is not part of C0.1–C0.5.

**Evidence.** Unit test only. No tick benchmark, because it is not in the tick.

**Risk.** Using it as the world clock would create a second simulation of time.

### Staged xianxia world generation

**Justification.** Separate RNG streams are a sound way to keep topography stable when herbs change. No generator was implemented or measured, so there is no evidence to adopt.

**Risk.** Treating the blueprint's lapse-rate constants or cave depths as Sealed Heaven canon.

## REJECT

| Proposal | Why |
|---|---|
| Replace the canonical scheduler with the energy queue or with daily activities | The tick, digest, and golden decisions are the accepted simulation. |
| A second simulation tick inside Bevy | The three clocks stay independent only if there is one simulation tick. |
| Odd/even updating of NPCs | It changes which actors commit on a tick. |
| 140 Z-level geology | Weight and theme do not fit this slice. |
| Copying Java object pools to "avoid GC" | Rust does not have that runtime. |
| Demographic exponent 2.65 | An arbitrary curve from another game, not reproduced here. |
| Installing hierarchical search into production navigation now | Demo-scale measurement does not clear the 20% bar. |

## Blueprint claims that remain unverified

40,000 agents at 60 FPS, a 4-level 768×768 hierarchy, and the Python reference tests were not reproduced. This session's largest kernel world is 1,000 actors on the existing tiny topology, at 56 unpaced ticks/second, with a p95 of 46.8 ms. That is a fixture measurement, not a population target.
