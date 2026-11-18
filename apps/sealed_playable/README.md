# Sealed Heaven — First Playable

**Current acceptance: C0.4 independent audit PASS.**
Native evidence is `../../docs/evidence/c04-native-acceptance/`. The verdict is
`../../C0_4_FINAL_INDEPENDENT_AUDIT.md`. This does not accept the 30-day or R2 gate.

Native 2D Bevy presentation of the **unchanged C0.3 kernel**. The app starts
paused with the exact accepted S4 mixed-goal twenty-NPC fixture. No external game
assets or credentials are needed. Placeholder markers are not physics bodies.

## Build and run (from repository root)

Rust 1.100.0 remains the repository toolchain. Bevy 0.17.3 is pinned in this
separate package; root `Cargo.lock` is not changed.

```powershell
cargo fetch --manifest-path apps/sealed_playable/Cargo.toml --locked
cargo run --manifest-path apps/sealed_playable/Cargo.toml --locked --offline
```

On this development machine, Cargo is at
`C:/Users/PC_4/.cargo/bin/cargo.exe` if it is not on PATH. The runnable debug build
is `apps/sealed_playable/target/debug/sealed_playable.exe`. It embeds the authored
checkpoint and both external JSON dictionaries at build time, so the executable
can run from another working directory. Editing a dictionary requires rebuilding.

Use the visible toolbar/help for keyboard bindings. Select NPCs 1–5 and step
through ticks 1–4 to observe each first goal: eat, rest, work, haul, return home.
IDs repeat these groups four times. Inspect the exact position and task stage:
actors sharing a site have small visual spacing offsets only. Needs do not decay
or reset and food/cargo are not replenished; subsequent goals reflect the frozen
kernel's actual eligibility, not a looping animation script.

Save writes exact C0.3 bytes to `%LOCALAPPDATA%/SealedHeaven/checkpoint.c03` on
Windows (path shown in the app). Atomic sibling replacement preserves the old
save on failure; no migration. Load validates before replacing live state,
pauses and clears host time debt/recent-event suffix. Language and camera are
presentation-only. File rename is not a power-loss durability guarantee.

## Verification

```powershell
powershell -ExecutionPolicy Bypass -File apps/sealed_playable/tools/verify.ps1
```

This runs frozen root checks and separate default-desktop app checks. To test the
same Bevy simulation plugin without native rendering dependencies enabled:

```powershell
cargo test --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --locked --offline
python apps/sealed_playable/tools/pack_demo.py --check
```

Headless tests do not replace the native interactive smoke or independent final
review. See `docs/C0_4_ACCEPTANCE_TESTS.md`, project verification log and acceptance
matrix for actual gate status. No 30-day/R2 performance claim.

Native QA script (requires safe foreground/mouse access) is
`tools/native_smoke.ps1 -EvidenceDirectory <fresh-absolute-folder>`. Its
`-KeyboardOnly` option is partial evidence only. For operator screenshots, set
`SEALED_HEAVEN_EVIDENCE_DIR` before launch and press F12; captures come directly
from the game framebuffer. The script does not send keys to other applications.
