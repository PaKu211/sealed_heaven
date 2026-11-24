# C2.8 — Physical corpse-cargo recovery

Revision 18 (`checkpoint.c28`, default ASCII). `--c27` retains revision 17 and `--c25` revision 16. Frozen C0.1 kernel unchanged.

If a watchman **actually sees** a corpse carrying herbs while shrine medicine is short, and is free and sufficiently rested, he walks to the body, transfers the finite cargo to his own inventory, carries it along the road, and delivers it at the shrine. No unseen remote recovery and no inferred killer. Retaliation and exhaustion preempt salvage. Receipt custody is explicit in the revision-18 state digest, with monotonic pickup/delivery event IDs and witness-local `salvagelog`. A watchman's death clears his undeliverable assignment but leaves the physical herb on his body. A locally observed, still-unawarded replacement contract can be closed as `ResolvedLocally` without a spurious failure, purchase, or provider blame.

## Evidence and critical review

- From an untouched revision-18 seed-zero campaign, public commands walk east fourteen turns, wait, attack living courier 32, chase him, land two more blows, then wait. At tick 47 the watch picks up the one herb; at tick 67 he delivers it. Public ASCII `salvagelog` shows only the pickup witnessed by the player, not the unseen delivery. A second public `take 32` branch at tick 27 leaves the player holding the herb; the watch cannot recover it, and the replacement contract remains active at tick 75.
- Both forks replay identically after save/load. Full regional material totals (market, garden, NPC inventories, shrine, remedies, consumption and regrowth) match the initial total plus actual regrowth on **each tick from 27 through 75**. Checkpoints restore during transit and after delivery. A separate adversarial test kills the herb-carrying watchman: the herb stays on his corpse and the player can take it once.
- `C2_7_VERIFICATION.md` records the earlier face-to-face player-reporting path. Revision-17 checkpoints remain version-isolated; recovery events are never added to them. Institutionally, recovery prevents unnecessary outside awards, but existing awarded obligations are not silently erased.
- Actual headless ASCII script captured in `/tmp/opencode/c28-ascii-play.txt`; no human playtest.

Offline Linux gates: full root `cargo test --offline` (simulation 282 passed, 1 ignored; 2 doctests passed), playable 86 passed. Rustfmt and both all-target Clippy checks with `-D warnings` clean. No push, merge or Windows verification.

Limits: only the Qinghe watchman currently salvages corpse herbs, and only for a shrine shortage; no broader scavenger economy or sect contest over abandoned cargo. No assault accusation is inferred from finding or recovering a body. Next vertical slice should give rival factions and couriers competing lawful/unlawful goals for scarce goods, rather than adding more read-only panels.
