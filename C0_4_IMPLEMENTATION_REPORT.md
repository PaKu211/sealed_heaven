# C0.4 implementation report — accepted

18 November 2026. Branch `feat/c04-first-playable`; accepted C0.3 baseline
`14d229bedc308d7e50225122caaec0ce5732a201`.

## Delivered source

- Independent `apps/sealed_playable` package/lock; pinned minimal-feature Bevy
  0.17.3 and unchanged toolchain. Root manifest/lock/source are untouched.
- Standalone initial checkpoint packer exactly reproduces the accepted S4
  mixed-goal twenty-actor fixture. No second simulator or kernel API change.
- `view.rs`: bounded read-only projection from public canonical bytes; kernel
  validates public ingress. Fixture-1/2 legacy views are never misused.
- `lib.rs`: actual Bevy plugin, integer fixed-tick debt/catch-up, paused startup,
  explicit action/command ordering, candidate publication, bounded event suffix.
- `persistence.rs`: validated/bounded I/O, sibling-temp sync/atomic replacement,
  preserves old save on failure (including tested Windows file locking).
- `locale.rs` and external JSON: 99 stable keys, English/Indonesian,
  duplicate/empty/missing/parity/fallback tests and UTF-8 validation.
- `desktop.rs`: actual native 2D map, selectable twenty-NPC markers, authoritative
  inspector/needs/goals/inventory/stock owners/events/hashes, clickable controls,
  camera, safe UI scene resync on load. Licensed Unicode font bundled.
- Reproducible root/app verification and targeted native window-message smoke
  helpers. Opt-in F12 captures the actual Bevy framebuffer and read-only QA JSON,
  never another application or an OS desktop image.

## Verification and review

Independent contract PASS preceded implementation. Independent source review
found SR-01 (ignored running-step dropped elapsed time), reproduced separately.
A regression failed before correction and passes after it; independent re-review
closed SR-01. Source-only PASS is archived, not mislabeled final acceptance.

Current headless results: 17 unit + 6 integration tests pass; fixture repack and
frozen-baseline diff pass. 100-tick adapter/headless parity compares complete
checkpoint/event bytes/digest/cursor after every tick across multiple FPS-like
partitions, including catch-up debt. Tick-2/tick-50 save/load resumes identically.

Final automated root and separate default-desktop verification passed: root
82 unit + 2 doctests per profile; app 17 library + 3 desktop + 6 integration tests
per profile; both fmt/check/strict Clippy and packers pass. Native debug and release
executables exist. The final log is `C0_4_VERIFICATION_LOG.txt`.

Actual native framebuffer evidence shows twenty actors, readable UTF-8 text,
roads/grid and the kernel's tick-1 Eat/Executing state. The earlier foreground
blocker is closed by a later foreground mouse and keyboard run. Forty-nine
game framebuffer pairs, the action log, and the performance sample are under
`docs/evidence/c04-native-acceptance/`. Partial screenshots under
`docs/evidence/c04-native-partial/` are historical and are not the acceptance
evidence. Independent final acceptance is `C0_4_FINAL_INDEPENDENT_AUDIT.md`.

## Basic performance observations and limitations

Native renderer reported NVIDIA GeForce RTX 4060 / Vulkan on Windows, i7-12700F,
approximately 16 GiB system RAM. An intermediate paused twenty-NPC debug sample
reported working set 497,659,904 bytes (about 475 MiB). A later foreground debug
run of the same twenty NPCs advanced 21 ticks in 14.558 seconds (1.44 ticks per
second against the 2 Hz clock), used 16.078 seconds of process CPU, and had a
peak working set of 534,978,560 bytes. Cold full Bevy app release verification
built dependencies in 9m17s; the cached repeat rebuilt the app in 12.48s. Final
local debug/release executable sizes are 84,369,920 and 46,057,984 bytes. This
is a short debug observation, not an FPS benchmark. No modest-hardware
guarantee, 30-day soak, or R2 acceptance is claimed.

## Boundaries

Stored needs do not decay/reset; resources are not automatically replenished.
Later goals can differ from first tasks. Position spacing is presentation only;
cost-1 movement is not smooth physics. Foreign large grids draw at most a 32×32
corner to bound presentation work; the entire approved 3×3 demo is visible.
No migration, new economy/AI, multiplayer, 3D, LOD policy or R2/30-day claim.

Run instructions: `apps/sealed_playable/README.md`. Runnable development output
is under the app's ignored `target/debug/`; source, assets and lock are deliverable,
not a claim that build artifacts are committed to Git.
