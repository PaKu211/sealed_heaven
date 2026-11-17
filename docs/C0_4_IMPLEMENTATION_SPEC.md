# C0.4 — First Playable Bevy Vertical Slice

Version 1.0-contract · 18 November 2026
Status: owner scope approved; independent contract review pending. Implementation
authorization becomes effective only after that review passes.

## 1. Authority and frozen baseline

Owner decision: C0.4 First Playable Bevy Vertical Slice, 18 November 2026.
Accepted baseline: `14d229bedc308d7e50225122caaec0ce5732a201`.
C0.1–C0.3 source, Cargo.toml, Cargo.lock, toolchain, normative contracts,
canonical vectors and behavior remain unchanged. No kernel API extension is
required. `main` is not a delivery branch. C0.5/R2 are not authorized.

## 2. Boundary and crate layout

The repository has one non-workspace `sealed_sim` crate. Add the independent
application package `apps/sealed_playable` with its own manifest/lock, using
`sealed_sim = { path = "../.." }`. Do not convert the root into a workspace.
Bevy and JSON/localization dependencies belong only to the application.

The public C0.3 API exposes `drive`, checkpoint restore/bytes/hash, event output,
tick and digest, but does not expose twenty-actor construction or complete
fixture-3 state views. Therefore the application:

1. Loads an authored revision-1 C0.3 initial checkpoint through
   `Simulation::restore_domain_checkpoint` (the kernel validates it).
2. Calls only `drive(current_tick, batch)` to advance the authoritative world.
3. Builds a bounded, read-only presentation projection from public canonical
   checkpoint bytes using the frozen documented schema. It does not simulate,
   alter or write those projected records back to a running simulation.
4. Never uses fixture-1/2 convenience accessors as fixture-3 state inspection.

Separate application modules cover: adapter/tick driver, read-only projection,
presentation/UI, localization, and persistence. Only the adapter owns `Simulation`.
All game decisions, stock transfers, needs, paths, stages and event ordering come
from the kernel. Rendering entities and visual offsets have no authority.

## 3. Authored demonstration

The initial checkpoint exactly reproduces the initial `World` of accepted test
`c03_s4::twenty_living_actors_run_all_five_goals_in_both_modes_and_resume_every_tick`:
seed 100, actors 1–20, no tasks, wake 1, goal channel `(id-1)%5` at 1000,
other channels/roles/relationship 0; homes 2 for rest/haul/return, otherwise 1;
actor-owned quantity-1 food definition 1 for eat and cargo definition 7 for haul;
stocks 101/102 definitions 4/5 class 2 quantity 4 at site 1; cursor stock 103.
All other constructor defaults, graph, holders, grid, cursors and H0 are frozen
C0.3 values. A standalone packer records its provenance and reproducibly checks
the shipped bytes. This is fixture initialization, not a new gameplay rule.

The scene shows all 20 actors. IDs 1–5 demonstrate eat/rest/work/haul/return;
all five groups complete their first task within four successful ticks. No need
decay, artificial resupply, extra edge or animation-driven travel is introduced.
Needs display the stored channels; absence of decay is stated in the help.

Authored screen coordinates for sites 1, 2, 9 are presentation-only. Site/grid
loci and mid-edge ratios derive from canonical position. Small deterministic ID
offsets prevent overlap at a site; the inspector shows exact canonical position.
Cost-1 travel may arrive in a single tick; do not claim smooth physics.

## 4. Adapter and deterministic time

Startup is paused. Run uses a fixed 500,000,000 ns per logical tick. Rendering
elapsed time controls only how many `drive` calls are due, never kernel inputs.
Integer accumulation carries unspent debt; limit catch-up per frame (8 ticks)
without discarding debt. Frame partitioning at equal admitted tick/command
boundaries must produce identical state and events. No random/frame-time payloads.
Tests can provide deterministic elapsed durations directly to the Bevy adapter.

UI actions are processed in arrival order before timed ticks. Pause stops time
admission and clears presentation debt; resume starts a fresh interval. While
paused, single-step advances exactly one logical tick. While running, step is
disabled. A failed drive publishes no events/state, pauses and reports the error;
no retry loop silently drops commands. UI controls initially need no gameplay
command; any admitted kernel command must have producer 1, explicit target tick,
actor and stable sequence, and be reused identically by comparison drivers.

Camera, selection, language and UI activity do not touch `Simulation`. The event
inspector retains a bounded recent suffix in kernel order, showing tick/actor,
kind, raw IDs/payload, digest and checkpoint hash. Logs are presentation history,
not a replacement canonical event stream.

## 5. Interactive desktop surface

Provide a native 2D Bevy window, simple authored sites/roads/grid, 20 colored NPC
markers, pan/zoom/reset camera, mouse NPC selection (and keyboard cycle fallback),
and a visible inspector with enabled/mode, exact position, five need channels,
current task goal/stage (or none), actor inventory, stock/holder ownership and
recent events. All displayed values originate from a validated kernel checkpoint.

Controls must be visible: pause/resume, single step, save, load, reset demo,
language English/Indonesian, selection and camera. Keyboard controls may accompany
clickable toolbar controls. Reset restores the authored initial checkpoint and
pauses. Desktop/window/GPU absence is a verification blocker, not a PASS.

## 6. Persistence

Save exact public domain checkpoint bytes to a documented application save path,
not the source/fixture directory. Write a temporary sibling then atomically
replace the old save where supported; report I/O failure without claiming saved.
Load is bounded (16 MiB maximum), validated by the kernel and projection before
replacing live state. A bad/missing/unsupported save leaves live state unchanged.
Successful load pauses, clears time debt and the presentation event suffix, and
retains the canonical cursor/digest without re-emitting prefix events. UI-only
language/selection/camera are not in the canonical checkpoint. No migration.

## 7. Localization

External UTF-8 flat JSON key/value dictionaries: `locales/en.json`, `id.json`
within the application. English is source/fallback; Indonesian is selectable.
Stable dotted keys cover user-facing labels, goals/stages, events, help and errors.
Values/numeric canonical IDs/hashes are data, not translated IDs. No credentials.
Validate JSON, key parity, nonempty values, duplicate keys, required keys and
fallback/missing-key behavior. Unknown language falls back to English; an unknown
key displays its key as a diagnostic. Locale switching must not change kernel
bytes/digest. Preserve UTF-8 (including non-ASCII validation text).

## 8. Dependency and performance policy

Select a pinned Bevy release compatible with the unchanged Rust 1.100.0 toolchain.
Disable default Bevy features; enable only native window/input, 2D sprite/text/UI,
fonts and required renderer/platform features. No audio, 3D, networking or assets
pipeline is necessary. Commit the app lock separately; root lock is frozen.
Network fetch for app dependencies is allowed, then use locked offline commands.
Record native debug/release build, hardware/render backend, startup observations
and a short 20-NPC measurement, with methodology and limitations. No universal
performance threshold or 30-day performance/R2 acceptance is claimed.

## 9. Milestone exit

All `docs/C0_4_ACCEPTANCE_TESTS.md` rows must pass, including an actual native
interactive smoke test; unit/headless evidence alone cannot close C0.4.
Independent contract review precedes implementation. Final independent review
must inspect source and verification evidence and issue its own verdict. Record
limitations honestly; no self-issued independent PASS. Maintain progress, handoff,
execution log, implementation report and final acceptance matrix. Commit/push
verified development increments without force push or merge to main.
