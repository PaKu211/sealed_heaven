# C0.4 independent source review

Date: 18 November 2026

Reviewer: independent source-review session; no application implementation or fix authored

Verdict: **PASS — source review only; SR-01 closed**

**This is not final C0.4 milestone acceptance.** Native interactive smoke,
rendered UTF-8/visibility/input verification, performance observations, the full
verification results, and final evidence/commit/remote reconciliation still need
their own review. No native interactive result is asserted here.

## 1. Scope and reviewed identity

- Worktree: `D:\projek\sealed\Sealed_Heaven_C0_1_Source (1)\sealed-heaven-c04`.
- Branch observed: `feat/c04-first-playable`.
- HEAD observed: `858b044c203dcbc8ef9b2b2720865f0113d12c10`.
- Frozen baseline: `14d229bedc308d7e50225122caaec0ce5732a201`.
- The application was untracked at review time. This verdict covers the inspected
  working-tree content, not a claim that HEAD already contains the application.
- Authority: `docs/C0_4_IMPLEMENTATION_SPEC.md`,
  `docs/C0_4_ACCEPTANCE_TESTS.md`, and the previously archived contract-only PASS
  in `C0_4_CONTRACT_INDEPENDENT_REVIEW.md`.
- Inspected application modules, integration/unit tests, dictionaries, fixture
  packer, manifest, persistence behavior, desktop scene/input/paint systems,
  bundled font provenance/license, and the opt-in F12 capture path. Compared the
  projection/fixture against the frozen kernel encoding and accepted S4 test.

Selected SHA-256 identities of the reviewed content:

| File | SHA-256 |
| --- | --- |
| `apps/sealed_playable/src/lib.rs` | `cc0be14869f9487b754477c6c9fa72a4bb058ddaab60db6d6f2674e43eb125ea` |
| `apps/sealed_playable/src/desktop.rs` | `e2b0238f485010ff663ad5609c9737e71af998e2aa6c38f7220a919ae59d1a60` |
| `apps/sealed_playable/src/view.rs` | `eae6c576ea2dc9047b63692222a6dac57eac38ab7b08faadf32bf4223396d9ae` |
| `apps/sealed_playable/src/persistence.rs` | `482c9527f3f41bec8ec9d7c3246d12f59eab13ba72d3767f71a35acc8d2798b2` |
| `apps/sealed_playable/src/locale.rs` | `72cfa5b7e8735ecfcbef55b8c575367b555da8d975487ddcd43835ee3fc0a58c` |
| `apps/sealed_playable/tests/adapter.rs` | `8df04a9447f9888cec1409e31196943755d645a9abfdbd8f123e2554452cafd4` |
| `apps/sealed_playable/Cargo.toml` | `b718e3e72d612682c421b6e06c00220dede8fa7bb934b6d6041c8a021ef56296` |
| `apps/sealed_playable/Cargo.lock` | `efaa8dc38173ae9909a6aefa7868ffc625aed482252262020b9a20860bb2fd08` |

Material subsequent source changes require a review delta.

## 2. Finding and independently checked closure

### SR-01 — P2: ignored running-step discarded admitted frame time — CLOSED

Original location: `apps/sealed_playable/src/lib.rs`, `drive_frame`, lines
233–242. A step action was unconditionally classified as a transport transition,
even though `Session::act` ignores stepping while running. Consequently the
frame's elapsed time was discarded and existing debt was not drained that frame.

Independent pre-fix reproduction used the actual Bevy `SimulationPlugin` from an
external scratch probe, without changing repository source: resume, then supply
ten frames of 500,000,000 ns. Without actions the session reached tick 10; with
an ignored `Action::Step` every frame it stayed at tick 0, debt 0, still running.
The native input systems filtered this action already; the demonstrated defect
was in the shared adapter API, not an asserted native-input reproduction.

The implementation author changed transport classification so `Action::Step`
suppresses admission only when `session.paused` is true **before that action is
applied** (`lib.rs:235–239`). Independent inspection confirms this is evaluated
sequentially for queued actions: running steps are inert, while effective paused
steps and actual transport transitions retain fresh-interval semantics.

Independent execution of
`c04_t04_ignored_running_step_must_not_discard_frame_time`
(`tests/adapter.rs:241–258`) passed. The regression compares complete checkpoints
after every one of ten frames and asserts tick 10. The complete current headless
suite also passed. Closure is based on the inspected fix and executed tests;
the author's reported pre-fix regression failure is not substituted for this
reviewer's original independent reproduction.

