# C0.7 handoff

**Verdict:** IN PROGRESS. Not accepted. Do not merge to `main`. Do not start C0.8.

**Branch:** `feat/c07-shrine-supply`  
**Base:** `e4598385291abd2e87133c97aa03ca49e91e1a1d` (`docs(c06): record the verified village commit` on `feat/c06-readable-village`)  
**Worktree:** `sealed-heaven-c06`  
**Stopped:** 2026-11-18, after the session was interrupted mid-implementation.

## Already on origin

| Work | Branch | SHA |
|---|---|---|
| Tri-game research spikes | `research/tri-game-integration-spikes` | `fa1da41dc23cae222803edd502896ebfb1d6f01a` |
| C0.6 readable village | `feat/c06-readable-village` | `e4598385291abd2e87133c97aa03ca49e91e1a1d` |

Those branches were not changed by this checkpoint.

## What is in the working tree

A playable-layer shrine desk on the existing C0.3 actor-owned ledger. Actor 1 starts with 12 spirit herbs (definition 7). The shrine site stock starts at 0. Empty kernel drives move one herb per haul. The desk stores demand 5 and the kernel task id it has already seen. It does not store a second quantity.

Files:

- `apps/sealed_playable/src/supply.rs`
- `apps/sealed_playable/tests/c07_supply.rs`
- `apps/sealed_playable/locales/c07_en.json`
- `apps/sealed_playable/locales/c07_id.json`
- `apps/sealed_playable/src/agency.rs` — save/load/reset/step also touch `Supply`
- `apps/sealed_playable/src/agency_desktop.rs` — panel reads `Supply::status`
- `apps/sealed_playable/src/lib.rs`, `locale.rs`, `Cargo.toml`, `Cargo.lock` (`sha2 = 0.11.1`)
- `C0_7_IMPLEMENTATION_SPEC.md`
- `C0_7_ACCEPTANCE_TESTS.md`

Frozen `src/` (the kernel) was not edited.

## Verification actually run

Command, from the repo root, 2026-11-18:

```
cargo test --manifest-path apps/sealed_playable/Cargo.toml --test c07_supply -- --test-threads=1
```

Result: **8 passed, 1 failed.** Debug build. Finished in about 7s compile + 0.07s tests.

Passed:

- `five_deliveries_conserve_twelve_herbs`
- `a_second_demand_does_not_change_the_first`
- `the_same_open_task_is_not_a_second_job`
- `two_herbs_cannot_satisfy_a_demand_of_five`
- `an_unknown_worker_rolls_back`
- `cancelling_the_open_haul_keeps_the_herb_total`
- `save_load_and_replay_match`
- `the_panel_reads_the_checkpoint_not_a_private_balance`

Failed:

- `the_session_panel_follows_the_checkpoint_through_save_and_language` at `tests/c07_supply.rs:153`

The test reaches shrine 5, total 12, and 5 deliveries, then Save, Reset, Load. After Load, `session.supply().checkpoint()` is the reset shrine checkpoint (carrier still holds 12, shrine 0), not the saved completed bytes. Direct `Supply::restore` in `save_load_and_replay_match` does match. The session path is the gap: `Session::act(Action::Load)` either rejects the saved supply bytes or does not install them.

Compiler warning, not a failure: unused `again` in `five_deliveries_conserve_twelve_herbs`.

## Not run

- `tools/c05_verify.ps1` (frozen C0.1–C0.6 gate)
- `cargo fmt` / `clippy -D warnings`
- Native loop evidence under `docs/evidence/c07-native`
- Daylight at tick 720 (explicitly out of this contract)
- Multi-hauler contention (explicitly not claimed)

## Resume

1. Make `Action::Load` restore the supply checkpoint the session just saved, and keep the Indonesian locale assertion.
2. Remove the unused `again` binding.
3. Re-run the `c07_supply` test, then `tools/c05_verify.ps1`.
4. Capture native evidence only after those gates pass.
5. Do not treat this branch as accepted until a separate review.
