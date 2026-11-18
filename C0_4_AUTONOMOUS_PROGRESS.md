# C0.4 autonomous progress

18 November 2026. Owner approves First Playable Bevy Vertical Slice conditionally
after independent contract review. Baseline `14d229bedc308d7e50225122caaec0ce5732a201`.
Branch `feat/c04-first-playable`. Main and accepted kernel untouched.

Contract and executable acceptance criteria drafted in `docs/C0_4_*`.
Independent architecture/contract review PASS: `C0_4_CONTRACT_INDEPENDENT_REVIEW.md`.
No material contradiction; conditional implementation authorization effective.
Implemented separate `apps/sealed_playable` package with its own lock and Bevy
0.17.3, exact authored S4 checkpoint, read-only projection, Bevy tick plugin,
safe save/load, 99-key en/id catalogs, native UI and 20-NPC map.

Independent source review initially found SR-01: ignored running-step dropped
frame time. Reproduced, corrected, regression added; independent re-review closes
SR-01 and records source-only PASS in `C0_4_SOURCE_INDEPENDENT_REVIEW.md`.
17 unit + 6 integration headless tests pass. Actual native window/framebuffer
observed, but initial glyph/road issues required presentation corrections.

Full verification completed: frozen-root five mandatory commands passed (82 unit
+ 2 doctests in debug/release); both canonical packers passed; separate default
desktop app fmt/check/strict Clippy/debug/release/build passed (17 library + 3
desktop + 6 integration tests in each profile). Runnable debug/release outputs
exist. `C0_4_VERIFICATION_LOG.txt` records the final automated pass.

Native gate BLOCKED by unavailable reliable foreground/input access. Partial
game-only framebuffer evidence shows startup and actual tick-1 Eat/Executing;
full mouse/toolbar and remaining interactions did not pass. Guarded foreground
attempts were refused. Do not substitute headless tests for C04-T07.

Next: safe interactive desktop access or owner/operator evidence per
`C0_4_NATIVE_INPUT_BLOCKER.md`, complete native smoke/performance, then independent
final acceptance. C0.4 is NOT milestone PASS. No later milestone authorized.
