# C0.5 continuation

C0.6 continues on `feat/c06-readable-village` in the `sealed-heaven-c06` worktree.
This file remains the C0.5 record. See `C0_6_HANDOFF.md`.

Status: **C0.5 PASS** for the approved player-agency contract. Independent
re-review returned PASS after the same-frame duplicate-intent fix. Do not merge
to main, force-push, or start C0.6.

Base C0.4: `f35bfd3664d4baf2feaf78755b1a18d29cccf795`. Branch:
`feat/c05-player-agency`. First verified increment `ad0410cf469eea29df47baa84de4da4e7f45fd72`
was committed/pushed and remote matched. Current shared tree contains additional
core/client remediation plus reports; preserve all existing changes.

Latest verified implementation/evidence checkpoint (not acceptance) was pushed
as `a50fcad8fd5333dde3292211622cbf60a93727d5`; `origin/feat/c05-player-agency`
matched that SHA. The acceptance increment is the following commit on this
branch. It records the independent re-review and the non-adjacent intent dedupe.

## Independent re-review

A separate auditor inspected the post-remediation source, re-ran the three
forgery tests, the exhausted-cursor test, and the agency input tests, and read
`C0_5_VERIFICATION_LOG.txt` plus `docs/evidence/c05-native-final`. First verdict
was FAIL: repeated intents were collapsed only when consecutive. That host gap
was fixed and `c05_verify.ps1` was rerun (exit 0; root 100 passed, 1 ignored;
agency 9 passed in both profiles). The same auditor then returned PASS with no
remaining findings. The exhausted NPC cursor stays the approved contract:
restore of a cursor-only `u64::MAX` blob succeeds, later drives may succeed
while the frozen kernel accepts them, and the drive that returns `Error::Npc`
leaves the checkpoint unchanged. The three forged outcomes still restore as
`InvalidState` and do not replace the live world.

## Next

Do not merge this branch into main. Do not begin C0.6 until its contract is
written. The recommended next milestone is a session/content extension that
keeps this tick kernel, the 4096-tick horizon, and one-way occupancy. It is not
authorized by this acceptance. The ignored 4096-tick roundtrip, NPC overlap of
the player, and the absence of a 30-day/R2 sample remain limitations.

## Implemented and checked

- Core remediation replays exact frozen-NPC history for restore validation, with
  4096-tick bounded horizon and the three checksum-valid audit regressions.
- Client fixes address real native timing, same-frame ordered input, and player
  label transform. The client source review passed and is recorded in this handoff.
- Full `tools/c05_verify.ps1` on this tree completed with exit 0: root 100 passed
  and 1 ignored maximum-horizon probe, 5 benchmark tests and 2 doctests in both
  profiles; app 41 tests per profile. Fmt/check/strict Clippy, packers and
  debug/release builds pass.
- Latest guarded native foreground run:
  `C:\Users\PC_4\AppData\Local\Temp\opencode\c05-native-gameplay-09` (31
  PNG/JSON pairs), release SHA-256
  `4421E31DD362F9A638F6A918F1604E17F286402565CD13A9D566DE6C3CB2FD44`.
- Final release performance matrix/raw files:
  `C:\Users\PC_4\AppData\Local\Temp\opencode\c05-performance-final-03`;
  see `C0_5_PERFORMANCE_BASELINE.md`. Not an R2/30-day claim.

## Historical blocker, now closed

The first core review recorded CORE-01. That restore gap was remediated before
this acceptance. The paragraph below is the record of that open item; the
independent re-review section above is the closing verdict. The intermediate
review notes were removed after this handoff became the record.

Button labels were `FocusPolicy::Block` by default, so a click on the label
never pressed the parent button and a later step could advance with no intent.
Labels on action buttons now pass the click through. `drive_frame` queues every
intent before the frame's single step and collapses a duplicate intent in that
frame. Keyboard E/N and mouse Interact/Step share that pipeline. Out-of-range
still records `Rejected: Target is not within one cell`.

Final native run at `c05-native-gameplay-09` completed delivery at tick 23
(source 2, player 0, shrine 1), deterministic save/load suffix, both locales,
NPC progression, pause/step and reset. Queue captures and exact one-step
consumption gate each E/N interaction. The earlier button-click failure is now
traced to labels blocking parent clicks; label click-through and ordered
same-frame input were fixed and exercised by native and automated tests.

## Next bounded actions

1. This re-review is closed PASS. Do not merge to main and do not start C0.6.
2. Push only `feat/c05-player-agency` and confirm the remote SHA.

Owner-approved scope includes one-way occupancy: NPCs may later overlap the
player, but player entry into currently occupied cells is rejected. No dependency,
toolchain or root lockfile changes are authorized.
