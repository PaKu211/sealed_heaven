# C0.5 gameplay verification — native run passed; final acceptance pending

Base `f35bfd3664d4baf2feaf78755b1a18d29cccf795`; branch `feat/c05-player-agency`.
Native gameplay driver completed successfully on 2026-11-18. This closes the
interactive gameplay run only; it does not establish final C0.5 acceptance.

Automated loop: `tools/c05_verify.ps1` passed after core/client remediation:
root 100 passed + 1 explicitly ignored stress test, 5 benchmark tests, 2 doctests;
app 41 tests per profile (debug/release), with formatting, checks, strict Clippy,
packers and debug/release builds successful. Integration tests submit
movement/request/gather/deliver through the same host plugin, compare headless
checkpoints/events at each tick across frame partitions, and save/load at each
action.

Native procedure: `tools/c05_native.ps1 -EvidenceDirectory <fresh-absolute-folder>
-Profile release`. Uses guarded actual foreground mouse/key events and game-only
F12 framebuffer captures plus JSON. Never writes canonical state to drive a test.
Checks invalid range feedback, entry/exploration, Yun's request, herb acquisition,
atomic delivery, carried/completed save/load and suffix replay, Tab/mouse target
selection, localization/hash invariance, pause/step/reset, camera centering and
continuing autonomous NPC events. A failure preserves partial evidence, not PASS.

Successful final native evidence: `C:\Users\PC_4\AppData\Local\Temp\opencode\c05-native-gameplay-09`
(31 game F12 PNG/JSON pairs; release binary SHA256
`4421e31dd362f9a638f6a918f1604e17f286402565cd13a9d566de6c3cb2fd44`). Guarded
foreground keyboard/mouse input exercised movement/exploration, mouse target
selection, invalid out-of-range Talk feedback with a visible canonical rejection,
nearby Talk, Gather, one-time delivery, duplicate rejection, carried and completed
save/load plus deterministic suffix, English/Indonesian, pause/step/run, camera
follow, autonomous NPC progression, and reset. Tick 23 completed delivery; tick 48
paused after autonomous progression. Conservation remained 3 throughout. The
script log ends `PASS`; this is truthful native run evidence, not separate review.
Supplemental same-binary evidence in `docs/evidence/c05-native-final/` also
exercised mouse Interact/Step successfully for Yun's request and shrine delivery.

During harness debugging, native Interact/Step button clicks sometimes updated the
UI/clock without queueing the expected intent. The final driver uses real focused
keyboard shortcuts (E/N/F5/F9/etc.) for those actions, while retaining real mouse
target selection and the same gameplay oracles. Two earlier attempts failed
without being counted as gameplay passes; their evidence remains in separate
folders `c05-native-gameplay-03` and `-05`. A subsequent foreground attempt
after rebuilding the final release binary failed during delivery; that capture
(`c05-native-gameplay-08`) is not counted. The final driver snapshots the visible
queued intent and requires exactly one subsequent step to consume it, retrying
only when queue state proves the step was not committed.

Controls: WASD/arrows movement; E interaction on selected target; Tab/mouse
selection; Space resume/pause; N single step while paused; F5/F9 save/load; L
language; R reset; C follow/center. Paused intents wait in the visible queue;
N or resume commits them at canonical boundaries.
