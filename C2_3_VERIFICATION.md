# C2.3 verification

Host: Linux, rustc 1.100.0. Worktree `/root/sealed_heaven_c22`. Branch `feat/c23-player-agency`. Parent `4f7bdd69d36856daa55d789997ca0351a5235952`. Commands were run with `HOME=/root` except the banner smoke, which used `HOME=/tmp/c23_smoke` so it would not write a save under the user profile.

```text
cargo fmt --all -- --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline --lib -- --test-threads=8
cargo clippy --offline --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --all-targets -- -D warnings
cargo test --offline --manifest-path apps/sealed_playable/Cargo.toml --no-default-features -- --test-threads=8
```

Results actually observed:

- rustfmt clean after `cargo fmt --all`.
- `sealed_sim` Clippy `--all-targets -D warnings`: clean.
- `cargo test --lib`: 266 passed, 1 ignored.
- `living::` tests: 151 passed, including 17 `player_agency` tests.
- `sealed_playable` Clippy `--no-default-features --all-targets -D warnings`: clean.
- Playable `--no-default-features`: 50 lib, 6 adapter, 9 agency, 3 clock, 9 supply, 2 conflict, 2 memory. Total 81 passed. Empty harnesses passed with 0 tests. Desktop-feature tests were not part of this gate.

`git diff 7694ce2 --` the C0.1 kernel paths (`src/lib.rs`, domain, encoding, fixture3, player5, `src/tests`, `tests`) produced no diff.

Headless `sealed_agency` with `SEALED_ASCII_COMMANDS`:

- Default help is the C2.3 text. `seed 1` stays on that help text.
- `--c22` help is C2.2.
- `--c21` help is C2.1.
- `--c20` help is C2.0.

Not run: Windows, a human playtest, and `cargo test` with default desktop features. The scripted ASCII smoke is not a human playtest.

Reproduction of the patch and archive is recorded with the files `C2_3_FROM_C2_2.patch` and `sealed-heaven-c2.3-player-agency.zip` after the local commit. Those two files are not part of the commit.
