# C0.4 acceptance matrix — final acceptance

18 November 2026. Normative rows: `docs/C0_4_ACCEPTANCE_TESTS.md`.
Verdict and frame-level evidence: `C0_4_FINAL_INDEPENDENT_AUDIT.md`.

| ID | Current evidence | Status |
| --- | --- | --- |
| T01 | Frozen diff empty; `verify.ps1` repeated on `0cc47cd`; five mandatory root commands and C0.3 packer passed, 82+2 per profile | Pass |
| T02 | Exact packer, kernel restore/roundtrip, five-group goals/completions, bounded projection tests | Pass |
| T03 | Actual Bevy plugin: 100-tick per-tick parity, FPS-like partitions/irregular/catch-up | Pass |
| T04 | Pause/step/UI invariance/failure tests; SR-01 regression and independent closure | Pass |
| T05 | File replacement/preservation, hostile/wrong/large saves, Windows locked destination, tick 2/50 continuation | Pass |
| T06 | 99-key external en/id dictionaries; duplicate/empty/missing/parity/fallback/UTF-8; language-state invariance | Pass |
| T07 | Foreground mouse and keyboard run of the debug binary. 49 framebuffer pairs in `docs/evidence/c04-native-acceptance/`. Tick 1 goals, save/load hash match, arrivals, locale, camera, toolbar, pause hold, and tick-0 reset checked | Pass |
| T08 | App fmt/check/strict Clippy/debug/release/build passed (26 tests per profile). Short foreground 20-NPC debug sample recorded; 30-day and R2 gates not claimed | Pass |
| T09 | Contract PASS, source PASS with SR-01 closed, this matrix, and `C0_4_FINAL_INDEPENDENT_AUDIT.md` | Pass |

C0.4 acceptance is limited to these rows. No 30-day soak, R2 performance gate,
or later milestone is accepted.
