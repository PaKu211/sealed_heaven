# Sealed Heaven C1.9 — Autonomous Faction Relief & Consequence

**Status: compiled on 20 November 2026.** `living::` 76/76. Playable tests without the desktop feature: 73/73. Rustfmt and Clippy `-D warnings` are clean. Revision 10 replays both roads, and revision 9 stays isolated. A five-minute human playtest was not done.

Baseline is the user's *tested*, 483-file C1.8 ZIP, identified in its ZIP
comment as commit `0d5c202`. The baseline passed 64 `living::` tests,
16 ASCII tests and five `sealed_agency` tests on the user's Windows machine.
Those are *C1.8 facts*, not claims about C1.9.

## The gameplay change

The Qinghe shortage is now a genuine point of competition between **two
institutions**, not a single scripted Hall response:

1. Reed Hall retains the watchman's locally observed request, face-to-face
   report to steward Qiao Lin, and physical medicinal delivery (C1.8).
2. **Stonebank independently dispatches its envoy** on periodic inspections
   of Qinghe. The envoy knows the shrine has a shortage *only if they
   physically arrive close enough to see it*. A distant global variable
   does not authorize them to begin purchasing.
3. On witnessing critically low shrine stock (0–1 herb), the envoy uses
   **one already-carried herb** or buys **one real herb** from Willowford at
   that turn's market price. The purchase transfers stock from the market
   to the envoy, and coins from Stonebank's treasury to the market.
4. The envoy returns to the shrine, deposits that herb, and only then do
   the shrine stock and NPC delivery counter increase. Stonebank's claim
   pressure rises by one (capped at eight). Hall deliveries instead lower
   the existing claim pressure, as before.
5. If Shen Wei **personally witnesses** a delivery, he gains trust in the
   actual deliverer and Qinghe's local standing moves toward that
   institution. A remote delivery still provides medicine but cannot
   create a fictional witness or global gratitude.

Both institutional procurement intents are planned before NPC movement and
resolved in **canonical actor-ID order** with a *commit-time market stock
recheck*. Stonebank envoy 35 competes for the same market stock as Hall
steward 36, ordinary trade, gardening and the real Qinghe courier. No
duplicate herbs, minted money, teleports or background player quests.

Each institution now preserves **four coins in routine purchasing** as a
contingency. Emergency procurement can spend the reserve. This rule exists
only in revision 10: prior campaign prices, buys, and checkpoints are left
intact. The choice has a meaningful cost: less routine faction influence in
exchange for more reliable medical capacity.

The Stonebank mission yields to self-defense, needs roadside rest at high
fatigue, and ends if the envoy dies, accepts company with the player or
exceeds 320 turns. Owned cargo stays physically where it was. A completed
inspection without a shortage closes the case without purchases and cannot
read unseen future stock. Inspections have a 112-turn completion cooldown.
No global "mission accomplished" alert is sent to uninvolved actors.

## Simulation and persistence

- `World::faction_agency(seed)` activates the new **revision 10** rules,
  alongside all existing C1.8 systems. It is the default ASCII campaign.
- Checkpoint envelope stays `SH:C10:WORLD`; new fingerprints
  `SH:C10:RULES:v10` and `SH:C10:STATE:v10` explicitly separate the new
  rules and state digest.
- The v10 digest commits to survey count, active phase, envoy ID, start
  time, last closure, Hall and Lodge deliveries, bounded local standing,
  witnessed flags and a bounded 128-event chronology, plus prior NPC
  activity, location, cargo, money, stock, social relationships and journal.
- Event-sourced restore replays the *same* actor decisions. The checkpoint
  does not independently deserialize an editable global state. A direct
  test-fixture mutation is therefore **not** a replayable player action.
- Revisions 1–9 retain their original rule tags and digest serialization.
  `sealed_agency --c18` uses revision 9 and `checkpoint.c18`; C1.9's
  default uses revision 10 and `checkpoint.c19`. The older `--c17`,
  `--c16`, `--c15` and `--c14` switches remain available.
- The frozen C0.1 `src/` simulation kernel outside `src/living*` is
  untouched. The optional Bevy pixel projection shares the new campaign
  but is not accepted without GUI verification.

### Player-facing observations

- `aidlog` shows Stonebank's reconnaissance, purchase, journey, delivery,
  loss, and Hall deliveries **only if the player was present and saw that
  event**. `relieflog`, `trafficlog` and `factionlog` remain.
- The ASCII client supports English and Indonesian, and inspecting logs
  never advances canonical time.
- The aggregate local standing is **not** shown to a remote player.
  It is exposed only under `--debug-world`, while the direct witnessed
  experiences are presented through `aidlog`.
- Players can already intervene using existing social systems: joining
  Stonebank, helping Reed Hall, trading market herbs, offering cargo to
  allies or fighting the actual workers. A player is not required to
  initiate the faction story.

## Regression and acceptance plan

The C1.9 source adds **12 engine tests** and **two ASCII tests**:

1. Revision-10 replay and 25-turn continuation, both routes; revision-9
   restore stays revision 9.
2. No unseen scouting information and no unseen player log entry.
3. Shortage/healthy-shrine observations occur only with local visibility.
4. Market purchase debits exactly one herb and its quoted price.
5. One-herb concurrent Hall/Stonebank market race; only one real purchase.
6. Zero stock and zero treasury do not fabricate cargo.
7. Reserve funds are protected from routine spending, but available in
   emergencies, with revision-nine trade behavior retained.
8. Material delivery changes shrine, pressure and local trust **only when
   the right witnesses are physically present**.
9. Death and companion promises terminate missions without erasing cargo.
10. Hall relief is credited to the new standing system only under new rules.
11. An unattended Stonebank expedition completes on *both* regional roads
    in a controlled, durable shortage fixture.
12. Faction decisions and reconnaissance continue after the protagonist dies.
13. Read-only, localized ASCII `aidlog`, seed/reset and C1.8 isolation.

Rust tooling is currently unavailable in this environment. This is the
**required** native gate before calling C1.9 accepted:

```powershell
cargo fmt --all -- --check
cargo test --manifest-path Cargo.toml living::
cargo test --manifest-path apps/sealed_playable/Cargo.toml --no-default-features
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --all-targets -- -D warnings
$env:SEALED_ASCII_COMMANDS = "seed 1;wait;help;aidlog;relieflog;save;load;lang id;aidlog"
cargo run --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --bin sealed_agency
cargo run --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --bin sealed_agency -- --c18
```

After these, run a **human five-minute campaign**, a 400-turn material and
traffic soak on each seed, and compare revision-9 checkpoint bytes against
the exact C1.8 tested ZIP (the unit test alone does not prove byte
compatibility with another compilation). Stress a crowded shrine, an
absent/hostile patron, an empty market, a dead envoy and player death.

### Architectural limits, intentionally deferred

This is a **two-institution regional vertical slice**, not a generalized
faction planner or a whole-world economy. There is one Stonebank
reconnaissance/relief case at a time and one Reed Hall case. Local standing
currently affects observed trust and recorded faction history, not every
NPC's ideology. No invisible map-wide rumor, dynamic faction spawning,
generic multi-commodity transport, succession across generations or
generative runtime AI is claimed. Replay reconstruction still scales with
campaign length: benchmark long saves before extending the `MAX_TICKS`
horizon. The next rational architectural stage is a generic bounded
institutional **request/offer/fulfillment** engine after C1.9 completes its
native acceptance, not an unverified rewrite of the C0.1 kernel.