## 3. Source conclusions

- **Frozen boundary:** explicit baseline diff for `src`, root manifest/lock,
  toolchain, C0.1–C0.3 contract documents and the C0.3 vector packer was empty.
  The app remains a separate package. No kernel mutation/API extension or second
  gameplay simulation was found.
- **Time/commands:** integer debt, eight-tick catch-up cap without debt discard,
  ordered adapter actions, deterministic command schedule, and candidate-based
  publication are consistent with the contract after SR-01's fix.
- **Fixture/view:** the packer reproduces the accepted twenty-actor mixed-goal
  S4 initialization, not the distinct contention fixture. Projection follows
  canonical field order, uses bounded reads/counts, and public ingress first
  requires kernel validation. No fixture-1/2 compatibility accessor misuse found.
- **Persistence:** sibling-temp replacement, synced contents, bounded reads
  including post-metadata growth, and candidate validation precede live-state
  replacement. Failed load does not replace the canonical simulation; successful
  load pauses and clears debt/recent history. Tests cover Windows locked-file
  preservation and invalid/wrong-fixture candidates.
- **Presentation/localization:** markers, canonical positions, selection,
  inspector data and map synchronization derive from the read-only view. Stored
  grid/passability is rendered without pathfinding or gameplay mutation. Native
  input marks paint dirty for selection/language/load/reset. Dictionaries have
  duplicate/empty/required-key/parity/fallback/UTF-8 tests. No additional concrete
  source defect was established in these areas.
- **Native deltas:** setup and camera reset both use Z=0. The embedded Noto Sans
  file matches its documented SHA-256
  `b85c38ecea8a7cfb39c24e395a4007474fa5a4fc864f6ee33309eb4948d232d5`;
  provenance and OFL text are bundled. F12 capture requires an explicit evidence
  directory environment variable and uses Bevy's primary-window framebuffer
  screenshot API plus read-only JSON diagnostics. That specific path does not
  capture the OS desktop or advance the simulation.

No outstanding concrete source finding remains from this review.

## 4. Commands actually executed by this reviewer

Cargo executable: `C:\Users\PC_4\.cargo\bin\cargo.exe`. Git executable:
`C:\Program Files\Git\cmd\git.exe`. Commands ran from the repository root.

| Command / check | Observed result |
| --- | --- |
| `cargo test --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --locked --offline c04_t04_ignored_running_step_must_not_discard_frame_time` | Exit 0; focused integration regression 1 passed, 5 filtered; unit tests filtered |
| `cargo test --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --locked --offline` | Exit 0; 17 unit tests and 6 integration tests passed; 0 failures; 0 doc tests |
| `python apps/sealed_playable/tools/pack_demo.py --check` | Exit 0; 3657 bytes; SHA-256 `f6a4be4d54a82e6400c31fd0a4c241c638399a1e1961b23eeddbbe6cba0dddad` |
| `git diff --exit-code 14d229bedc308d7e50225122caaec0ce5732a201 -- src Cargo.toml Cargo.lock rust-toolchain.toml 'docs/C0_1*' 'docs/C0_2*' 'docs/C0_3*' docs/tools/c03_vectors.py` | Exit 0; no output |
| `git branch --show-current`, `git rev-parse HEAD`, `git status --short` | Branch/HEAD and untracked application scope recorded above |

The complete headless suite includes real Bevy-plugin versus direct-kernel
100-tick parity under multiple frame partitions/catch-up, control/failure
publication checks, persistence continuation, localization validation, and
projection/fixture checks. It does not execute the default-feature desktop binary.

## 5. Remaining acceptance limitations

- This reviewer has not executed or approved native interactive QA, screenshot
  contents, camera/road visibility, font glyph rendering, mouse hit testing, or
  runtime performance observations. Source inspection cannot establish those.
- Default-desktop build/fmt/Clippy/release and frozen-root full verification are
  delegated to the parent's ongoing verification, not claimed as independently
  executed here. Their eventual logs still require reconciliation.
- F12 screenshot writing is asynchronous. Verify actual PNG completion and
  correlate images/JSON with observed actions; paused captures provide clearer
  tick correspondence. A JSON file alone is not proof of a successful screenshot.
- The app was untracked; delivery commit and remote SHA verification remain open.

**Source-only PASS does not close C04-T07, C04-T08, C04-T09 or the milestone.**
Final independent acceptance must follow inspection of actual native evidence and
the complete verification/delivery record. Only this review document was added
to repository content by this reviewer; no implementation source was edited.
