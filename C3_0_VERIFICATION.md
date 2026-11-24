# C3.0 — Sect Cultivation: Keep, Use, or Sell

Revision 20 (`checkpoint.c30` default). `--c29` preserves revision 19; prior campaigns and frozen C0.1 kernel unchanged. No push or merge.

## Gameplay

The player can now **keep** a finite physical relic rather than sell it. A carried SpiritOre lot can be `attune <lot>`d into two one-hit wards; a carried JadeScript lot becomes two `channel <npc>` strikes with sight and a two-tile range. The same lot cannot be attuned or sold twice: the identity remains in the material ledger with `Consumed(player)` custody and an observed `Attuned` event. A ward spends one charge to reduce real incoming damage by one; an invalid attack does not spend a jade charge. `cultivation` displays remaining charges; `lots` hides consumed goods and `lotlog` shows only witnessed transfers. Revision 20 adds an accessible JadeScript site; revision 19 retains its original three-lot baseline.

Public-command acceptance route from seed zero: move east five turns, `claim 3`, `attune 3`, east three, `claim 0`, `attune 0`; moving toward a live target and `channel 32` hits at a range where ordinary `attack 32` is rejected. The alternative from the same unmodified campaign carries ore across the road and uses `offerlot 36 0`: the steward gains the ore, the player receives coins **but no ward**. A passive branch still has the autonomous faction outcomes from C2.9. The target's incidental courier occupation does not participate in the relic economy; no new courier or herb exceptions were added. Actual headless ASCII binary script captured at `/tmp/opencode/c30-ascii-play.txt`; no human playtest.

## Adversarial review and verification

Tests cover command parser and public intents, save/load after both attunements and after an attack, C2.9 revision-19 restore with its original three lots, input gating, lost/out-of-range casts, two successful casts and refusal of a third, rejected re-attunement, exact one-hit reduction for two wards then ordinary damage, and sale-versus-attunement branches. Wards are consumed before a combat incident is recorded so witnesses see the **actual** post-ward damage rather than a fictitious blow. All four original revision-20 lot identities persist through crafting, trade and consumption.

Offline Linux gates: root `cargo test --offline`: 292 simulation passed, 1 ignored; 2 doctests passed. Playable workspace `cargo test --offline --no-default-features`: 57 library tests and all integration suites passed. Rustfmt, both all-target Clippy `-D warnings`, and whitespace checks passed. Windows unverified.

## Remaining RPG scope

This is a bounded personal-cultivation vertical slice, **not** a full sect-technique system: rival NPCs do not learn different forms from deposited relics, the ward has no custom animation, and jade channels are finite charges rather than a persistent progression tree. Next expansion should tie sect-owned relics and reputation to mentor access, NPC tactical techniques, and an interactive rivalry/duel rather than another cargo-recovery scenario or an information panel.
