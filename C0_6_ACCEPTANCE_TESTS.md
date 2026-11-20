# C0.6 acceptance checklist

| Gate | Proof |
|---|---|
| C0.5 frozen tests | `tools/c05_verify.ps1` exit 0 on this branch |
| Checkpoints unchanged by reads | Clock and `View` reads leave `World::hash` unchanged |
| Derived time | `floor(tick / 2) % 1440`, same tick always same minute and day |
| Save/load | Existing C0.5 agency save/load tests still pass |
| Visual state matches authority | NPC color and goal text come from `View` tasks, not the calendar |
| No duplicate commands | Existing same-frame intent tests still pass |
| Day/night is presentation | `is_night` is a pure function of the minute |
| Language | Existing locale switch tests still pass; new keys exist in English and Indonesian |
| Pause, step, reset | Existing agency tests still pass |
| Schedule is not a fake action | At tick 1 the calendar is Sleep while kernel tasks already have goals |
| Native play | Separate evidence. Headless tests are not a visual pass |
