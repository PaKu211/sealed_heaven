# C0.5 — Qinghe herb delivery slice

From the repository root (Rust/toolchain and locked dependencies unchanged):

```powershell
cargo run --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency --release --locked --offline
```

The original C0.4 binary is still the default `sealed_playable`; no replacement.
C05 starts paused at the settlement entrance. Space resumes; WASD/arrows move
one cell per committed command (hold repeats). E interacts with the selected
target; Tab cycles targets or click a marker/object. N single-steps while paused.
F5 saves, F9 loads (paused), L changes English/Indonesian, R resets. The toolbar
offers the same actions; camera follows the player and C centers the view.

Talk adjacent to Healer Yun (blue NPC1); find the spirit-herb patch east of the
gate; gather one herb; return and deliver to the shrine south of Yun. The right
panel shows the player's physical inventory and both world stocks. Gold is the
player; small blue markers are autonomous NPCs. Marker offsets are cosmetic;
interaction/collision uses the canonical cells printed in the inspector.

Paused movement/interactions queue without changing the world; use N or resume.
Queue limit32, one action per tick, nominal clock2 Hz. Capacity4096 unique actions
per timeline is a bounded prototype limit (reset starts a new timeline). NPCs keep
the accepted abstract/shared site movement: the player cannot enter NPC-occupied
cells, but an NPC can later arrive over the player, who may leave.

The prototype stops at tick4096 (about34m08s at2 Hz); save/history validation is
bounded by this horizon. Reset starts a new attempt. This is not a long-run world
gate. The completed herb delivery can be saved and restored.

C05 saves to `LOCALAPPDATA/SealedHeaven/checkpoint.c05` (same fallback base as C04),
never the old `.c03` path. Queued unadmitted inputs, language, selection and camera
are not saved; load/reset clears queues. No implicit old-save migration.

Verification: `tools/c05_verify.ps1` at repository root. Native procedure:
`tools/c05_native.ps1 -EvidenceDirectory <fresh-absolute-folder> -Profile release`.
Native evidence/independent acceptance status is in the root C0_5 reports, not
inferred from compilation. Set `SEALED_HEAVEN_EVIDENCE_DIR` before launch and F12
captures the actual game framebuffer and read-only diagnostics.
