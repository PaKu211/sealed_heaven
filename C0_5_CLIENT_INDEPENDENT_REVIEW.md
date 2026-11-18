# C0.5 independent client source review

Date: 18 November 2026. Reviewer authored no implementation changes.

**Current verdict: PASS — reviewed client source/remediation only.** All three
initial client findings are closed by independently inspected parent-authored
fixes and independently rerun debug/release unit tests. No additional client
defect was established in this delta review. **Not C05 final/native/performance
acceptance.** See the re-review evidence below.

Initial verdict, preserved: **NOT CLEAN** — two reproduced client defects and one
source-confirmed presentation defect. The original findings and repros below
describe the pre-fix source, not the remediated version.

Reviewed the working tree subsequently observed at
`ad0410cf469eea29df47baa84de4da4e7f45fd72`, against
`C0_5_IMPLEMENTATION_SPEC.md` and `C0_5_ACCEPTANCE_TESTS.md`.
Scope: `apps/sealed_playable/src/agency.rs`, `agency/persistence.rs`,
`agency_desktop.rs`, `agency_main.rs`, `tests/agency.rs`, C05 dictionaries,
`Catalog::agency`, and the app Cargo binary changes. The parallel core auditor
owns domain correctness. No kernel edit, full verification script, native input,
or additional agent was run by this reviewer.

## C05-CLIENT-01 — High: native elapsed time silently loses long frames

Locations: `apps/sealed_playable/src/agency_desktop.rs:460,466,504,819-822`.

`keyboard` uses `Res<Time>`, which Bevy 0.17.3 aliases to the virtual clock during
Update. Its default maximum delta is 250 ms. The app does not disable that clamp.
The host accumulator receives already-truncated time; its otherwise-correct
eight-step cap cannot recover it. This regresses C04's explicit `Time<Real>`
choice (`src/desktop.rs:644-650`) and the retained-elapsed-time contract.

An isolated headless probe ran the actual adapter with the native `keyboard`
function and its helpers extracted verbatim from source, `MinimalPlugins`, and
`TimeUpdateStrategy::ManualDuration`. After initialization/resume, two ordinary
1-second updates produced:

```text
ONE_SECOND real_ns=1000000000 virtual_ns=250000000 canonical_tick=0 debt_ns=250000000
TWO_SECONDS canonical_tick=1 debt_ns=0
```

Expected: ticks 2 and 4, respectively, with zero debt. No pause/load/reset action
occurred. Real stalls above 250 ms permanently slow the canonical timeline rather
than leaving catch-up debt. Existing adapter tests inject elapsed nanoseconds
directly, bypassing this native clock conversion.

`metrics` uses the same virtual clock, so a 1-second frame is also reported as
250 ms, despite the F12 report claiming a wall-frame interval. This compromises
the native frame-time characterization, not just gameplay pacing.

Required remediation: measure native host elapsed input and frame observations
with the real clock; retain explicit transport debt-reset semantics. Add a
regression exercising the actual native input clock path with >250 ms deltas and
assert both canonical catch-up and unclipped frame observations.

## C05-CLIENT-02 — Medium: batched keys resolve against stale presentation state

Locations: `apps/sealed_playable/src/agency_desktop.rs:484-495` (also button/keyboard
composition at `435-455`).

Native keyboard messages are consumed in order but each action reads the same
pre-frame `session.selected`, `session.paused`, and `session.locale`. Previously
queued actions are not considered. This affects ordinary multiple events between
frames, especially after a stall; it is not a canonical domain rejection bug.

The isolated probe delivered pressed, non-repeat messages `Tab`, `E`, `N` in
that order in one Update from the initial paused state:

```text
TAB_E_N selected=HerbPatch command=[Command { key: Key { target_tick: 1, actor_id: 21, producer_id: 1, sequence: 0 }, intent: Talk(1) }] event=[Event { id: 1, tick: 1, key: Key { target_tick: 1, actor_id: 21, producer_id: 1, sequence: 0 }, outcome: Rejected(OutOfRange) }]
TWO_TABS selected=HerbPatch
TWO_SPACES paused=false
```

