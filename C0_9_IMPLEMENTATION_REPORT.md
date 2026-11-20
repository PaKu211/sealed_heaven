# Sealed Heaven C0.9 — implementation handoff

**Date:** 19 November 2026

**Branch:** `feat/c09-world-memory` (local)

**Base:** `1b410e0` local C0.8 compatibility correction, whose parent is the published `09534cb`

**Scope:** One witnessed/unwitnessed combat death, delayed rumor, healer attitude, versioned save/load, ASCII EN/ID feedback.
**Gate status:** **PENDING / NOT EXECUTED** — this environment has no Cargo or Rust toolchain; do not claim tests or Clippy passed.

## Implementation

- `src/player5/memory.rs`: fixture-scoped incident, exact witness eligibility, two-tick report, stable memory event IDs and digest, and standing 0/-1; no runtime model API or randomness.
- `src/player5/codec.rs`: revision-3 checkpoint tail and stricter validation, using exact replay to derive memory event history. Revision-1/2 writers remain unchanged.
- `src/player5/memory_tests.rs`: witnessed death, pending/report save/load, unwitnessed westward kill, retreat, tampered save, empty-tick progression and dead-player continuity.
- `apps/sealed_playable/src/agency.rs` and `ascii.rs`: distinct new scenario and reset semantics, English/Indonesian social status, pending incident kept hidden; C0.8 session and `--c08` command-line mode retained.
- `apps/sealed_playable/tests/c09_memory.rs`: UI-only no-tick, localized report, save/load, supply 12-herb conservation, reset and direct/script parity.
- `C0_9_CHECKPOINT_AMENDMENT.md`: exact byte-level revision-3 contract.

## Required verification when a Rust toolchain is available

```text
cargo test --lib player5
cargo test --manifest-path apps/sealed_playable/Cargo.toml --test c09_memory --test c08_conflict --test c07_supply
cargo clippy --all-targets -- -D warnings
cargo clippy --manifest-path apps/sealed_playable/Cargo.toml --all-targets -- -D warnings
tools/c05_verify.ps1   # PowerShell on Windows; preserve the accepted C0.5 log
cargo run --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency
```

For a scripted test, `SEALED_ASCII_COMMANDS` may contain `east east east east east f f n n`. The legacy revision-2 client remains available with `--c08`; the Bevy window is unchanged and still has no combat keys. A distinct five-minute human ASCII playtest remains necessary.

## Known limits and stop conditions

- **No build/test proof yet.** Inspect the source and execute the commands above before calling C0.9 accepted.
- Existing C0.8 correction `1b410e0` has not been pushed to GitHub; do not silently rebase this branch on older `main`.
- The death/witness fixture does not implement real LOS, a routing-based rumor network, broad faction politics, NPC pathfinding revisions, or the cross-generation successor system.
- The existing two-file C0.7 save remains non-atomic across a crash, and no third sidecar was added.
- Any compiler or replay mismatch requires a targeted correction and new evidence rather than weakening the acceptance assertions.
