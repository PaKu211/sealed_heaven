# C2.7 — Player Reporting & Social Consequences

New revision 17; default ASCII starts `checkpoint.c27`. `--c25` still starts revision 16 in `checkpoint.c25`, and `--c24` remains revision 15. No C0.1 kernel file changed.

`reportbody <npc> <body>` is an actual timed, face-to-face action. The player must already have personally discovered that corpse; invented body IDs, dead or distant recipients, duplicates, hostile recipients, and unencodable journal IDs cannot become reports. A recipient hears after two turns only while the sender and recipient remain alive. The report identifies a corpse and its original eyewitness, *not* the killer. The existing bounded two-hop social conversations carry the news further. A watchman with no more urgent duty travels to the body and upgrades the account to direct observation only on sight. An exhausted watchman may instead rest; higher-priority contracts and danger may preempt the investigation. The original report's provenance and receipt time survive the upgrade in revision-17 state and checkpoint digest.

## Gameplay and adversarial evidence

- Clean seed-zero public actions: fourteen east moves; wait; `attack 32`; north/east/east/north/north/east/east/south; `attack 32` twice; south; eight west moves; three waits; `reportbody 2 32`. No NPC/cargo/position is injected on this path.
- At tick 39, the report is pending. Tick 41, the watch has a one-hop report witnessed by the player, while a no-report branch does not. Neither branch gives the watch knowledge of the distant fatal strike. At tick 42 the watch sees the actual corpse; the original report remains attached as `prior_report`. Save/load before and after arrival reproduces the exact world.
- The same sequence succeeds through ASCII `Session::input`, including `save`/`load`, and through the actual headless binary with `SEALED_ASCII_COMMANDS`; binary output captured in `/tmp/opencode/c27-ascii-play.txt`.
- Isolated adversarial tests show false/unseen reports are rejected; an unencodable ID or revision-16 report is rejected *before* mutation; a dead sender cannot deliver pending testimony. Comparing a report to a withheld report proves a rested, uncommitted watch chooses `InvestigateBody` only with evidence, while fatigue can preempt it. These policy fixtures are distinct from the public-command proof.

Offline Linux gates: simulation 280 passed, 1 ignored; playable 85 passed; Rustfmt both workspaces and both all-target Clippy checks with `-D warnings` clean. C2.5 and earlier checkpoint rules remain version-isolated. No human or Windows playtest.

Limitations: a body report never accuses a murderer, even if the player has separate eyewitness crime knowledge; reporting a witnessed *attack* as evidence is future work. The watch may independently discover a corpse or prioritize an existing contract before acting on the player's report. No physical supplies are created or destroyed by reporting.
