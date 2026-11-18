# C0.4 final independent audit

18 November 2026. Independent acceptance review of the existing C0.4 First
Playable implementation. The implementation was not rewritten for this review.

## Verdict

**PASS.** Every normative row in `docs/C0_4_ACCEPTANCE_TESTS.md` is satisfied
by evidence inspected in this review. This does not accept a 30-day soak or
the deferred R2 performance gate, and it does not authorize C0.5.

## Reviewed identity

- Repository: `https://github.com/PaKu211/sealed_heaven.git`
- Branch: `feat/c04-first-playable`
- Implementation checkpoint reviewed: `0cc47cd654cf2f88b4f0c09253f1fe6d4a551e62`
- Frozen C0.3 baseline: `14d229bedc308d7e50225122caaec0ce5732a201` (ancestor; frozen diff empty)
- Debug binary exercised: `apps/sealed_playable/target/debug/sealed_playable.exe`
- Debug SHA-256: `302a3b3a3bf09ab67bde09865cf192596045a17a0d0bcbf68b9bcea716c80aa7`
- Renderer reported by the application: NVIDIA GeForce RTX 4060 / Vulkan
- Machine: 12th Gen Intel Core i7-12700F, 16,980,426,752 bytes physical memory, Windows

The evidence and this audit were committed as `ea6d6430b93b94a49ff13ea018b169987ae4e16f`
and that SHA matched `origin/feat/c04-first-playable` after push. No merge to
`main` and no force-push.

## What was re-executed

`apps/sealed_playable/tools/verify.ps1` was run again on this checkpoint.
`C0_4_VERIFICATION_LOG.txt` records exit 0 for every step:

- Root format, locked offline check, strict Clippy, and tests: 82 unit tests
  and 2 doctests in debug and in release.
- C0.3 vector packer and the C0.4 demo packer.
- Application format, locked offline check, strict Clippy, debug tests, release
  tests, and debug build. Each profile ran 17 library tests, 3 desktop tests,
  and 6 integration tests.

Prior contract review and source review were read, not replaced. SR-01 stays
closed. No canonical vector, frozen manifest, lock, toolchain, or kernel source
was edited.

## Native evidence inspected

Fresh directory: `docs/evidence/c04-native-acceptance/`.

Forty-nine Bevy framebuffer PNG/JSON pairs, `smoke-log.txt`, and
`performance.json`. Each PNG ends with an IEND chunk and is 1424×821. None is
a minimized or single-color capture. Each JSON reports 20 actors, an empty
diagnostic, and the RTX 4060 / Vulkan adapter. The action log and driver
scripts in that directory describe real foreground `mouse_event` / `keybd_event`
input sent only after the game window owned the foreground. F12 captures were
posted to that window. This is not the earlier keyboard-only partial run.
`docs/evidence/c04-native-partial/` remains historical and is not this verdict.

Frames checked directly: tick-1 Eat/Executing (`native-002`), Indonesian tick 4
with arrivals (`native-028`), pan (`native-030`), and English reset to paused
tick 0 (`native-049`).

## C04-T07 results

| Check | Evidence |
| --- | --- |
| Paused tick 0, 20 NPCs, map visible | `native-001`. Hash `f6a4be4d54a82e6400c31fd0a4c241c638399a1e1961b23eeddbbe6cba0dddad`, the accepted S4 digest prefix. |
| Mouse select NPCs 1–5 at tick 1 | `native-003` through `native-007`. Goals Eat, Rest, Work, Haul, Return home. Stages Executing, Travelling, Executing, Travelling, Travelling. Needs, canonical position, and inventory present. Hash unchanged from tick 1. |
| Tab selection | `native-008` selects NPC 6. Tick and hash unchanged. |
| Exactly-one-tick steps | Tick 1 → 2 (`native-009`) → 3 (`native-011`) → 4 (`native-014`), paused after each step. |
| Save at tick 2, load, continue | Save status `status.saved` (`native-010`), hash `6f1131bcb9541f94bea374a7a4aba875e3c1149242a3109a333a9ad14a445599`. Tick 3 hash `133c7a921bbd119eaf0df18df932af07fe9251be447e1cccf1f118fb9981ed97`. Load returns paused tick 2 and the saved hash (`native-012`). The next step reproduces the same tick-3 hash (`native-013`). |
| Arrivals, ownership, ordered events | Tick 4 hash `8faa211de3f29bc79031093ed4bec3a18b241baa088ac38f4bdacae1422c088f`. NPCs 1 and 3 are at site 1; NPCs 2, 4, and 5 are at site 2. Four definition-7 units are owned by site 2. Durable stocks 101 and 102 remain quantity 4 at site 1. Event IDs 77–144 are strictly ascending. Inspector scroll (`native-026`) does not change the hash. |
| Language | English and Indonesian clicks (`native-027`, `native-028`). Indonesian inspector reads Pemeriksa dunia / Tujuan. Hash and digest stay on the tick-4 value. |
| Camera | Pan moves NPC 1 from viewport x 178.0 to -32.1 with spacing unchanged (`native-029` to `native-030`). Q changes NPC 1–2 spacing from 472.0 to 182.7 (`native-031`). E moves it back to 460.6 (`native-033`). C restores NPC 1 to 178.0, 338.5 and spacing 472.0 (`native-034`). Hash unchanged throughout. |
| Toolbar, pause, resume, reset | Single step, Save, Load, Reset, English, Indonesian, and Resume/Pause were mouse clicks on the control rectangles reported by the framebuffer JSON. A focused 10-second run advanced tick 572 to 593, about 1.44 ticks per second against the 2 Hz nominal clock. Pause then held tick 594 and its hash for one second (`native-046`, `native-047`). Reset returns paused tick 0, status `status.reset`, and the original checkpoint hash (`native-048`). English is restored with that same hash (`native-049`). |

One NPC-3 click in the arrival series landed on NPC 2 (`native-017`) and was
repeated; the successful click selected NPC 3 without changing the checkpoint.
The first resume/pause sample did not advance during an interval when the
window was not producing frames (`native-035` still tick 4). When frames
resumed, stored running time caught up across `native-036` to `native-040`
(tick 367, then 495, 500, 523) and a later pause held at tick 572. That matches
the specified catch-up drain. It is not treated as a failed pause control,
because the later focused pause stopped the simulation on the next tick and
the hash stayed fixed while paused.

## C04-T08 performance observation

`docs/evidence/c04-native-acceptance/performance.json`, debug build, 20 actors,
window kept in the foreground:

- Sample length 14.558 seconds, 21 ticks advanced, 1.44 observed ticks/second, nominal tick rate 2 Hz.
- Process CPU 16.078 seconds, about 110% of one core, including rendering.
- Working set 521,945,088 bytes before the sample and 509,370,368 after. Peak working set 534,978,560 bytes.

This is one short debug observation after an earlier unfocused catch-up. It is
not an FPS benchmark, a modest-hardware guarantee, or the 30-day / R2 gate.

## Findings

No product defect required a source change. No regression test was added.
Frozen files were not modified.

## Limitations

- The interactive procedure is the foreground driver in the evidence directory, checked against the captured framebuffer JSON and the images named above. Headless tests are not substituted for that procedure.
- The running sample is debug, short, and includes input and capture overhead.
- `docs/evidence/c04-native-partial/` is not acceptance evidence.
