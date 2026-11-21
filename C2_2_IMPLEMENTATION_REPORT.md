# Sealed Heaven C2.2 — Information, Rumors & Institutional Disputes

## Delivery status

**Implementation commit:** `7d92ad3f2173875ed630ca13e967d63db2da9e5f`

**Implemented and verified on Linux (cloud) against the C2.1 baseline commit `7694ce2`.**  
The supplied Windows ZIP archive was not present in this environment; the authoritative Git tip `feat/c21-institutional-accountability` @ `7694ce2` was used instead.

| Gate | Result |
| --- | --- |
| `cargo test --lib living::` | **130 passed** (after C2.2.1 remediation) |
| `cargo test --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --lib` | **48 passed** |
| Full playable `--no-default-features` | **79 passed** (48 lib + 6 adapters + 9 agency + 3 clock + 9 supply + 2 conflict + 2 memory) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --lib -- -D warnings` | clean |
| `cargo clippy --manifest-path apps/sealed_playable/Cargo.toml --no-default-features -- -D warnings` | clean |
| Human playtest | **not done** |
| Windows re-verification | **pending** (commands below) |

## Design purpose

C2.1 taught institutions to judge providers only after a physical site audit.  
C2.2 makes those findings **travel as information**: named events, provenance, face-to-face testimony with delay, and bounded institutional disputes that change later contract awards.

Causal chain required by acceptance:

**Event → Observation → Information transfer → Institutional knowledge → Decision → World consequence.**

## Canonical rules (revision 13)

1. A completed C2.1 site audit records a stable `EventId` and grants the auditor **firsthand** knowledge.
2. Distant actors do **not** learn from global world state. Knowledge moves only by in-reach testimony into a delayed pending report (`REPORT_DELAY = 2`).
3. Provenance is preserved: `Firsthand { observer }` or `Reported { sender, origin, hops }` with `MAX_HOPS = 2`.
4. Learning the same event twice never multiplies dispute effects.
5. Institutional requesters (Watch, Mentor) may open a dispute when they learn an **Unfulfilled** finding.
   - Firsthand failure → immediate **Suspended** cooperation with that provider.
   - Secondhand hop-1 → **Open**, then site investigation can promote to **Confirmed/Suspended**.
6. Credible **Delivered** testimony (firsthand or hop-1) can **Refute** open/investigating/suspended disputes about the same provider and need.
7. Bid ranking adds `dispute_bid_delta`; **Suspended** providers are excluded from awards entirely.
8. Bounds: 96 events, 48 knowledge receipts/actor, 64 pending reports, 32 disputes, 160 history lines, news lifetime 640 ticks.
9. Player `infolog` shows only **witnessed** history. Inspection and language changes do not advance time.
10. Revision-12 (`--c21`) and revision-11 (`--c20`) remain unchanged and do not silently upgrade.

## Architecture

New module: `src/living/information.rs` (plus `information_tests.rs`).

Reuse, do not duplicate:

- Social trust filters (`Relationship::trusts_news`) for who will accept briefing.
- Contract memory audits as the sole source of institutional findings.
- Contract bid selection as the consequential decision surface.
- Event-sourced checkpoints (`SH:C10:WORLD` rev 13, rules/state fingerprints `v13`).

Goals added: `DeliverTestimony`, `InvestigateDispute` (wire tags 34–35).

C2.2 seeds a small Watch↔Mentor and Watch↔Hall briefing acquaintance so institutional findings can reach decision-makers without merging into combat gossip.

## Persistence

- Constructor: `World::information_disputes(seed)`
- Revision **13**, fingerprints `SH:C10:RULES:v13` / `SH:C10:STATE:v13`
- State digest includes events, knowledge+provenance, pending reports, disputes, history, `next_dispute`
- Default ASCII save: `checkpoint.c22`
- `--c21` → rev 12 / `checkpoint.c21`
- `--c20` → rev 11 / `checkpoint.c20`

## Player controls

```text
cargo run --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency --no-default-features
cargo run --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency --no-default-features -- --c21
cargo run --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency --no-default-features -- --c20
```

- `n` / `wait`: one canonical turn
- `contractlog` / `accountlog` / `infolog`: witnessed-only readouts
- `lang en` | `lang id`
- `seed <n>` before the first turn; `v`/`o`/`r` save/load/reset

## C2.2.1 remediation

See `C2_2_1_REMEDIATION.md`. Checkpoint revision stays **13**. Later deliveries **restore** cooperation without rewriting failure events; hop-2 knowledge can upgrade; suspensions age out; settled event slots recycle.

## Known limits

- Testimony currently carries **contract audit findings**, not arbitrary combat rumors or forged papers.
- Investigation confirms hop-1 reports at the event site; hop-2 allegations are remembered until stronger evidence arrives.
- If every event slot is still hot (active dispute/pending), new findings may still wait.
- Event-sourced restore replays the intent journal; ad-hoc test mutations outside `step()` are covered by digest membership tests, not by forged journals.
- Human play feel and long bottleneck navigation still need a Windows play session. Windows gates were not run in this environment.
