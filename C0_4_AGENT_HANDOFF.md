# C0.4 handoff

Branch: `feat/c04-first-playable`; based on accepted `14d229b`.
Current stage: implementation and automated checks complete; independent contract
and source-only PASS. SR-01 closed. Native input BLOCKED; milestone NOT accepted.
Root kernel and dependencies remain frozen; app has its own Cargo.lock.

Next exact action: resolve native foreground/input access or obtain manual owner
evidence using `C0_4_NATIVE_INPUT_BLOCKER.md`. Complete C04-T07 and the running
performance observation, then independent final acceptance. Do not restart
completed implementation or claim PASS from existing partial native evidence.

Current verified headless results: 17 unit + 6 integration tests pass; 100-tick
parity at every tick (30/60/144-like/irregular frame partitions and catch-up),
save/load tick 2/50, exact authored fixture and locales. Latest native debug/release
builds and automated default-desktop verification have now passed.

Dedicated app lock; no kernel edits. GUI runtime uses Noto Sans (OFL bundled).
Source-only review: `C0_4_SOURCE_INDEPENDENT_REVIEW.md` includes source hashes.
Final automated verification: root 82+2 debug/release; default app fmt/check/Clippy,
build and 17+3+6 tests in BOTH profiles passed. Log complete.
Debug executable SHA256: 302a3b3a3bf09ab67bde09865cf192596045a17a0d0bcbf68b9bcea716c80aa7.
Snapshots under `docs/evidence/c04-native-partial/` are NOT final acceptance.
Git: `C:/Program Files/Git/cmd/git.exe`; Cargo: `C:/Users/PC_4/.cargo/bin/cargo.exe`.
