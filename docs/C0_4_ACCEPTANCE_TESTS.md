# C0.4 executable acceptance contract

Version 1.0-contract · 18 November 2026 · contract review pending.
Baseline: `14d229bedc308d7e50225122caaec0ce5732a201`.

Automated tests live in `apps/sealed_playable/tests/` (or unit modules for private
helpers); the Bevy adapter must actually run as a Bevy plugin in integration tests,
not only through a substitute plain-Rust driver. Test names map to these IDs.

| ID | Executable procedure / required assertion |
| --- | --- |
| C04-T01 frozen | Git diff baseline for root Cargo.toml/lock/toolchain, src, C0.1–C0.3 normative docs and vectors must be empty. Run root fmt, locked/offline check, strict all-target Clippy, debug and release tests plus C0.3 standalone vector check. |
| C04-T02 fixture/view | Repack demo bytes independently and compare shipped bytes; kernel restore/roundtrip identical; 20 distinct enabled actors, configured channels/homes/inventory/topology correct. After ticks 1–4, assert each of five groups has the expected kernel goal/stages/completion, consumption, durable release, hauled ownership and home position. Read-only projection agrees with canonical record fields and rejects truncation, hostile count and unsupported layout without a panic. |
| C04-T03 determinism | Run 100 ticks through Bevy plugin and direct `Simulation::drive` from identical checkpoint and predetermined command schedule (at least a mode command and disable/re-enable). Compare complete checkpoint, hash, cursor/digest and canonical event bytes after every successful tick. Repeat with 30/60/144-FPS-like and irregular integer frame partitions, plus catch-up >8 ticks drained over frames. No debt is silently lost. |
| C04-T04 controls | Default paused; 100 frame updates change no kernel byte. Single step advances exactly one tick and re-pauses; running step disabled; pause/resume and restart do not admit paused elapsed time. Camera/selection/language actions leave checkpoint/digest unchanged. Invalid drive pauses without publication. |
| C04-T05 persistence | Save/load checkpoint at tick 2 with in-flight tasks and at tick 50; continue to tick 100 with same tick-indexed commands, compare each suffix event and checkpoint to uninterrupted run. Missing, truncated, checksum-bad, oversized or wrong-fixture save fails non-destructively. Saving/replacing and loading actual files succeeds; unwritable destination reports failure and preserves prior save. Load resets host debt/log, pauses and emits no prefix. |
| C04-T06 localization | Both external JSON dictionaries parse, detect duplicate/empty/missing keys, have equal required keys; unknown locale falls back to English, missing key is diagnostic, Indonesian/UTF-8 survive. Switch languages repeatedly before/after ticks and save/load: state hash/digest identical. No hardcoded user-facing UI sentences outside resources. |
| C04-T07 desktop | Build and launch native desktop development binary. Observe window/map/20 selectable NPCs, pan/zoom/reset. At ticks 1–4 inspect NPCs 1–5 and see eat/rest/work/haul/return, authoritative positions, needs/inventory and ordered event inspector. Click pause/resume/step/save/load/reset/language controls; select NPC and verify display. Restore a save and continue. Capture screenshot plus observed actions and any errors; automated GUI input is acceptable, but headless-only tests are not this row. |
| C04-T08 build/performance | App fmt/check/strict all-target Clippy/debug/release tests pass locked/offline after app fetch; native runnable development build present with run instructions. Record basic native startup/backend/hardware and short 20-NPC runtime observations; do not assert the deferred 30-day/R2 gate. |
| C04-T09 review | Independent contract PASS archived before implementation; final independent source/evidence review PASS, all rows above trace to actual results; report, logs, matrix/progress/handoff committed and remote SHA verified. |

Command roots are distinct: root checks use root manifest/lock; app checks use
`--manifest-path apps/sealed_playable/Cargo.toml` and its lock. The app is not a
root workspace member. Run frozen commands even if the app passes.

Final acceptance requires every row; a desktop/GPU/input access limitation must be
reported as incomplete, never replaced with a unit-test PASS.
