# C0.5 implementation report — implementation verified; acceptance pending

Accepted base: `f35bfd3664d4baf2feaf78755b1a18d29cccf795`.
Development branch: `feat/c05-player-agency`. No final acceptance yet.

## Changes

- New `sealed_sim::player5::World`: sole aggregate authority, embeds the unchanged
  NPC Simulation and owns player21, terrain/range rules, finite typed herb ledger,
  command registry, event chain, revision-1 aggregate checkpoint and validation.
- Existing `Simulation`/C01–C04 checkpoint semantics and vectors unchanged. Root
  dependencies/lock/toolchain unchanged. Existing lib receives only module export.
- New `sealed_agency` binary in the existing isolated Bevy package; old C04 binary
  remains. Read-only world rendering, input stamping, bounded queue, context menu,
  player inventory/status, target selection, camera following, time controls.
- Separate C05 save path and candidate validation/atomic sibling replacement;
  no implicit C04 import. English/Indonesian overlay preserves old labels.
- Reproducible headless benchmark for actual 20/100/1000-NPC worlds; C05 aggregate
  supports 20 NPCs only and explicitly declines unsupported scaled configurations.

## Explicit decisions / limits

Owner approved one-way NPC occupancy after clarification: player cannot enter an
occupied NPC/object cell, but accepted NPC movement may later overlap the player.
NPC site positions are abstract/shared; art offsets never define collision.
The shrine is a new physical receiving inventory, not a duplicate of C03 stock.
Herbs are finite3; one delivery completes the request. No rewards/combat/quest
framework/new NPC AI. At most one new player action per tick and 4096 retained
command identities per slice timeline. Input queue32 is transient, not admitted.

## Verification state

Final full verification passed: root 100 tests (1 maximum-horizon latency probe
explicitly ignored), 5 benchmark tests, and 2 doctests in debug/release; app 41
tests per profile, formatting, checks, strict Clippy, canonical packers and
debug/release binary builds. Native foreground gameplay completed and final
headless/native measurements are documented. Independent client source review
passed after fixes. Core remediation adds exact frozen-NPC replay with a bounded
4096-tick horizon and the three original regressions; the independent public-API
probe also passes 8/8 in debug/release, including exhausted-cursor atomic failure.
CLIENT-04 button-label click-through was fixed and exercised in the latest native
run, but not independently reviewed. **A distinct independent core/client delta
re-review is still required**, and final independent acceptance is not
available. Therefore C0.5 remains NOT ACCEPTED despite successful implementation
and tests. See handoff and reports for limitations and exact evidence.
