# C2.2 Gameplay (ASCII)

## Start

```powershell
cargo run --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency --no-default-features
```

Legacy:

```powershell
cargo run --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency --no-default-features -- --c21
cargo run --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency --no-default-features -- --c20
```

## What changed

Institutions still audit delivery sites in person (C2.1).  
Those findings can now be **briefed** to trusted contacts. A credible failure report may open a **dispute** that changes later contract awards, including refusing a suspended supplier.

## Useful commands

| Command | Effect | Advances time? |
| --- | --- | --- |
| `n` / `wait` | One simulation tick | yes |
| `contractlog` | Witnessed contract events | no |
| `accountlog` | Witnessed site audits | no |
| `infolog` | Witnessed testimony / disputes | no |
| `lang id` / `lang en` | UI language | no |
| `seed 1` | Alternate authored road (before first turn) | no |
| `help` | Campaign blurb | no |

## Observing the new loop

1. Let a shrine medicine contract fail (for example, the provider dies mid-delivery elsewhere).
2. Wait until the Watch can audit the shrine after the deadline — only then does firsthand knowledge exist.
3. If you stand with the speakers, `infolog` may show a briefing being queued.
4. After Mentor investigates and suspends a provider, later Hall offerings awards can refuse that supplier.
5. Save/load (`v` / `o`) keeps revision-13 information state through event-sourced replay of stepped campaigns.

Unseen private briefings never appear in `infolog`.
