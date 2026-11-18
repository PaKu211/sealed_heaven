# C0.6 implementation report

Feature commit `48085b62202ce9906a710f3622a1fc2965bc5dcd` on `feat/c06-readable-village`, from `main` `496045841cedaf0f485ebef4c9e123b3354c6760`. The branch tip is the commit that records this line.

The village client now draws the player, a larger healer, other NPCs colored by their checkpoint task goal, the herb patch, the shrine, three canonical sites, and the y=4 lane. The clock is `floor(tick / 2) mod 1440`. The panel shows that time, the day, Night or Daylight, and a schedule line that says it does not move anyone. The inspector still prints needs and task id, and adds the goal from `View` tasks.

No simulation file changed. `tools/c05_verify.ps1` exited 0 after the final color fix (root 100 passed, 1 ignored; playable tests include the 3 new clock tests). Native capture `docs/evidence/c06-native` completed the existing foreground loop: request, gather, deliver, save/load, Indonesian, pause, and reset. The log ends in PASS.

Captured frames stay at night because that loop ends at tick 47 (00:23). Day starts at tick 720. The boundary is covered by `clock::tests`, not by a daylight screenshot.
