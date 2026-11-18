# C0.6 handoff

**Verdict:** PASS. Do not merge to `main`. Do not start C0.7.

**Branch:** `feat/c06-readable-village`  
**Base:** `496045841cedaf0f485ebef4c9e123b3354c6760`  
**Worktree:** `sealed-heaven-c06`

## What landed

Readable Qinghe presentation, a tick-derived clock, schedule text that is explicitly not the simulation, and NPC goal/needs/task from the checkpoint view. English and Indonesian strings are in `locales/c06_*.json`.

## Evidence

- `tools/c05_verify.ps1` exit 0 after the final sprite-color fix
- `docs/evidence/c06-native` foreground PASS
- Independent review closed the healer-color finding

## Next action

None for C0.6. A later milestone can show a daylight frame by advancing to tick 720, or can spread shared-site NPCs further. Do not copy the research roster's private positions into the kernel.
