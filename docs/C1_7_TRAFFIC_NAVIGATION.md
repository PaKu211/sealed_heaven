# Sealed Heaven C1.7 — Traffic Courtesy / Navigation & World Agency

Status: **compiled on 20 November 2026**. `living::` 56/56, ASCII client 14/14, `sealed_agency` 5/5. Rustfmt and Clippy `-D warnings` are clean. A five-minute human playtest was not done.
Baseline: user's tested C1.6 ZIP, described as commit `c4806bf` on
`feat/c16-sect-politics` (66 tests passed in the earlier Windows test gate).
This document describes C1.7 source changes, not a claim that C1.7 passed that gate.

## Problem and player-facing contract

The C1.6 planner rejects an occupied cell. At a one-tile bottleneck an idle
villager could stop a courier, companion or player indefinitely, even when
there was walkable ground on both sides. The original collision policy remains
intact for existing saves. New campaigns opt into *courtesy passing*:

1. `wasd` into an **adjacent, willing NPC** exchanges the player's and NPC's
   positions in **one canonical turn**. This consumes the player's action; it
   does not grant an extra attack, item, or movement step.
2. An NPC tries the existing occupied-cell-aware shortest route first. If
   blocked completely, it advances toward the objective on the static map;
   when the immediate obstruction is a willing NPC, it waits for **two
   attempted path steps** and then exchanges positions with that NPC.
3. The exchange moves both actors **one cardinal cell**; never teleports
   either actor, never trades with a player involuntarily, and never moves
   an NPC that has already committed an action or movement this tick. NPC
   commit priority continues to be actor-ID order, as in C1.6.
4. Dead NPCs, NPCs who distrust the approaching actor, actors treating a
   patient / performing field care / retaliating / retreating, an on-duty medic
   at the clinic and an on-duty teacher at Reed Hall refuse. A refusal leaves
   the obstruction in place; it is not magical crowd evacuation.
5. Each collision still occupies at most one tile. A finite `BTreeMap` tracks
consecutive stalls and a deterministic `BTreeSet` reserves actors for the
   current tick. A rolling history holds at most 256 exchanges.
6. `trafficlog` shows at most eight **witnessed** exchanges, in English or
   Indonesian; unobserved NPC exchanges remain hidden from the player.
   Inspecting the log, switching language, saving and loading take no tick.

This is deliberately a local, deterministic solution to a meaningful
navigation failure, **not** a complete multi-agent traffic optimizer. It
does not guarantee liveness when both sides have critical duties, one refuses,
the player occupies the only passage, or three or more actors form a cycle.
There is no AI-wide knowledge of a remote collision.

## Engine, determinism and persistence

`Rules::TrafficCourtesy` is a new **revision 8** in `SH:C10:WORLD`.
Its scenario (`SH:C10:RULES:v8`) uses the same authored Qinghe–Willowford
maps and NPC roster as revision 7, and adds no new intent opcode.
Its state digest (`SH:C10:STATE:v8`) covers stall counts, total passes,
static-route approach count, and bounded witnessed/unwitnessed event history.
The temporary committed-actor set is emptied at every committed turn boundary and
therefore is intentionally *not* encoded in the state digest.

Older revision 1–7 rules, scenario fingerprints, state digests and replay
semantics are unchanged. Saved C1.6 campaigns remain revision 7 and do not
gain passing; a *new* campaign opts into revision 8. `--c16` opens the old
campaign with `checkpoint.c16`; default terminal and `--pixel` now open a new
campaign using `checkpoint.c17`. `--c14` and `--c15` are retained. Load
reconstructs the world by replaying its canonical inputs and verifies the
state digest and envelope checksum as before.

No change was made to the frozen C0.1 kernel. The `src/living/traffic.rs`
module owns the new rule, and the old `travel` and `Move` branches are still
used verbatim whenever traffic rules are absent.

## Verification gate to run in a Rust-capable environment

All code and tests in this ZIP are **uncompiled** in this environment: it
contains no `rustc`, `cargo` or `rustfmt`. Six new engine tests cover:

- Player passage through a single-tile gate and one-pass accounting.
- NPC passage after two stalls, including two-person movement reservation.
- Refusal by busy and hostile actors.
- Older revision rejection of occupied movement and revision-7 checkpoint
  replay on both seeds.
- Revision-8 simulation for 200 turns on each route, exclusive occupancy,
  exact checkpoint restore, checksum stability and 12-step continuation.
- Reaching and passing the patrol guard in a real seeded campaign, then
  restoring the witnessed exchange from the replay journal.

Three new terminal-client tests cover localized, read-only inspection,
revision-7 reset isolation and a visible player passage.

Run from repository root:

```powershell
cargo fmt --all -- --check
cargo test --manifest-path Cargo.toml living::
cargo test --manifest-path apps/sealed_playable/Cargo.toml
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path apps/sealed_playable/Cargo.toml --all-targets --no-default-features -- -D warnings
$env:SEALED_ASCII_COMMANDS = "background wanderer;east;east;wait;trafficlog;save;load"
cargo run --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency
```

If the root is a workspace, adjust the `--manifest-path` according to the
existing project test commands. For the GUI, explicitly enable its desktop
feature and use `--pixel` after the Rust gate; no GUI validation is claimed.

Follow-up manual acceptance: start both seeds, create a gate obstruction,
verify one-tile swaps and non-moving busy workers, tour Qinghe/Willowford for
at least five minutes, save/load during congestion, then test a 200-turn
economy/politics balance session. Compare C1.6 revision-7 checkpoint bytes
against the separate C1.6 archive before merging. Neither a Windows-native
session nor the independent checksum comparison has yet been performed.

## Limits and next step

C1.7 gives the living world reliable *local* passage without weakening
determinism. Bigger world agency—multi-cell group routing, shared navigation
intent, queue etiquette, ownership of roads, actual trade caravans and
cross-region consequences—should build on a later revision after this narrow
invariant is compiled and measured. Do not move combat, time or witness rules
into the frozen kernel merely to implement those features.
