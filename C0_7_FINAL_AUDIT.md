# C0.7 audit

**Verdict: PASS** for the shrine-supply contract on `56ccb51f6a3ee69bd29e5a35d63b62b9e4976042`.

This is a contract review of that branch tip. It is not a second reader. No separate review session closed these findings.

Branch `feat/c07-shrine-supply`. Base `e4598385291abd2e87133c97aa03ca49e91e1a1d`. `git diff e459838 HEAD -- src Cargo.toml Cargo.lock rust-toolchain.toml` is empty.

| Gate | Result |
|---|---|
| Conservation, five deliveries, duplicate demand, duplicate task, insufficient stock, unknown worker, cancellation | `c07_supply` tests, included in the verify run below |
| Save, load, replay, and the session path | `save_load_and_replay_match` and `the_session_panel_follows_the_checkpoint_through_save_and_language` |
| No second herb balance | Quantities in `Supply::status` come from `View` and `resource_stock_quantity`, and the two must agree. The desk stores demand, the open task id, and observation counts |
| Frozen C0.1–C0.6 | `tools/c05_verify.ps1` exit 0 on this commit, 2026-11-19 17:10 +07. Log copy: `C0_7_VERIFICATION_LOG.txt`. `C0_5_VERIFICATION_LOG.txt` restored. The older log in the commit recorded `8d5cf71` and does not cover this tip |
| Visible supply state | Native capture `c05-019`: Delivery 5/5, carried 7, shrine 5, total 12, Completed. `c05-031` after reset: Delivery 0/5, carried 12, shrine 0, Requested. `c05-024` shows "Pasokan kuil" |
| One hauler | The scenario checkpoint has actor 1 only. A second hauler is not claimed |
| Haul stops at demand 5 | After completion, `step` does not drive. The kernel would otherwise move the remaining 7 herbs |

## Findings that do not fail the contract

The lifecycle label is host state rebuilt by replaying empty drives. Herb counts are read from the checkpoint on every status call. A saved hauler file does not contain the label; load fails if replay cannot reach those exact bytes.

Load now requires both the C0.5 village file and `shrine-supply.c03`. A village save from before this desk has no sibling file, so that load fails and the live session stays put.

`docs/evidence/c07-native` was captured from the Bevy window. On this commit the default `sealed_agency` binary is the ASCII map. `tools/c05_native.ps1` does not pass `--window`, so that command no longer opens the window that produced the captures. The captures still show the supply panel from the run that created them. Repeating the window loop on this commit requires `sealed_agency --window`.

The ASCII client steps one tick per command. The 2 Hz clock remains on the window client. Daylight at tick 720 and multi-hauler contention stay outside the contract.
