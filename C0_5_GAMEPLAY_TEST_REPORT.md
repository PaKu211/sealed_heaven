# C0.5 gameplay verification — pending

Base `f35bfd3664d4baf2feaf78755b1a18d29cccf795`; branch `feat/c05-player-agency`.
No native gameplay PASS has been claimed.

Automated loop: integration tests submit movement/request/gather/deliver through
the same host plugin, compare headless checkpoints/events at each tick across
frame partitions, and save/load at each action. Execution results pending.

Native procedure: `tools/c05_native.ps1 -EvidenceDirectory <fresh-absolute-folder>
-Profile release`. Uses guarded actual foreground mouse/key events and game-only
F12 framebuffer captures plus JSON. Never writes canonical state to drive a test.
Checks invalid range feedback, entry/exploration, Yun's request, herb acquisition,
atomic delivery, carried/completed save/load and suffix replay, Tab/mouse target
selection, localization/hash invariance, pause/step/reset, camera centering and
continuing autonomous NPC events. A failure preserves partial evidence, not PASS.

Controls: WASD/arrows movement; E interaction on selected target; Tab/mouse
selection; Space resume/pause; N single step while paused; F5/F9 save/load; L
language; R reset; C follow/center. Paused intents wait in the visible queue;
N or resume commits them at canonical boundaries.
