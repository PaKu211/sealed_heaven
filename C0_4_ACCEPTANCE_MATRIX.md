# C0.4 acceptance matrix — not final acceptance

18 November 2026. Normative rows: `docs/C0_4_ACCEPTANCE_TESTS.md`.

| ID | Current evidence | Status |
| --- | --- | --- |
| T01 | Frozen diff empty; five mandatory root commands and C0.3 packer passed in final verification, 82+2 per profile | Pass |
| T02 | Exact packer, kernel restore/roundtrip, five-group goals/completions, bounded projection tests | Headless pass |
| T03 | Actual Bevy plugin: 100-tick per-tick parity, FPS-like partitions/irregular/catch-up | Pass |
| T04 | Pause/step/UI invariance/failure tests; SR-01 regression and independent closure | Pass |
| T05 | File replacement/preservation, hostile/wrong/large saves, Windows locked destination, tick 2/50 continuation | Pass |
| T06 | 99-key external en/id dictionaries; duplicate/empty/missing/parity/fallback/UTF-8; language-state invariance | Pass |
| T07 | Actual native startup/tick-1 Eat framebuffer archived; complete mouse/toolbar and other controls unverified due input-access blocker | BLOCKED |
| T08 | Separate app fmt/check/strict Clippy/debug/release/build passed (26 tests per profile); basic hardware/paused-memory/build observations recorded; running native sample remains open | Partial |
| T09 | Contract/source-only PASS archived; final milestone acceptance cannot pass with T07 blocked | BLOCKED |

C0.4 is not complete until every row, actual interactive smoke and final
independent acceptance pass. No later milestone is started.