Expected: the first sequence admits `Gather` for the newly selected herb patch;
two Tab presses select the shrine; two Space presses return to paused. Two L
presses similarly select Indonesian twice instead of toggling back. The same
stale-state hazard exists when a button action precedes keyboard input in the
chained systems. C04 already folds pending control actions for these controls.

Required remediation: interpret ordered native events against effective pending
selection/transport/language state, or resolve contextual input in the ordered
host action stream. Regress same-frame Tab/E/N, repeated Tab/Space/L, and mixed
button/keyboard actions. Preserve the domain as the only gameplay authority.

## C05-CLIENT-03 — Low: player ID label stays at the entrance

Locations: `apps/sealed_playable/src/agency_desktop.rs:247-256,747`.

The world-space `Text2d("21")` is spawned at the entrance with no marker component.
`paint` moves only `PlayerMarker`, not this text. After one successful East move,
the player sprite moves one cell but its identity label remains under `(0,4)`;
further movement leaves the label permanently detached. This is source-confirmed,
not a claim of a completed native visual run.

Required remediation: attach the label to the player presentation hierarchy or
update its transform from `world.player_position()`. Add a presentation regression
checking the sprite/label offset after movement and load/reset.

## Other reviewed boundaries and limitations

- `Session` has one private canonical `World`; ECS snapshots/markers do not write
  player inventory, player position, NPC tasks, stocks, or interaction outcomes.
- Queue stamping uses `world.tick()+1`/`next_sequence()` at admission; paused input
  alone does not drive the world. Successful load/reset clear queued intents,
  schedules, recent/admitted actions, capture evidence and debt before resuming.
- Persistence uses a distinct `checkpoint.c05`, bounded reads, core validation,
  and a synced sibling tempfile before replacement. No additional persistence
  defect was established in this client review.
- Inventory/range/rejection text reads canonical quantities, `in_range`, live
  target positions and returned outcomes. Both new dictionaries translate the
  new UI labels; overlay validation preserves old keys and rejects shadowing.
- The follow camera is presentation-only. Source inspection found explicit
  disjoint filters for `paint`'s mutable player/NPC/ring transform queries; no
  query-conflict defect was established. This does not replace native startup.
- C04's existing binary remains separate; the now-present `default-run` preserves
  its unqualified `cargo run` behavior. C04 dictionaries/desktop were not edited
  by the C05 client additions inspected here.
- Owner-approved one-way NPC occupancy is **not** reported as a bug.
- Full frozen/debug/release/build verification and actual native gameplay remain
  the parent's separate evidence work. No T09/T10/final T11 PASS is asserted.

## Probe evidence

External probe retained at
`C:\Users\PC_4\AppData\Local\Temp\opencode\c05-client-audit`.
Its `src/keyboard.rs` contains verbatim copies of `targets`, `context_intent`,
`toggle`, `language`, and `keyboard` from the reviewed desktop source; its executable
uses the real `sealed_playable::agency` adapter and `sealed_sim` path dependencies.
No implementation file was modified to expose private functions.

Executed successfully (exit 0):

```powershell
C:/Users/PC_4/.cargo/bin/cargo.exe run --manifest-path C:/Users/PC_4/AppData/Local/Temp/opencode/c05-client-audit/Cargo.toml --offline --target-dir 'D:/projek/sealed/Sealed_Heaven_C0_1_Source (1)/sealed-heaven-c04/apps/sealed_playable/target'
```

This reviewer changed only this repository report. The initial findings required
parent-owned fixes, regressions, and independent re-review before source closure.

## Independent remediation re-review — 18 November 2026

Re-reviewed the complete `agency_desktop.rs` and its delta from
`ad0410cf469eea29df47baa84de4da4e7f45fd72`. At review time HEAD still identified that
commit; the desktop fixes were **uncommitted working-tree changes**. Also re-read
the unchanged host adapter. Concurrent domain restore/history changes belong to
the separate core review and are not certified here.

