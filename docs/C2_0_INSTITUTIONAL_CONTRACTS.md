# Sealed Heaven C2.0 — Institutional Request / Offer / Fulfillment Engine

**Status: compiled on 20 November 2026.** `living::` 90/90. Playable tests without the desktop feature: 75/75. Rustfmt and Clippy `-D warnings` are clean. Revision 11 replays both roads, and revision 10 stays isolated. A five-minute human playtest was not done. Source base is the user's tested C1.9 ZIP, identified
with local Windows commit `f57b9b7` and 486 files. The user's C1.9 automated
gate passed 76 `living::` and 73 no-desktop client tests. Those results are
not presented as tests of C2.0.

## Architectural goal

C1.8 and C1.9 proved that a shortage can provoke a material response, but
each used a purpose-built mission state machine. C2.0 adds a **bounded
institutional protocol shared by multiple demand types**, without changing
the frozen C0.1 kernel:

| General contract element | Qinghe application | Reed Hall application |
|---|---|---|
| Observed need | Shen Wei sees shrine medicine at 0–1 | Ren Zhi at Hall sees missing offerings and high rivalry pressure |
| Requester | Watch 2 | Mentor 33 |
| Candidates | Hall steward 36, Stonebank envoy 35 | Hall steward 36 |
| Offers | Real owned cargo or affordable market herb | Same offer and affordability mechanism |
| Delivery site | Qinghe shrine | Reed Hall |
| Physical effect | Shrine +1; NPC deliveries +1 | Hall offerings +1; Hall supplies +1 |
| Political consequence | Hall weakens or Stonebank strengthens claim pressure | Hall weakens claim pressure |

## Canonical state machine

**Observed → Soliciting → Awarding → Procuring → Delivering → Closed.**

1. A named NPC must be alive and close enough to *see* the relevant need.
   It opens an identified case after a deliberate stage-specific threshold;
   a remote global shortage does not create a report.
2. The requester travels to each candidate's **known workplace or home**.
   A bid exists only after *face-to-face contact*. The bid records provider,
   quoted cost, and the requester's personally known trust toward the
   provider. A candidate can decline through distrust, insufficient resources,
   death, or existing company obligations.
3. After gathering offers, the requester chooses deterministically:
   higher local trust, lower quoted cost, Hall precedence on a final tie,
   then stable actor ID. **The chosen provider is not yet assigned a job.**
4. The requester returns to that particular provider to communicate the
   decision. Only this second contact makes a firm assignment and gives
   the provider the higher-priority `ProcureContract` goal.
5. A last-minute refusal or lost capacity at the actual award triggers a
   fallback to the next stored offer, which still requires a physical visit.
   A provider already performing an accepted contract cannot be assigned a
   second. Tentative awards do not secretly reserve a remote worker.
6. The provider uses **one pre-existing personal herb** or transfers one
   herb from real market inventory in exchange for the **current** quote in
   real Hall or Stonebank treasury coins. A stale offer never forces a
   purchase when stock or funds have changed.
7. Only after the actual NPC reaches the real destination does the
   inventory transfer. The destination's need changes, rival pressure
   changes, and the locally present requester may gain social trust.

The protocol runs on the **same canonical turn** as farming, Qinghe freight,
regional trading, interpersonal news, patrols, combat, companion care and
player movement. All NPC goals plan from one snapshot and commit in actor-ID
order with stock and money revalidated at purchase time.

**Failure boundaries:** Deadline 480 turns, death of the active responsible
actor, or an accepted provider honoring a companion promise. A failed
mission does not delete cargo, synthesize a replacement herb or transfer
money. Cases are bounded: at most one per need, two needs total, two offers
per shrine case, one offer per Hall case, and a history of 160 entries.
Cooldown is **per need**, preventing unrelated requests from blocking each
other. Crisis money reserves also apply to revision 11.

