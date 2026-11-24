# C2.9 — Contested Resources & Autonomous Interests

Checkpoint revision 19 (`checkpoint.c29` by default; `--c28` preserves revision 18). The frozen C0.1 kernel and revisions 1–18 are unchanged. No push or merge.

## Playable slice

Three finite, individually identified lots (two SpiritOre, one JadeScript) have one physical holder each: site, player, living or dead NPC, or sect depot. The Stonebank envoy and Reed Hall steward independently observe ground/corpse locations, choose nearby known lots, walk, claim, transport and deposit them; both can race for the **same** ore. A removed lot remains a stale lead until the other NPC reaches and observes the empty location, never an omniscient live tracker. Their deposits change sect prestige and rivalry pressure; the two goods have different values. Other faction missions, retaliation, fatigue, travel and relationships retain their normal priorities and access rules.

Player commands `lots`, `lotlog`, `claim <lot>` and `offerlot <npc> <lot>` permit discovery, physical appropriation and face-to-face sale to either sect. Goods and events outside player sight are hidden in the ASCII interface. A sale transfers exact coins from that sect's treasury into the player's purse, changes custody and is refused if the patron is unreachable, hostile, or unable to pay. A second sale cannot duplicate goods or money. Killing a carrier leaves the lot on the corpse for recovery; killing the player drops carried lots at the death site. Deliveries record monotonically numbered events with observation provenance.

Acceptance route from an unmodified seed-zero campaign: move east eight times, `claim 0`, east twenty times, south and `offerlot 36 0`. The steward deposits the ore; an alternate branch walks south, west twice and `offerlot 35 0`, so the envoy receives it instead. Without intervention, the Stonebank envoy deposits lot 1 and Reed Hall deposits lot 2 on their own while lot 0 remains unclaimed. Both NPCs observe lot 1 early; only the first physical claimant obtains it. Saved states replay byte-for-byte through each branch; even observations, pressure, receipts, and cargo state are included in the revision-19 digest. Actual headless binary command transcript: `/tmp/opencode/c29-ascii-play.txt`. There is no human playtest.

## Adversarial review and gates

Tests exercise public `Intent` and ASCII command routes, three independent outcomes, conservation of three finite lot identities, single custody and single deposit, exact zero-sum sale coins, idempotent failed resale, unfunded patron, invisible goods, stale NPC information, duplicate claimant, death and corpse recovery, checkpoint replay, and revision-17/18 isolation. Review fixes include clearing player custody on death and never reading a changed holder to choose an NPC's distant destination.

Offline Linux gates: root `cargo test --offline`: 289 simulation passed, 1 ignored; 2 doctests passed. Playable workspace `cargo test --offline --no-default-features`: 56 library tests plus integration suites passed. Both all-target Clippy `-D warnings`, both Rustfmt checks and `git diff --check` passed. No Windows verification.

## Gameplay evaluation / next milestone

C2.9 adds an autonomous sect-vs-sect race and player choice of patron rather than another special courier-herb case. However, depositing a relic currently changes prestige and pressure, **not** a unique player ability; the player has no reason to keep a relic except denying both sects. There are only two beneficiary roles, three initially placed lots and no lawful claim or forging system yet. Do not multiply cargo cases or add another read-only panel for the next milestone.

Choose **C3.0 — Sect Techniques & Personal Cultivation**: make retained SpiritOre and JadeScript usable for distinct, finite player techniques/crafting, while sect-owned lots unlock rival combat or training consequences. Provide meaningful keep/use/sell/steal choices, scarce reagents and public-command combat or training tests; build on the lot custody API without hardcoding outcomes to today's three lots. A complete vertical slice should prove different character capabilities and sect responses from the same physical goods, including inaction.