| Finding | Independently verified closure |
| --- | --- |
| CLIENT-01 | `keyboard` now receives `Time<Real>` at lines 497-505; held-repeat elapsed time uses that same clock. `metrics` also receives `Time<Real>` at lines 860-863. Actual native-function unit regression verifies two 1-second updates advance to ticks 2 and 4 and record a 1000 ms frame interval. **Closed.** |
| CLIENT-02 | `PendingControls` folds preceding actions and each subsequently generated action. Button and keyboard code derive selection, pause toggles and language from that effective state, not the pre-frame session. Regression verifies Tab/E/N admits Gather; repeated Tab/Space/L preserve ordered behavior; and a preceding queued selection affects subsequent keyboard input. The last case seeds an action rather than exercising an actual mouse/button query. Button folding itself was inspected directly. No gameplay field is copied into this control helper. **Closed.** |
| CLIENT-03 | Production `spawn_player` creates `PlayerIdentity` as a child of `PlayerMarker`, replacing the abandoned entrance text. The regression runs production `spawn_player`/`paint` with `TransformPlugin`, and checks actual parent/local/global transforms after an East move and reset. Load was not separately asserted by this unit test; source confirms every paint updates the same parent from the restored canonical position, so children follow by the same mechanism. **Closed.** |

Independently executed from the repository root, both exit 0:

```powershell
C:/Users/PC_4/.cargo/bin/cargo.exe test --manifest-path apps/sealed_playable/Cargo.toml --locked --offline --bin sealed_agency
C:/Users/PC_4/.cargo/bin/cargo.exe test --manifest-path apps/sealed_playable/Cargo.toml --release --locked --offline --bin sealed_agency
```

Both runs: **4 passed; 0 failed; 0 ignored**:

- `c05_client01_native_real_clock_retains_stalls_and_frame_intervals`
- `c05_client02_batched_controls_use_effective_ordered_state`
- `c05_client03_player_label_follows_hierarchy_after_move_and_reset`
- `every_reason_and_target_has_a_translated_label`

These are headless tests compiled in the native binary's test harness, invoking
the actual private native functions. They are **not native gameplay/input proof**.
The production paint system also initialized/executed without a query conflict
in the hierarchy regression's minimal scene; a complete native scene/startup
was not exercised by this reviewer.

SHA-256 of reviewed working-tree file bytes (not Git-normalized blobs):

| File, relative to `apps/sealed_playable/` | SHA-256 |
| --- | --- |
| `src/agency_desktop.rs` | `2321c2593b5c83bf95ec2cbf447f9920e5239aaa31472c81c362af7c4ba677c7` |
| `src/agency.rs` | `098aa21b4bc909957050254814b621f9073a8d07acfe4f0b73757850c867d73a` |
| `src/agency/persistence.rs` | `c14cbab256660aed26257bae554494a29ebaab4bf4f10b9ccb2a8fc9b56bd056` |
| `src/agency_main.rs` | `eb8cb20fef26ff9546774fd813d70ce26a6c79f22bd0db86c6b5573d31057f41` |
| `tests/agency.rs` | `223e360cecacec5aa8301379cbcfd1393c903473c2d41df23a070fbfc4bd84b1` |
| `src/locale.rs` | `a1e74b26e2242a42d74c1b932690fdade86885e84e80dacd3ecbe0f297507dd6` |
| `locales/c05_en.json` | `4bd8d5d09656994b3ed2938bd63a2f8c5b18ed1516065ef6bb9abb52c97d45e0` |
| `locales/c05_id.json` | `f7e9f4600e9b79237e77ad807ddb4c3cb1dc77de1093b9d56b014d16eb10fb63` |
| `Cargo.toml` | `818484e75991e0bb4320d5929db10259dc7f651c14f14051fd9840a451b2aaab` |

The parent reports the initial guarded native driver was denied foreground
access and sent no input. This reviewer did not independently run that driver,
send native input, or assess runtime access. That attempt does not close T09.
Performance, full compatibility verification, separate core findings and final
independent C05 acceptance remain outside this **source-only client PASS**.
