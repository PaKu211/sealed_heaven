# Partial native evidence — NOT C04-T07 PASS

Native Windows/Vulkan debug executable; game-only framebuffer PNGs and read-only
QA JSON. `native-001` is paused startup/tick 0. `native-002` is paused tick 1 with
actual Eat/Executing state after a native single-step key.

The keyboard attempt later timed out and did not finish selection, save/load,
language, camera, or run/pause checks. `smoke-log.txt` contains only completed
observations, not a PASS. No mouse-toolbar success or running performance sample
is asserted. See `C0_4_NATIVE_INPUT_BLOCKER.md`.

These are not OS desktop images and contain no other applications. Save paths
identify the isolated temporary QA profile, not a shipped save requirement.
