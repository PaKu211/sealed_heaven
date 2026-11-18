# C0.5 continuation

Status: input-pipeline remediation, automated verification, and a fresh native
keyboard/mouse run are recorded below. **Milestone remains NOT ACCEPTED** until
the separate independent audit returns PASS. Do not merge to main, force-push,
or start C0.6.

Base C0.4: `f35bfd3664d4baf2feaf78755b1a18d29cccf795`. Branch:
`feat/c05-player-agency`. First verified increment `ad0410cf469eea29df47baa84de4da4e7f45fd72`
was committed/pushed and remote matched. Current shared tree contains additional
core/client remediation plus reports; preserve all existing changes.

Latest verified implementation/evidence checkpoint (not acceptance) was pushed
as `a50fcad8fd5333dde3292211622cbf60a93727d5`; `origin/feat/c05-player-agency`
matched that SHA. This checkpoint is still awaiting independent core/client-delta
review and final acceptance.

## Implemented and checked

- Core remediation replays exact frozen-NPC history for restore validation, with
  4096-tick bounded horizon and the three checksum-valid audit regressions.
- Client fixes address real native timing, same-frame ordered input, and player
  label transform; independent client source review recorded PASS in
  `C0_5_CLIENT_INDEPENDENT_REVIEW.md`.
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

## Outstanding acceptance blocker

Initial independent core source audit found CORE-01, with three reproducible
history-forgery variants; see `C0_5_CORE_INDEPENDENT_REVIEW.md`. Domain-author
remediation is present and root regression tests pass in debug/release, but the
review must be rerun by a **different independent core auditor** against the
post-remediation tree, including the public-API forgery probe, bounded replay,
and valid occupancy/request controls. This continuation session cannot spawn an
independent subagent, so it has not self-approved the fix. An independent final
acceptance review must also explicitly cover remaining criteria. Do not call
C0.5 complete or push/commit a final acceptance claim until that is done.

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

1. Obtain a separate independent core re-review of the exact uncommitted tree;
   retain evidence and findings in the report.
2. If findings remain, remediate narrowly and rerun full verifier, native gate
   if affected, performance if core timing changes, and independent re-review.
3. Inspect final git diff, stage only authorized C0.5 files, commit a meaningful
   verified increment, push normally to this branch, then compare remote SHA.
4. Update reports with exact commit and honest acceptance state. No main merge or
   C0.6 authorization.

Owner-approved scope includes one-way occupancy: NPCs may later overlap the
player, but player entry into currently occupied cells is rejected. No dependency,
toolchain or root lockfile changes are authorized.
