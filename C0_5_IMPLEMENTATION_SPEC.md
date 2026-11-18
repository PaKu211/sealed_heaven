# C0.5 — Player Agency & First Gameplay Loop

Authorized scope: owner's C0.5 request, 18 November 2026. Base:
`f35bfd3664d4baf2feaf78755b1a18d29cccf795`. Branch `feat/c05-player-agency`.

## Authority and compatibility

Add `sealed_sim::player5`, a pure Rust **extension of the existing core**, not a
second Bevy simulation. Its `World` owns one unchanged C0.3 NPC `Simulation` and
only the new player/settlement fields. The embedded simulation remains the sole
owner of NPC tasks, movement and stocks. Existing implementations, commands,
checkpoint formats, root dependencies/lock/toolchain and vectors are unchanged;
the only old-root-source addition is the public module declaration. The C0.4
binary/plugin remain runnable. C0.5 uses a separate binary in the same app package.

The new aggregate checkpoint has its own magic/revision/content identity,
checksum, exact embedded C0.3 bytes, player position/identity, conserved herb
ownership, request/completion flags, command identities, sequence high-water,
event cursor/digest. Restore is bounded, rejects malformed/inconsistent/unknown
versions and publishes only a complete validated candidate. No implicit import,
migration, rewriting or overwriting of C0.4 saves; use `checkpoint.c05`.
The frozen C03 exhausted event cursor (`next_event_id == u64::MAX`) remains a
valid atomic-failure state: restore may preserve that cursor only when all other
NPC history bytes and its digest match exact replay; the next failing NPC step
must leave the whole aggregate unchanged.

## Authored slice

Qinghe settlement, 12×9 cells, one novice player (ID 21, distinct from NPCs 1–20),
entry `(0,4)`, wall at x=6/y=1..6 with gate y=4, finite spirit-herb patch `(9,2)`,
delivery shrine/store `(3,5)`. Healer Yun is autonomous NPC 1, not a UI-only actor.
Talk nearby to learn the request; gather one herb; deliver to the designated
shrine; inspect source/player/store quantities; save/load the completed result.
No payment, combat, general quest framework, new needs/AI or renewable resources.

NPC positions are **read from the embedded canonical simulation**: site 1 maps
to `(3,3)`, site 2 to `(9,5)`, site 9 to `(10,7)`; site-1 grid coordinates map to
`(3+x,3+y)`; actors in transit have no interaction cell. Marker offsets are art
only. Enabled NPC occupancy prevents player entry. Frozen NPC rules can share
cells and can subsequently arrive on a player's cell; allow the player to leave,
but do not add bidirectional collision semantics to the accepted NPC kernel.

Owner decision, 18 November 2026 (during initial assessment): explicitly approved
this one-way occupancy boundary (“Izinkan batas ini”) after being offered the
alternative of a new integrated mutually exclusive spatial ruleset. This is a
bounded C0.5 limitation, not a claim that NPC collisions are implemented.

## New command contract

Immutable commands carry `(target_tick, actor_id, producer_id, sequence)`, player
21 / producer 1, and one `Move(N/E/S/W)`, `Talk(npc)`, `Gather`, or `Deliver` intent.
Accept only next-tick input; order keys deterministically. One new unique action
per tick. Exact identity+payload retries have no repeated effects/events; identity
conflicts fail admission atomically. Persist registry/high-water to retain retry
safety after load. Registry/batch limits are explicit and tested.

Drive stages the whole aggregate, advances unchanged NPC simulation once, then
revalidates the player's action against that committed candidate. Movement is one
cardinal cell; enforce map bounds, terrain, object and enabled-NPC occupancy.
Interactions require Manhattan distance ≤1 to the live target. Only NPC 1 grants
the request. Gather requires request/range/stock; deliver requires request/range,
player stock and an incomplete request. Transfer debits/credits together with
checked arithmetic. Total herbs source+player+store remains exactly 3; no minting.
Gameplay rejection is a committed reasoned event; malformed input or tick/cursor
overflow aborts the entire aggregate including NPC work.

## Host and UI

Restore-audit remediation: validate historical range/occupancy by replaying the
actual frozen NPC history and each player outcome, not by trusting checksummed
history assertions. Explicit slice horizon4096 ticks (also4096 commands) bounds
this work: reject larger saves before replay; refuse drives at the horizon
atomically. At2 Hz this is about34m08s, not a long-run/M0 promise. C05 rev1 remains
an unaccepted development format; no accepted checkpoint schema changes.

Bevy owns input, camera, selection and read-only snapshots only. Bounded input
buffer, canonical stamping at the eligible boundary, no ECS writes to domain
fields. Pause runs no steps; N explicitly steps while paused; resume/load/reset
reset elapsed debt. Keep C0.4's nominal 2 Hz for controlled comparison (not the
provisional future M1 cadence experiment). Player-follow camera is presentation.
Keyboard movement, clickable/Tab target selection, contextual action, inventory,
canonical feedback, pause/time/save/load/reset, English/Indonesian labels.
Queued input is visible and cleared on load/reset; it is host input, not admitted
future commands. Checkpoints contain all **admitted** future-affecting state.

## Evidence and exit

See `C0_5_ACCEPTANCE_TESTS.md`. Benchmark actual supported NPC fixtures only;
separate tick compute time/unpaced throughput from configured host pacing and
rendering. Native interaction and independent acceptance are separate from build
and headless tests. Record any unsupported 100/1000 actor configurations honestly.
No main merge, force-push or C0.6. Audit fixes need regressions and reruns.
