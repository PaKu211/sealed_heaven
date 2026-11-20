# C0.8 First Conflict — implementation report

**Date:** 19 November 2026
**Branch:** `feat/c08-first-conflict`
**Base:** `ba8c2707c787c8fd9a88ad83e66963df24569b02` (C0.7 audit tip; owner baseline `56ccb51` is its parent)
**Original implementation commit:** `09534cb`. The revision-1 compatibility correction is tracked by the subsequent feature-branch commit.
**Verdict:** automated acceptance evidence for `09534cb` is green. The later revision-1 compatibility correction has not been independently executed; it requires a new test run before merge.

## G0

Combat cannot live in `SH:C05:STATE` revision 1 or in the ASCII renderer. Revision 1 stays the settlement payload and stays byte-compatible. The hostile actor is a C0.5-layer actor (id 22), not a C0.3 kernel NPC. Frozen `src` outside `player5` is untouched. The amendment is `C0_8_CHECKPOINT_AMENDMENT.md`.

A revision-1 village file without `shrine-supply.c03` still fails session load. Nothing migrates that file.

## Revision-1 compatibility correction after `09534cb`

Independent static review C08-R01 found that the revision-1 settlement world could record C0.8-only intent tags 5–7 while retaining a revision-1 checkpoint header. The correction rejects those intents **before admission**, and its decoder rejects a re-signed revision-1 payload containing them. Three regression tests cover unchanged tick/checkpoint/highwater, a forged-but-re-signed revision-1 tag, and exact restoration of a checked-in C0.7 revision-1 save. The new tests are **pending execution** in an environment with the pinned Rust toolchain; the acceptance log below predates this correction.

## What plays

`sealed_agency` opens `Session::conflict()`. The player starts at `(0,4)`. One foe starts at `(8,4)` on the herb route. `f` attacks an adjacent foe for 2. `b` guards for one tick and reduces the foe's strike from 2 to 1. `wasd` moves, including retreat. `n` waits one canonical tick on a conflict world and stays an empty step on a settlement world. `t`, `l`, inspect, save, and load do not submit a tick.

The foe attacks when adjacent, otherwise steps closer while Manhattan distance is at most 4, otherwise waits. Step order is north, east, south, west. A dead actor emits death once and does not act again. The hauler still runs on the same successful drive. Guard is not stored; it is false at every checkpoint.

## Acceptance

| ID | Evidence |
| --- | --- |
| C08-T01 | `the_same_trace_has_one_hash_and_survives_reload` |
| C08-T02 | `deliberation_does_not_tick_and_attack_guard_and_wait_do` |
| C08-T03 | Out-of-range attack on a revision-2 world is `OutOfRange` and consumes the admitted tick. Settlement `Attack`, `Guard`, and `Wait` are rejected at admission as `UnsupportedIntent` without a tick. **At the C0.8 commit**, revision 3 was `UnsupportedRevision`; C0.9 introduced real revision 3, moving the unsupported-revision assertion to revision 4. A revision-1 blob stamped as revision 2 or 3 is `ContentMismatch`. |
| C08-T04 | Foe closes during the scripted east/attack/guard/west trace in `docs/evidence/c08-ascii-play.txt`. |
| C08-T05 | `attack_and_guard_use_the_published_integers` |
| C08-T06 | `a_dead_player_cannot_act_again_and_dies_once`; herb sum stays 3. Supply completion stays 5/5, carried 7, shrine 5, total 12. |
| C08-T07 | Kernel restore and session save/load keep the same checkpoint and tick. |
| C08-T08 | `scripted_and_direct_commands_match_and_supply_still_completes` plus the existing C0.7 suite. |
| C08-T09 | Restore rebuilds combat events from the command log; a second death is not emitted. |
| C08-T10 | Scripted ASCII commands and `Action::Intent` plus `Step` produce one checkpoint. |
| C08-T11 | `tools/c05_verify.ps1` exit 0. Log copied to `C0_8_VERIFICATION_LOG.txt`. `C0_5_VERIFICATION_LOG.txt` restored afterward. |
| C08-T12 | Scripted terminal session shows approach, attack, guard, retreat, supply progress, and Indonesian `Musuh` / `Nyawa` / `Pasokan kuil`. It is not a five-minute human play. |

## Commands

```
cargo test --lib player5
cargo test --manifest-path apps/sealed_playable/Cargo.toml --test c08_conflict --test c07_supply
cargo clippy --all-targets -- -D warnings
cargo clippy --manifest-path apps/sealed_playable/Cargo.toml --all-targets -- -D warnings
powershell -File tools/c05_verify.ps1
```

`c05_verify.ps1` finished exit 0 on 19 November 2026. Kernel tests: 104 passed, 1 ignored horizon probe. Playable debug and release suites, including `c08_conflict`, passed. Clippy `-D warnings` passed for `sealed_sim` and `sealed_playable`.

## Limits

- One foe, fixed integers, no RNG, no second playable character, no dynasty.
- `tools/c05_native.ps1` was not re-run. It still looks for a Bevy window, and the default `sealed_agency` is ASCII. The window client compiles and its unit tests pass; it has no `f`/`b` keys.
- A corrupted revision-1 header whose revision byte is 2 now fails as `ContentMismatch`, because revision 2 is a real payload. Revision 3 is still unsupported.
- This report is from the implementing session.
