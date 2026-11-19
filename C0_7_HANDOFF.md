# C0.7 handoff

**Verdict:** Contract audit PASS in `C0_7_FINAL_AUDIT.md` for `56ccb51f6a3ee69bd29e5a35d63b62b9e4976042`. That review is not a second reader. Not merged by this pass.

**Branch:** `feat/c07-shrine-supply`  
**Base:** `e4598385291abd2e87133c97aa03ca49e91e1a1d`  
**Worktree:** `sealed-heaven-c06`  
**Date:** 2026-11-19

## Fix

Session save/load sent the hauler checkpoint through the C0.5 `World::restore` validator, so the C0.3 ledger file was rejected and Load kept the reset shrine. The village file still uses `agency::persistence`. The hauler file uses `crate::persistence` (`View::validated`).

After the haul that meets demand, the desk marks Completed on that same step. The panel no longer stays on "In progress" once the shrine quantity is 5 and the task id is 0.

## Verification

`cargo test --manifest-path apps/sealed_playable/Cargo.toml --test c07_supply --test agency -- --test-threads=1`

9 supply tests passed, including `the_session_panel_follows_the_checkpoint_through_save_and_language`. 9 agency tests passed. Repeated after the phase-label change.

`tools/c05_verify.ps1` exit 0 after the save/load fix. Output copied to `C0_7_VERIFICATION_LOG.txt`. `C0_5_VERIFICATION_LOG.txt` restored. The five-line phase update came after that script; it is covered by the supply tests and by:

`cargo clippy --manifest-path apps/sealed_playable/Cargo.toml --all-targets --locked --offline -- -D warnings`

exit 0.

Native foreground loop, release `sealed_agency`, fresh directory:

`powershell -NoProfile -File tools/c05_native.ps1 -EvidenceDirectory docs/evidence/c07-native -Profile release`

exit 0. Gameplay log ends in PASS. Capture `c05-019` shows Delivery 5/5, carried 7, shrine 5, total 12, Completed. Capture `c05-031` after reset shows Delivery 0/5, carried 12, shrine 0, Requested. Indonesian capture `c05-024` shows "Pasokan kuil".

Kernel `src/` was not edited.