**Partial information:** Only the requester knows the offers gathered.
Only the provider learns an award after contact. The `contractlog` ASCII
command shows only events the player personally saw when they happened,
not every global ledger entry. The full ledger remains available to a
developer for deterministic testing. The world proceeds when the
player is dead.

## Compatibility and migration

- `Rules::InstitutionalAgency` is checkpoint **revision 11**, tagged
  `SH:C10:RULES:v11` and `SH:C10:STATE:v11`.
- The `SH:C10:WORLD` envelope and journal intent record sizes stay intact.
  No new player input opcodes are needed.
- The v11 state hash serializes each contract's ID, site/need, requester,
  phase, candidate progress, all collected bids (quoted cost and trust),
  selected sponsor, opening time, per-site cooldowns, counters and bounded
  event history, in addition to all existing material, NPC, relationship
  and world state.
- Revisions **1–10** keep their original tags and behavior. New campaigns
  use the generalized contracts instead of running both C1.8 and C1.9's
  old aid mission machines simultaneously. This prevents accidental double
  dispatch while retaining older saves under their original rules.
- Default `sealed_agency` runs v11 with `checkpoint.c20`.
  Explicit `--c19` runs the tested v10 faction campaign with
  `checkpoint.c19`; `--c18`, `--c17`, `--c16` and earlier switches
  remain available. The Bevy `--pixel` projection shares the default
  session but still needs native GUI verification.
- The C0.1 kernel, hash primitives and low-level determinism contract
  remain untouched.

## New test plan and review gate

**Fourteen new engine tests and two ASCII tests were authored**:

1. v11 deterministic checkpoint replay and continued ticks, both roads.
2. v10 isolation and no silent rules upgrade.
3. Local witnessing requirement for Qinghe and Hall's distinct need.
4. Physical face-to-face solicitation; no remote bids.
5. No remote/telepathic awards; requester must return to the chosen provider.
6. Rejection at award and physical fallback to alternate bid.
7. A provider cannot receive two committed jobs.
8. Shared-stock competition without double purchase.
9. Exact herb and coin conservation on a real market buy.
10. No free cargo from an empty market or empty treasury.
11. Same delivery machinery applied to shrine and Hall destinations.
12. Death and companion promises do not delete owned cargo.
13. End-to-end autonomous shrine mission in an authored crisis, on both roads.
14. Institutional decisions continue after player death.
15. ASCII `contractlog` is localized, witnessed-only and consumes no turn.
16. ASCII seed/reset maintains v11; explicit v10 campaign remains isolated.

Cloud environment limitation: **no `cargo`, `rustc` or `rustfmt`**.
Static patch/tree checks are not substitutes for compilation or a playtest.

Windows/native gate:

```powershell
cargo fmt --all
cargo test --manifest-path Cargo.toml living::
cargo test --manifest-path apps/sealed_playable/Cargo.toml --no-default-features
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --all-targets -- -D warnings
$env:SEALED_ASCII_COMMANDS="seed 1;wait;help;contractlog;save;load;lang id;contractlog"
cargo run --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --bin sealed_agency
cargo run --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --bin sealed_agency -- --c19
```

Then verify **byte-for-byte v10 checkpoint digests** against the tested C1.9
ZIP, review the 600-turn scarcity scenario on both roads, test conflicting
contracts during one-stock scarcity, and play five human minutes. For
playability, check whether four-stage physical negotiations create excessive
waiting and whether narrow road contention prevents completion. Long-save
replay performance remains a measurable risk.

## Explicit design limits

The protocol is reusable across **two authored needs for one commodity**,
not yet a generic economy for hundreds of institutions or multiple item
types. There is no global knowledge bus, procedural NPC generation,
dynamic price auction, arbitrary item ownership, chain of subcontractors,
or generative AI runtime. Bids are local snapshots; actual acquisition
always rechecks current prices and stock. The first two needs deliberately
share the same working code to validate the abstraction before expanding
it to housing, security, caravan contracts, debts, or multi-generation
institutions.
