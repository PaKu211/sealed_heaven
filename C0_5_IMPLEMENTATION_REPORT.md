# C0.5 implementation report — verification in progress

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

Parent full verification passed: root93 tests+5 benchmark tests+2 doctests in
debug/release; app33 tests in each profile (all old26 retained), both formatting,
check/strict Clippy, canonical packers, debug/release binary builds. Log complete.
Native gameplay and independent core/client reviews are pending. See handoff.
These are not a milestone PASS. Report exact verified commits at delivery.
