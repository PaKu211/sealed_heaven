# C2.2 Test and Audit Report

## Environment

- Branch: `feat/c22-information-rumors-disputes`
- Verified code tip: `20f7c0ac90f773774b2e63d3018b34ee619cd1fd`
- Host: Linux cloud agent
- rustc: 1.100.0
- Baseline: Git commit `7694ce2` (`feat/c21-institutional-accountability`)
- Checkpoint revision remains **13**. The C0.1 kernel is unchanged.

## Identity correction

`EventId` is allocated from `Information::next_event` and is no longer a storage index. Settled findings may leave the 96-slot operational ledger. A retired id is not issued again. Pending reports and open, investigating, confirmed, or suspended disputes stay stored. `next_event` is part of the revision-13 state digest. Replaying a campaign reconstructs that counter; a digest with a forged counter does not restore.

## Final Linux gates

```bash
cargo fmt --all -- --check
cargo test --lib
cargo test --lib living::
cargo test --manifest-path apps/sealed_playable/Cargo.toml --no-default-features
cargo clippy --all-targets -- -D warnings
cargo clippy --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --all-targets -- -D warnings
```

| Command | Result |
| --- | --- |
| `cargo test --lib` | **249 passed**, 1 ignored |
| `cargo test --lib living::` | **134 passed** |
| playable `--no-default-features` `--lib` | **48 passed** |
| playable adapters | **6 passed** |
| playable agency (`tests` c05) | **9 passed** |
| playable clock (c06) | **3 passed** |
| playable supply (c07) | **9 passed** |
| playable conflict (c08) | **2 passed** |
| playable memory (c09) | **2 passed** |
| fmt / clippy `-D warnings` | clean |

Total playable `--no-default-features` automated tests: **48 + 6 + 9 + 3 + 9 + 2 + 2 = 79**.

## Compatibility smoke

Headless `sealed_agency --no-default-features`, seeds 0 and 1:

| Invocation | Banner after `seed 1` |
| --- | --- |
| default (C2.2) | C2.2 help text |
| `--c21` | C2.1 help text, not C2.2 |
| `--c20` | C2.0 help text, not C2.2 |

Living replay covers both seeds in `v13_replays_canonical_worlds_and_keeps_v12_campaign_exact` and `next_event_counter_is_in_the_digest_and_replay_reconstructs_it`.

## Still not claimed

- Windows host re-run
- Human playtest
