# C0.4 native-input access blocker

18 November 2026. **C0.4 is NOT accepted.** All automated checks pass, but C04-T07
requires reliable mouse/toolbar, keyboard, save/load and locale interaction.

## Observed evidence

- The actual development executable launches a native Bevy window and renders all
  twenty NPCs, the authored graph/grid and canonical inspector using Vulkan on
  NVIDIA GeForce RTX 4060.
- Actual Bevy framebuffer images/JSON are under `docs/evidence/c04-native-partial/`.
  The second capture is tick 1 after a native single-step key, showing NPC 1's
  Eat/Executing state and changed kernel checkpoint hash.
- The harness now rejects minimized/small images and waits for asynchronous PNG
  completion. Neither incomplete PNGs nor minimized black images count as evidence.
- Posted mouse messages did not establish toolbar interaction. Guarded
  `SetForegroundWindow` and scoped `AttachThreadInput` attempts did not obtain
  foreground input. The helper refused input rather than targeting another app.
- A keyboard-only attempt reached tick 1; a later capture timed out during
  selection. This is partial evidence, not a complete interactive smoke.
- No complete native smoke or running-window performance sample is claimed.
  These limitations do not establish that the unverified controls are correct.

This is an essential desktop/input-access blocker, not a missing specification
or request for generic implementation authorization. No system-wide focus or
security setting was changed. Safe complete automation requires reliable input.

## Runnable build

From the repository root:

```powershell
.\apps\sealed_playable\target\debug\sealed_playable.exe
```

Release output also exists under the app's `target/release/`. Build/run instructions
and bindings are in `apps/sealed_playable/README.md` and in the native window.
Debug artifact SHA-256 used for partial evidence:
`302a3b3a3bf09ab67bde09865cf192596045a17a0d0bcbf68b9bcea716c80aa7`.
Build artifacts are local/ignored, not committed binaries.

## Required access / exact continuation

Provide an interactive desktop where the agent can focus the game safely, or have
the owner/operator perform C04-T07 and preserve evidence. No kernel change needed.

Manual operator checklist (initial state paused):

1. Click Single step: tick becomes 1. Mouse-select NPCs 1–5 and observe Eat,
   Rest, Work, Haul, Return Home, current stages, needs and inventory.
2. Pan/zoom with WASD/arrows and Q/E; C resets camera; Tab cycles selection.
3. Step to tick 2, Save, step to 3, Load: exact tick-2 hash returns and app pauses.
   Step again: tick-3 hash must equal the first tick-3 hash.
4. Step to 4; inspect arrivals, stock ownership and ordered events (scroll panel).
5. Switch English/Indonesian: labels change, kernel hash/digest do not.
6. Resume, then Pause; wait and confirm tick/hash stay fixed. Reset restores
   paused tick 0. Verify every clickable toolbar control, not only shortcuts.
7. Preserve game-only screenshots/errors and a short running performance sample.
   Set `SEALED_HEAVEN_EVIDENCE_DIR` before launch to enable F12 framebuffer capture.
8. Independent final review must examine the resulting evidence/delivery commit.

Retry the automated mouse smoke with a fresh absolute evidence folder when access
exists: `powershell -ExecutionPolicy Bypass -File
apps/sealed_playable/tools/native_smoke.ps1 -EvidenceDirectory <folder>`.
`-KeyboardOnly` is explicitly partial evidence and cannot close C0.4.
