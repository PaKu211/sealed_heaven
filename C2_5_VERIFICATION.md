# C2.5 — Violence, Logistics & Systemic Consequences

Branch `feat/c25-violence-logistics-consequences`, checkpoint revision 16. `--c24` retains revision 15 and its isolated checkpoint file. No frozen C0.1 kernel files changed.

The public-command route in `src/living/consequences_tests.rs` and `apps/sealed_playable/src/living_ascii.rs` walks east fourteen times, waits, attacks courier Bao Jun, pursues him, attacks twice more, examines the body with `bodies`, takes its one real herb, saves, and loads. The courier dies at tick 26, his herb stays on the body until taken, and he cannot deliver it. On the comparison route he delivers; on the attacked route the shipment remains undelivered at tick 180. The shrine's *locally observed* shortage at tick 33 leads the watch to solicit an institutional contract. An independent provider buys and carries another real herb, fulfills the contract, and the requester eventually audits the delivered obligation. This is a delayed shipment with a costly institutional response, not a fabricated direct report of the distant murder.

Attacks identify assailants only to direct witnesses; secondhand assault reports still require in-person meetings. New body discoveries record a corpse without inferring its killer, optionally record *unproven* nearby-player proximity, and pass corpse news at most two hops through physical conversations and delayed delivery. In the public route the watch discovers the corpse, tells the steward face-to-face, and the steward tells the mentor. Neither recipient gains knowledge of the distant fatal strike. New body receipts and pending messages enter the revision-16 digest. A regression caught a misleading ASCII rendering of a zero-damage theft incident as a hit; it now says cargo was taken.

## Verification (Linux, offline)

- `cargo test --offline --lib`: 276 passed, 1 ignored.
- `cargo test --offline --manifest-path apps/sealed_playable/Cargo.toml --no-default-features`: 83 passed across all suites.
- `cargo fmt --all` in both workspaces; both `cargo clippy --offline --all-targets -- -D warnings` gates clean.
- Public ASCII binary scripted through `SEALED_ASCII_COMMANDS` (14 moves, pursuit, three accepted attacks, `bodies`, `take 32`, `save`, `load`, `help`); captured locally in `/tmp/opencode/c25-ascii-play.txt`.
- Deterministic dual-run digest comparison on every tick through 180; restores at ticks 26, 43, 68 and 84; revision-15 save round-trips with no discoveries.
- Herb-total comparison on the death turn against the no-attack branch. No herbs created by combat or theft.

No human or Windows playtest performed. This scenario proves a delayed delivery and an institutional replacement, not every possible contract failure. Corpse-news receipts concern the corpse only; they are not forensic proof of who caused the death. The next valuable milestone is player-initiated reporting/investigation of discoveries and material recovery so that the player can influence more than the first assault.
