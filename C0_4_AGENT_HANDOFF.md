# C0.4 handoff

Branch: `feat/c04-first-playable`; based on accepted `14d229b`.
Current stage: C0.4 accepted. Independent contract, source, and final audits
PASS. SR-01 closed. Native C04-T07 closed with foreground mouse and keyboard
evidence under `docs/evidence/c04-native-acceptance/`. Root kernel and
dependencies remain frozen; app has its own Cargo.lock.

Next exact action: none for C0.4. Do not start C0.5 from this handoff. Do not
treat `docs/evidence/c04-native-partial/` or the old input-blocker note as the
current verdict. The verdict is `C0_4_FINAL_INDEPENDENT_AUDIT.md`.

Current verified headless results: 17 unit + 6 integration tests pass; 100-tick
parity at every tick (30/60/144-like/irregular frame partitions and catch-up),
save/load tick 2/50, exact authored fixture and locales. Latest native debug/release
builds and automated default-desktop verification have now passed.

Dedicated app lock; no kernel edits. GUI runtime uses Noto Sans (OFL bundled).
Source-only review: `C0_4_SOURCE_INDEPENDENT_REVIEW.md` includes source hashes.
Final automated verification: root 82+2 debug/release; default app fmt/check/Clippy,
build and 17+3+6 tests in BOTH profiles passed. Log complete.
Debug executable SHA256: 302a3b3a3bf09ab67bde09865cf192596045a17a0d0bcbf68b9bcea716c80aa7.
Snapshots under `docs/evidence/c04-native-partial/` are historical only.
Acceptance frames are under `docs/evidence/c04-native-acceptance/`.
Git: `C:/Program Files/Git/cmd/git.exe`; Cargo: `C:/Users/PC_4/.cargo/bin/cargo.exe`.
