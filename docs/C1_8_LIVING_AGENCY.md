# Sealed Heaven C1.8 — Living Agency: Qinghe Medical Relief

Status: **compiled on 20 November 2026**. `living::` 64/64, ASCII client 16/16, `sealed_agency` 5/5. Rustfmt and Clippy `-D warnings` are clean. Both regional seeds complete a physical relief chain in the engine test. A five-minute human playtest was not done.

## Player promise

Qinghe's medicine shortage is no longer only a number on the supply panel.
Under a genuine crisis, a named NPC can notice the shortage, travel to a
different settlement to request aid, and persuade another NPC to spend
real faction money on real herbs. The purchased medicine must physically
reach Qinghe before its stock changes. This can happen while the player is
elsewhere or already dead; the story is part of the simulation, not a quest
trigger tied to the protagonist.

### Canonical decision chain

| Transition | Cause / prerequisite | Material or world consequence |
|---|---|---|
| Observe | Tick 48 or later; shrine has 0–1 herb; living Watch physically sees the shrine; living Hall steward exists | Watch starts a relief request with known messenger and sponsor |
| Report | Watch walks to steward's **known home/work area** and comes within one tile of the actual steward | Steward learns of the shortage; `SeekingSponsor` → `Procuring` |
| Procure | Steward walks to Willowford market; market open; stock and Hall funds sufficient | One market herb moves into steward inventory; market receives its quoted coins, Hall treasury pays. Alternatively, a pre-existing herb in the steward's pack is redirected |
| Deliver | Steward walks to the Qinghe shrine while carrying that herb | Steward inventory −1, shrine +1, NPC delivery counter +1, faction pressure −1. Mission closes |
| Withdraw | Shrine recovers to at least 2 before cargo is committed | Open request closes; no fabricated cargo |
| Fail | Messenger or sponsor dies, or 240-turn mission deadline expires | Request closes, but any physical cargo remains with its actor/corpse |

If the steward personally distrusts the messenger, he can refuse after the
face-to-face meeting. If the steward has committed to accompany the player,
the institution does not override that promise: an open mission is withdrawn,
and new relief requests wait for an available steward.

**Road rest:** Messenger and steward may pause on the road when fatigue reaches
36; their fatigue decreases by seven. It is an explicit action, consuming a
canonical turn. It cannot teleport a worker home or advance the mission at
night when the market is closed.

Several subtleties are intentional:

- The relief request is discovered by **the watchman** through line of sight.
  The steward does not learn it until contact. While the steward is away,
  the messenger waits at a known workplace instead of tracking his invisible
  live coordinates. The guard does not know whether a remote courier has
  cargo; an unnecessary request can be withdrawn when routine supply arrives.
- A crisis is not forced on the opening tutorial. Qinghe has an existing
  five-herb haul. This system activates only when that normal chain genuinely
  leaves the shrine with critically low stock later (through use, violence,
  lost workers, scarcity, etc.); the local courier may still recover it.
- The original courier and supplier retain their usual physical duties.
  Market supply orders, Hall commerce, sect rivalry and witness-based rumors
  still advance on the same canonical tick.
- NPC decision priorities preserve self-defense; critical emergency logistics
  supersedes routine patrol/trade and travel comfort, with roadside rest as a
  temporary interruption.
- Only events **witnessed by the player at the time** appear in `relieflog`.
  The simulation stores the rest for deterministic replay, without leaking
  unseen events in the playable client.
- No generative AI is used in runtime decisions. Goals, costs and arbitration
  are bounded and deterministic.

## Engine and checkpoint contract

- The new `Rules::LivingAgency` is **revision 9** in the same `SH:C10:WORLD`
  replay envelope. It uses `SH:C10:RULES:v9` and `SH:C10:STATE:v9`.
- The map, actor IDs, campaign seeds and existing intent opcodes are
  unchanged from revision 8. Revision 9 also retains traffic courtesy.
- The revision-9 state digest includes active mission phase, opening tick,
  messenger/sponsor, counters, cooldown, bounded relief history, actor goals,
  holdings, market inventory, treasury, shrine inventory and other existing
  simulation state. It stores *no* global knowledge of unseen incidents.
- **Revision 1–8 rules, digests and decoding are untouched.** Explicit
  `--c17` selects revision 8 and `checkpoint.c17`; the new default uses
  revision 9 and `checkpoint.c18`. `--c16`, `--c15` and `--c14` remain.
- No changes to frozen C0.1 kernel or external graphics engine. ASCII remains
  default; `--pixel` uses the new session but needs native GUI verification.

## Static review and pending acceptance

The source adds eight engine tests in `src/living/relief_tests.rs` and two
terminal tests. Planned/implemented test coverage:

1. Revision-9 checkpoint replay across both map seeds and revision-8 isolation.
2. Genuine observation by a nearby Watch, plus no player news leakage.
3. Accounting for a market purchase, treasury payment, physical herb handoff
   and shrine delivery without material duplication.
4. Empty-market refusal (no synthetic herbs or negative balances).
5. Mission failure and inventory survival after the sponsor's death.
6. Refusal when the steward personally distrusts the messenger.
7. Withdrawal when other villagers replenish medicine before dispatch.
8. Full multi-actor travel from both seeds following a controlled shortage,
   with `Observed → Reported → Purchased/Redirected → Delivered`.
8. Read-only localized `relieflog`, seed/reset and revision-8 client isolation.

**No cargo/Rust toolchain is present in this cloud workspace.** This package
is not accepted until these commands pass in a Rust-capable environment:

```powershell
cargo fmt --all -- --check
cargo test --manifest-path Cargo.toml living::
cargo test --manifest-path apps/sealed_playable/Cargo.toml --no-default-features
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --all-targets -- -D warnings
$env:SEALED_ASCII_COMMANDS = "background wanderer;east;east;wait;relieflog;trafficlog;save;load"
cargo run --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --bin sealed_agency
```

Test actual completion of the 240-turn request with two seeds, verify
physical adjacency, cargo and negative balances at *every* step, compare
revision-8 checkpoint byte hashes to the tested C1.7 ZIP, then run a human
playtest and a two-day simulation soak with death/succession. A passing
unit-suite is necessary but does not replace those checks.

## Honest limitations / future directions

This is a **single bounded relief case** at a time, not generic emergent
agency across all institutions. Scarcity requires identifiable material
conditions; ordinary peaceful campaigns may never trigger the chain.
Unresolved design questions include competing faction funding, replacing an
unavailable sponsor, multi-request arbitration,
market reserves during simultaneous high demand, civilian patients, slow
time-scale economy balance and independent narrative acceptance. The next
stage should generalize requests and resource goals after this vertical slice
has passed the Rust gate, rather than rewriting the kernel prematurely.
