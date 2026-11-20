# Sealed Heaven · C1.5 — Stonebank and Reed Hall

Status: **compiled and tested on 20 November 2026**. Rustfmt and Clippy `-D warnings` pass. `living::` 42/42 and the ASCII client tests pass. A five-minute human play and a 200-turn tuning pass were not done.
This is the next bounded Stage E experiment after C1.4, not a claim that the full sect or inheritance systems are complete.

## Player-facing conflict

Qinghe and Willowford continue on one canonical turn. A Stonebank lodge envoy (ID 35, symbol `V`) starts in Willowford. The envoy has a finite treasury of **16 coins** and independently walks to the actual market to purchase one herb at the current buying price. He carries the herb to Reed Hall and petitions the teacher, consuming it and increasing Stonebank pressure by two. The purchase transfers coins to the market and removes market stock; it is not an abstract faction counter. The envoy does not purchase after dark, while a truce is active, or if the treasury or market stock runs out. Fatigue, injuries, retaliation, reports, path obstruction and actor-ID ordering continue to use the ordinary NPC planner.

Pressure reaches **4/8**: Hall teaching services (enrollment, remedy brewing, shrine-duty reporting and learning) become contested, while the market buying quote increases by one coin. Player movement, existing knowledge, cargo delivery, combat, healing outside the Hall, and the world's independent clock continue normally.

The player can intervene in two ways:

- `accord`: next to the living envoy, offer **one carried herb** to reduce pressure by three and create a 48-turn truce. The envoy pauses purchases and petitions during the truce, even if some cargo is already in transit. A hostile envoy can refuse.
- `sponsor`: as a Reed Hall member within reach of the Hall, donate **one carried herb** to reduce pressure by two and permanently increase the Hall offerings ledger. This action can be used while teaching is contested.

`factions` shows local political status only within sight of Willowford's market or Reed Hall. Stonebank's exact remaining treasury is visible only in the explicit developer view. `factionlog` shows up to eight rivalry events actually witnessed by the player; records witnessed remotely are **not** leaked to the player's journal. The regular `journal` continues to describe company travels. `inspect 35` and `talk 35` work like other NPCs.

Run the ASCII campaign (once a Rust toolchain is available):

```bash
cargo run --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency
```

`--c14` launches the untouched C1.4 rules and save file `checkpoint.c10`. C1.5's default saves as `checkpoint.c15`; `--pixel` also uses this new file, with keyboard keys **C** for `accord` and **V** for `sponsor`. For repeatable script input, set `SEALED_ASCII_COMMANDS='n;n;factions;factionlog'` (semicolon delimiters for multiword commands).

## Architecture and compatibility

- **Revision 6** uses the existing `SH:C10:WORLD` envelope, fixed 5-byte intent records, and SHA-256 envelope + replay-state verification. New intent tags 21 (`accord`) and 22 (`sponsor`), NPC goal codes 19–20, outcomes 27–28 and the event/stock/pressure ledger are appended to the v6 state digest. A fixed, revision-specific scenario digest captures the new actor and their contacts.
- Reader revisions **1–5** select exactly their former scenario, ruleset, NPCs, goals and digest prefixes. Original revision-5 `RoadCompany` remains byte-compatible; the new competition logic exists only under `Rules::SectRivalry`. `MAX_TICKS`, canonical tick admission, stable `ActorId`, and the C0.1 kernel are unchanged. Save data reconstructs through event replay, not an alternate mutable snapshot loader.
- A faction event records `tick`, `actor`, `location`, kind, and whether the player personally observed it. The referee retains all simulation history for replay integrity, but the client filters the player's journal by witness flag. This is a deliberately local first history rather than a global omniscient news feed.
- Market quote changes are applied once at the actual player or envoy transaction boundary; currency and material stock move explicitly. The Qinghe shrine's incoming-herb calculation still counts only Qinghe-bound supplier and courier cargo, never Stonebank's trade cargo or companion medicine packs.

## Acceptance matrix (written, not run)

| Case | Expected invariant |
| --- | --- |
| Both regional seeds | Authored scenario is valid; envoy physically trades and petitions; pressure changes from recorded acts |
| Economies | `market_purchases = petitions + envoy_cargo`; lodge spending credits market treasury; no purchases after dark |
| Revision 6 save/load | After 100 canonical waits, exact world replay and further continuation equality |
| Diplomacy | Each choice consumes exactly one player herb; pressure, truce, offerings, event journal change atomically |
| Older campaigns | New intents rejected *before* time advances; revision-5 save bytes unchanged |
| Market and schooling | Contest affects only v6 buy quote and teaching; lowering pressure removes restriction |

## Review findings and remaining risks

1. The first pass allowed the envoy to buy while the market was closed. Added a dawn-only check to the NPC buying precondition.
2. Economic tests initially overlooked herbs delivered from Willowford's garden. The test now accounts for the initial garden and actual regrowth, and checks market coin receipts separately.
3. C1.4 and C1.5 must not share a save filename. A `--c14` launch mode and separate C1.5 path now preserve the original campaign locally.
4. **Toolchain gate, 20 November 2026:** `cargo test living::` passed 42 tests after the closed-passage fixture moved the watch off the only gate. `cargo test --bin sealed_agency` passed 5 tests. `living_ascii` passed 9 tests. Rustfmt was applied. Clippy `-D warnings` is clean for `sealed_sim` and `sealed_playable`. A five-minute human session was not played.
5. The political conflict is presently one bounded economic contest. There is no opposing player-joinable sect, political succession, inter-sect war, dynamic recruitment, or generational inheritance yet. Long-campaign replay time and possible crowding around the Hall are not measured. Do not start large-scale Stage F simulation work until C1.5 is compiled, restored on both maps and benchmarked.

## Immediate next gates

1. Run `cargo test --manifest-path Cargo.toml living::` and `cargo test --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency`, followed by Rustfmt, Clippy and a scripted/native session for both seeds.
2. Audit the revision-5 binary checksum and historical vectors on the original C1.4 source; check save restoration across 1–6 under the actual compiler.
3. Observe the first 200 turns of NPC traffic, purchased-herb ledger and Hall blockage, then tune envoy priority, local consequences and pressure decay based on the witnessed play session.
4. Only after these gates, design Stage E's inter-sect recruitment/ideology and Stage F's regional actor-level LOD budget as separate, measurable changes.
