# C0.2 Acceptance Tests

Version 0.1-review-r1.1 · 17 November 2026  
Status: G1 conditional-pass polish. Implementation is not authorized.

Run these tests under the [C0.2 implementation specification](C0_2_IMPLEMENTATION_SPEC.md). C02-T01 and C02-T02 are the G0 exit groups. C02-T03 is the build audit. C02-T04 is the public-API compile check added by G1. The existing [C0.1 acceptance tests](C0_1_ACCEPTANCE_TESTS.md) remain mandatory and their golden literals stay unchanged. Previously published C0.2 golden blobs stay unchanged. New negative cases are checksum-repaired mutations of those blobs, or single-byte edits that keep the original trailer.

The domain blobs below were packed from the C0.2 schema with Python's standard library. SHA-256 was computed by `hashlib`. The approved event-3 digest was recomputed from H2 and the published event bytes before these blobs were written. A `sealed_sim` snapshot is not evidence. If an implementation disagrees with a vector, fix the implementation or return this document for review.

Hex blocks may wrap. Concatenate the characters and ignore whitespace.

## Acceptance matrix

| ID | Procedure | Required result |
| --- | --- | --- |
| C02-T01a | Slot resume in §1 | Loaded tick-2 blob matches the uninterrupted blob. Event 3 is the approved occupied rejection. Final digest and cursor match |
| C02-T01b | Probe resume in §2 | The stored ordinal produces the published second-probe word. Both domain blobs match |
| C02-T01c | Mint continuity in §3 | IDs 10 and 11 are issued across a restore. IDs 1 through 9 stay occupied. Cargo does not move |
| C02-T01d | Failure matrix in §4 | Each corrupt, foreign, and illegal blob returns the named error. No panic and no partial simulation |
| C02-T01f | Fixture invariants in §4.1 | Checksum-repaired illegal states return `InvalidState`. Every published golden blob still restores |
| C02-T01g | Bounded counts in §4.2 | `u64::MAX` list counts return `Corrupt` without panic or an input-sized allocation |
| C02-T04 | Public API examples in §4c | The listed snippets compile and the fixture 1 assertions pass |
| C02-T01e | File round-trip in §5 | The temp-file image is byte-identical and resumes as T01a |
| C02-T02a | Primary schedule in §6 | Modes switch at ticks 2 and 4. Both tokens move once at tick 3, actor 1 then actor 2. Blobs match |
| C02-T02b | Same-tick phases in §7 | Mint witness and both completion witnesses are scheduled. The mint event precedes the task events |
| C02-T02c | Disabled mint in §8 | One scheduler-command rejection, cursor unchanged, no marker |
| C02-T02d | Drivers and overdue work in §9 | Grouped `drive` calls match single `drive` calls. Overdue work rolls back. Cursor overflow rolls back |
| C02-T03 | Build audit in §10 | Locked format, lint, and tests pass. The dependency graph gains nothing |

A skipped required assertion blocks the gate.

## 1. C02-T01a — slot prefix resume

Construct fixture 1 with seed 100 and both actors enabled.

At boundary 0 submit `ClaimSlot(target=2, actor=2, producer=1, sequence=0)` and `ClaimSlot(target=2, actor=1, producer=1, sequence=0)` in either order. At boundary 1 submit `ClaimSlot(target=3, actor=2, producer=1, sequence=1)`. Save the domain blob. Restore it into a new simulation. At boundary 2 submit an empty batch through `step` or `drive`. The two calls are alternatives on fixture 1. A `step` result is the `Event` sequence. A `drive` result contains the same values as `DomainEvent::Command`, in the same order.

Assertions:

- The restored simulation's domain bytes and SHA-256 equal the source bytes and the tick-2 vector.
- Tick is 2, actor 1 owns the slot, the only pending command is the tick-3 claim, every RNG ordinal is 0, `next_actor_id` is 3, `next_record_id` is 1, both modes are active, and the record tables are empty.
- `checkpoint_bytes()` on this state equals the frozen C0.1 image in §11. That image is rejected by restore as `UnsupportedFormat`.
- Events 1 and 2 equal the C0.1 event bytes, and the digest at the save is H2 `54fabd2a30f91a021e8593391702c18e09e7ff407f0158ea5b532f86ac39acba`.
- The empty boundary-2 `step` or `drive` emits only event 3, bytes `03000000000000000300000000000000020000000100000001000000000000000202`, outcome occupied, tick 3, actor 2, producer 1, sequence 1.
- The resulting digest is `1a1cc22bf1041c19845bceb790e6761dad0edaf41cb3eca1f90b0ce06f1875b1` and `next_event_id` is 4.
- An uninterrupted run from construction, using the same batches, has the same domain bytes at tick 2 and tick 3 and the same three events.
- The prefix events are not emitted again after restore. A second empty `step` or `drive` at boundary 3 emits nothing, leaves the digest and cursor unchanged, and leaves actor 1 as owner.

`SetMode` or `MintRecord` submitted to fixture 1 returns `InvalidPayload`, names the lowest invalid key, and leaves domain bytes unchanged.

## 2. C02-T01b — ordinal resume

On a fresh fixture 1 with seed 100 and both actors enabled, at boundary 0 submit one probe: target 1, actor 1, producer 1, sequence 0, stream 1, draw count 2. Domain bytes match the probe-save vector. `checkpoint_bytes()` still matches the published C0.1 probe-end vector.

Restore. At boundary 1 submit a probe: target 2, actor 1, producer 1, sequence 0, stream 1, draw count 1.

Assertions:

- The new event's only word is `0xdf3be1d5`.
- Event bytes are `0200000000000000020000000000000001000000010000000000000000000000030101d5e13bdf`.
- The digest is `b5dfc10d13778e742283d07e2c7a2973fc2f399e0bff4211064236b7a579e344`.
- The stream-1 actor-1 ordinal is 2. The other three ordinals remain 0.
- Domain bytes match the probe-final vector. The uninterrupted run matches the restored run.

## 3. C02-T01c — cursor continuity

Construct fixture 2 with seed 100, task due tick 3, and both actors enabled. Do not submit mode commands.

At boundary 0 submit `MintRecord(target=1, actor=1, producer=1, sequence=0)`. Domain bytes match the mint-save vector. Restore. At boundary 1 submit `MintRecord(target=2, actor=1, producer=1, sequence=0)`.

Assertions:

- The first mint emits event bytes `0100000000000000010000000000000001000000020a0000000000000001`, digest `39cbd921dff575948405544eea3efa8ce08739359aba260decb5e670c2a65fae`, marker 10, `mode_at_mint` active, and `next_record_id` 11.
- The second mint emits event bytes `0200000000000000020000000000000001000000020b0000000000000001`, digest `c0aedb6c21860638585bd07ad6e1b3fbc279db0b0b93ba9ce437458cde1cf7da`, marker 11, and `next_record_id` 12.
- Record IDs 1 through 9 remain the constructed locations, cargo, tasks, and reservations. Neither mint returns one of those IDs.
- Both cargo tokens remain at location 1. Both reservations remain active. Both tasks remain pending and due at tick 3. No task event exists.
- The uninterrupted run matches the run that restored after the first mint.
- `scheduler(..., task_due_tick: 0, ...)` returns `InvalidDueTick` and does not produce a simulation.

## 4. C02-T01d — corrupt, unsupported, and illegal saves

Start from a known-valid blob where a case needs a body. Recompute the trailer after an intentional body edit. Each call uses `restore_domain_checkpoint` and expects the single error below. The function must not panic.

| Case | Edit | Error |
| --- | --- | --- |
| Empty buffer | Zero bytes | `Truncated` |
| Short header | First 10 bytes of the tick-2 save | `Truncated` |
| Frozen C0.1 initial checkpoint | The published 193-byte `SH:C01:STATE` vector | `UnsupportedFormat` |
| Frozen C0.1 image of the T01 save | The 222-byte image in §11 | `UnsupportedFormat` |
| Future revision | Copy the tick-2 save. Set the `u16` at offset 12 from 1 to 2. Keep every subsequent byte, including the original trailer | `UnsupportedRevision { found: 2 }` |
| Flipped tag | Copy the tick-2 save. Invert the first tag byte at offset 0. Keep the original trailer | `UnsupportedFormat` |
| Flipped body byte | Copy the tick-2 save. Invert the byte at offset 14, the first byte after the tag and revision. Keep the original trailer | `ChecksumMismatch` |
| Trailing byte | Append `00` to a valid blob | `ChecksumMismatch` |
| Leftover body byte | Append `00` to a valid body and recompute the trailer | `Corrupt` |
| Unknown mode | Change an actor mode byte to 9 and recompute the trailer | `Corrupt` |
| Unknown fixture | Set fixture ID to 99, leave a parseable body, recompute the trailer | `UnknownFixture { found: 99 }` |
| Wrong content hash | Replace fixture 1's content hash with 32 zero bytes and recompute the trailer | `ContentMismatch` |
| Unsorted commands | Swap two pending command records in the tick-1 scheduler blob and recompute the trailer | `InvalidState` |
| Overdue task | On the fixture-2 tick-0 blob, set a pending task due tick to 0 and recompute the trailer | `InvalidState` |
| Dangling owner | Set a cargo owner to 99 and recompute the trailer | `InvalidState` |
| Cursor reuse | Set `next_record_id` to 9 on a fixture-2 blob and recompute the trailer | `InvalidState` |

A successful restore of the unedited source still matches that source. Failure of one case does not change the bytes of a simulation already held by the test.

The three precedence rows are one valid blob edited in a single place. The tag row must not be reported as `ChecksumMismatch`. The revision row must not be reported as `ChecksumMismatch` or `Truncated`. The offset-14 row must not be reported as `UnsupportedFormat`, `UnsupportedRevision`, or `Corrupt`.

## 4c. C02-T04 — public API examples

The implementation adds these examples as crate doctests. They must compile and pass. They are the source-compatibility check for B02.

```rust
use sealed_sim::{
    ActorId, Command, CommandKey, DomainEvent, Event, FixtureConfig, GeneratedOutcome, Outcome,
    Payload, SchedulerCommandOutcome, Simulation, StepError,
};

let mut sim = Simulation::new(FixtureConfig { seed: 100, actor_enabled: [true, true] });
let claim = Command::new(
    CommandKey {
        target_tick: 1,
        actor_id: ActorId::ONE,
        producer_id: 1,
        sequence: 0,
    },
    Payload::ClaimSlot,
);
let events: Vec<Event> = sim.step(0, &[claim]).unwrap();
assert_eq!(events[0].id(), 1);
assert_eq!(events[0].key(), claim.key());
assert_eq!(events[0].outcome(), Outcome::Claimed);
let claim_bytes = events[0].canonical_bytes();

let mut replay = Simulation::new(FixtureConfig { seed: 100, actor_enabled: [true, true] });
let driven = replay.drive(0, &[claim]).unwrap();
match driven.events()[0] {
    DomainEvent::Command(event) => {
        assert_eq!(event.id(), 1);
        assert_eq!(event.key(), claim.key());
        assert_eq!(event.outcome(), Outcome::Claimed);
        assert_eq!(event.canonical_bytes(), claim_bytes);
    }
    DomainEvent::SchedulerCommand(_) | DomainEvent::Generated(_) => panic!("fixture 1 is command-only"),
}

let mut scheduled = Simulation::scheduler(100, 3, [true, true]).unwrap();
assert_eq!(scheduled.step(0, &[]), Err(StepError::UseDrive));
assert_eq!(scheduled.tick(), 0);
let quiet = scheduled.drive(0, &[]).unwrap();
assert!(quiet.events().is_empty());
```

The fixture 2 completion and mint scenarios in §3, §6, §7, and §8 call `drive` and inspect `DomainEvent`. A mint success matches `SchedulerCommandOutcome::RecordMinted` and `SchedulerCommandEvent::key()` equals the submitted `MintRecord` key. A disabled mint matches `SchedulerCommandOutcome::Rejected`. A task completion matches `GeneratedOutcome::TaskCompleted` and is a `GeneratedEvent`: the test uses `id()`, `tick()`, `actor_id()`, and `canonical_bytes()`, and it does not read a command key. Concatenating `canonical_bytes()` in `DriveOutput::events()` order reproduces the published event bytes and digest updates.

## 4.1. C02-T01f — fixture-specific illegal states

Each case starts from the named published blob, applies one edit, recomputes the SHA-256 trailer over the edited body, and restores. The repaired trailer is what makes the expected error semantic rather than `ChecksumMismatch`. The call must not panic.

| Case | Base and edit | Error |
| --- | --- | --- |
| Fixture 1 scheduled mode | Tick-2 slot save. Set actor 1's mode byte from 1 to 2 | `InvalidState` |
| Fixture 1 scheduler cursor | Tick-2 slot save. Set `next_record_id` from 1 to 2 | `InvalidState` |
| Fixture 1 foreign payload | Tick-2 slot save. Change the pending claim tag from 1 to 4 (`MintRecord`) | `InvalidState` |
| Fixture 1 probe stream | Probe-save blob, whose pending count is 0. Set that count to 1 and insert one command before the ordinal list: target 2, actor 1, producer 1, sequence 0, payload bytes `02 03 01` | `InvalidState` |
| Fixture 1 probe count | The same insertion with payload bytes `02 01 03` | `InvalidState` |
| Fixture 1 event cursor | Tick-2 slot save. Set `next_event_id` to 0 | `InvalidState` |
| Fixture 1 disabled owner | Tick-2 slot save. Clear actor 1's enabled flag while leaving actor 1 as owner | `InvalidState` |
| Fixture 2 occupied slot | Scheduler tick 0. Set the slot tag to 1 and insert owner actor 1, shifting the following bytes | `InvalidState` |
| Fixture 2 drawn ordinal | Scheduler tick 0. Set the stream-1 actor-1 ordinal from 0 to 1 | `InvalidState` |
| Fixture 2 foreign payload | Scheduler tick 1. Change the first pending payload tag from 3 to 1 and delete the following mode byte so the record is a one-byte `ClaimSlot` | `InvalidState` |
| Fixture 2 split completion | Scheduler tick 0. Mark task 6 completed, release reservation 8, move cargo 4 to location 2, and set `mode_at_completion` to 1. Leave task 7 pending | `InvalidState` |
| Fixture 2 split due ticks | Scheduler tick 0. Set task 7's due tick to 4 and leave task 6 due at 3 | `InvalidState` |
| Fixture 2 event cursor | Scheduler tick 0. Set `next_event_id` to 0 | `InvalidState` |
| Fixture 2 missing cargo | Scheduler tick 0. Set the cargo count to 0 and remove both cargo records | `InvalidState` |

Positive controls, with no edit: the tick-2 and tick-3 slot blobs, the probe save and probe final blobs, the mint save and mint final blobs, and scheduler ticks 0 through 4, the witness blobs, and the disabled-mint blob all restore to equal domain bytes.

The probe-stream and probe-count bodies are not new golden states. They exist only as negative fixtures. A legal probe ordinal remains the published probe-save blob, whose stream-1 actor-1 ordinal is 1 and whose pending list is empty, and that blob must still restore.

## 4.2. C02-T01g — bounded untrusted counts

Start from the scheduler tick-0 body. Replace one `u64` count with `u64::MAX`, recompute the trailer, and restore. Do this separately for the actor count, the pending-command count, and the cargo count. Each call returns `Corrupt`, does not panic, and does not allocate storage proportional to `u64::MAX`. A second variant sets the cargo count to one more than the number of remaining body bytes and recomputes the trailer. That call also returns `Corrupt`.

The unedited tick-0 blob still restores.

## 5. C02-T01e — file image

Write the T01a tick-2 simulation to a new file under the process temp directory. Read it back. The file contents equal `domain_checkpoint_bytes`. The loaded fixture 1 simulation resumes through an empty boundary-2 `step` or `drive` to the tick-3 vector.

Write only the first 10 bytes of that blob to a second file. Reading it returns `CheckpointReadError::Restore(Truncated)`. Reading a missing path returns `CheckpointReadError::Io` and no simulation. A failed write does not change the source simulation's domain bytes.

## 6. C02-T02a — primary mode and cargo oracle

Construct fixture 2 with seed 100, task due tick 3, and both actors enabled. The tick-0 domain blob matches §11 before any command is submitted. Both actors are active. Cargo 4 and 5 are owned by location 1. Reservations 8 and 9 are active. Tasks 6 and 7 are pending. `next_record_id` is 10. The digest is fixture 2 H0.

At boundary 0 submit, in any order:

- `SetMode(Scheduled)` target 2, actor 1, producer 1, sequence 0
- `SetMode(Scheduled)` target 2, actor 2, producer 1, sequence 0
- `SetMode(Active)` target 4, actor 1, producer 1, sequence 0
- `SetMode(Active)` target 4, actor 2, producer 1, sequence 0

Then call `drive` with an empty batch at boundaries 1, 2, and 3. Do not use `step` for these fixture 2 transitions.

| After boundary | Tick | Modes | Work and cargo | Events |
| --- | --- | --- | --- | --- |
| 0 | 1 | Both active | Tokens at origin, reservations active, four commands pending | None, digest still H0 |
| 1 | 2 | Both scheduled | Same cargo and reservations. Only the two return-to-active commands remain | None, digest still H0 |
| 2 | 3 | Both scheduled | Token 4 at location 2, token 5 at location 3, reservations released, tasks completed with `mode_at_completion` scheduled | Task 6 for actor 1, then task 7 for actor 2 |
| 3 | 4 | Both active | Same completed cargo, reservations, tasks, cursor, and digest | None |

Event 1 bytes: `01000000000000000300000000000000010000000106000000000000000800000000000000040000000000000001000000000000000200000000000000`  
Digest after event 1: `f9c54cce1eed252da770ffb4798d065cb255961a0fddd019d98d6168d776b70f`  
Event 2 bytes: `02000000000000000300000000000000020000000107000000000000000900000000000000050000000000000001000000000000000300000000000000`  
Digest after event 2: `c48936ae6b6e4245f63212280cb07659eab2e9dd2c189c2cf46d7e70d3a3dd67`

Further required checks:

- Domain blobs at ticks 0, 1, 2, 3, and 4 match §11.
- Exactly two cargo records exist at every tick. No token is duplicated, destroyed, or owned by two locations.
- No completion event exists before tick 3 or after it. One more empty `drive` at boundary 4 advances to tick 5, emits nothing, and leaves owners, reservations, digest, and `next_event_id` unchanged.
- Actor IDs remain 1 and 2. `next_actor_id` remains 3. `next_record_id` remains 10.
- The slot stays vacant and all four ordinals stay 0.
- `ClaimSlot` or `Probe` on fixture 2 is `InvalidPayload` and rolls nothing forward.
- Save at tick 2, restore, and `drive` the same empty batches at boundaries 2 and 3. Split bytes, suffix events, and tick-4 bytes equal the uninterrupted `drive` run. `step` on the restored fixture 2 simulation returns `UseDrive` and does not change its bytes.

## 7. C02-T02b — mode before commands and tasks

Construct fixture 2 with seed 100, task due tick 2, and both actors enabled. At boundary 0 submit:

- `MintRecord` target 2, actor 1, producer 1, sequence 0
- `SetMode(Scheduled)` target 2, actor 1, producer 1, sequence 1
- `SetMode(Scheduled)` target 2, actor 2, producer 1, sequence 0

The mint's command key sorts before actor 1's mode command. One shared queue would mint while actor 1 is still active and would record `mode_at_mint` active. The required phased runner applies both due mode commands before any mint or task, so both witnesses record scheduled.

At tick 1 the domain blob matches the witness tick-1 vector, both modes are still active, and no event has been emitted. One empty `drive` at boundary 1 reaches tick 2.

Assertions:

- Domain bytes match the witness tick-2 vector.
- Event order is mint, task 6, task 7.
- Mint bytes `0100000000000000020000000000000001000000020a0000000000000002`, with `mode_at_mint` scheduled. Intermediate digest `7a0f89b2b1ff28e4586615339b6bb757916f9ef141b27c9df1cd825a2c0f8ffb`.
- Task event bytes equal the witness task bytes in §11. Final digest `4dfb4804407daa5cc8bffe951d11ca8830e2fae4c6aa1b01f9e372956f9dfb93`.
- Both tasks store `mode_at_completion` scheduled. Marker 10 stores scheduled. Both actor modes are scheduled.
- Cargo moved once, reservations released once, `next_record_id` is 11, and `next_event_id` is 4.

If the work phase ran before the mode phase, `mode_at_completion` would be active and the test fails. If the command phase ran before the mode phase, `mode_at_mint` would be active and the test fails.

## 8. C02-T02c — disabled mint

Construct fixture 2 with seed 100, task due tick 3, actor 1 disabled, and actor 2 enabled. At boundary 0 submit `MintRecord(target=1, actor=1, producer=1, sequence=0)`.

Assertions:

- Event bytes are `01000000000000000100000000000000010000000301`.
- Digest is `55735de709c23c26eebe840a8c10b4fb560a19789232a740cdfcb6a3e69e60af`.
- Domain bytes match the disabled-mint vector.
- `next_record_id` remains 10. The marker table is empty.
- Cargo, reservations, and tasks are unchanged from construction.

## 9. C02-T02d — grouped steps, overdue work, overflow

Run §6 twice. One driver calls `drive` immediately for each boundary. The other records the same batches and later calls `drive` once per boundary in a loop. Compare domain bytes and the concatenated canonical event bytes after every boundary. They match, including the tick-2 save and the tick-4 result.

On a fixture-2 simulation at tick 0, set one pending task's due tick to 0 without going through restore. `drive` an empty batch at boundary 0. The result is `StepError::InvalidState`. Domain bytes, including the illegally edited due tick, are unchanged. Restoring a checksum-valid encoding of that overdue state still returns `RestoreError::InvalidState`, as in §4.

Set `next_record_id` to `u64::MAX` on a fresh fixture-2 simulation and `drive` an enabled `MintRecord` due on the next tick. The call returns `CapacityExceeded`, emits nothing, and restores the prior domain bytes, including the cursor. This is the same rollback class as the existing tick, event-cursor, and ordinal overflow tests. `step` on this fixture still returns `UseDrive` without mutation.

## 10. C02-T03 — build audit

From the package root, with dependencies already fetched from the committed lockfile:

```sh
cargo fmt --all --check
cargo check --locked --offline
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --locked --offline
cargo test --release --locked --offline
cargo tree -p sealed_sim --locked --offline -e normal,build,dev
```

Required observations:

- The production package is still only `sealed_sim`.
- Direct dependencies are still exact `rand_chacha` 0.10.0 without default features, and exact `sha2` 0.11.1.
- No Bevy, serde, renderer, windowing, networking, or OS entropy API is introduced.
- C0.1 golden literals in `src/tests/golden.rs` are unchanged.
- Debug and release both pass the C0.1 tests and every C0.2 assertion. No required test is ignored.

Record the toolchain, target triple, lockfile SHA-256, commands, and test counts in the implementation report. A documentation review is not that report.

## 11. Golden vectors

Fixture 2 content SHA-256: `043d3507909cf4718d94488279deb1943dfa69530f9e86b109e381c3cbd2dd92`  
Fixture 2 H0: `413408f703da057f8b39023e580c8c65649376824d1a77be0a4d24aaf048dd0b`

### Frozen C0.1 image of the T01 tick-2 save

222 bytes. SHA-256 `52dfa0b4fe5b1f1237f9d23f6d06606e70954beb030b907eaae2b68f7f3471a8`

```text
53483a4330313a5354415445010001000000042797b9b5032aaf66af7473dd8f
f50a349c865a5b4f147405446f78dad96c590200000000000000640000000000
0000020000000000000001000000010200000001010100000001000000000000
0003000000000000000200000001000000010000000000000001040000000000
0000010100000000000000000000000102000000000000000000000002010000
00000000000000000002020000000000000000000000030000000000000054fa
bd2a30f91a021e8593391702c18e09e7ff407f0158ea5b532f86ac39acba
```

### T01 domain save at tick 2

312 bytes. SHA-256 `446a3b22d12c800c95c40811038531422afe419fb8b9923bba2df6381c3ee6f7`

```text
53483a4330323a5354415445010001000000042797b9b5032aaf66af7473dd8f
f50a349c865a5b4f147405446f78dad96c590200000000000000640000000000
0000030000000000000001000000000000000200000000000000010000000101
0200000001010101000000010000000000000003000000000000000200000001
0000000100000000000000010400000000000000010100000000000000000000
0001020000000000000000000000020100000000000000000000000202000000
0000000000000000030000000000000054fabd2a30f91a021e8593391702c18e
09e7ff407f0158ea5b532f86ac39acba00000000000000000000000000000000
000000000000000000000000000000000000000000000000f68be05da3550604
513cfa062494d087f4ff74cee3725bb54f456cd30d878c1e
```

### T01 domain final at tick 3

287 bytes. SHA-256 `53939438b57d0dcc685929484ac15f54eea1a5f82fff44b8862943de2fadf3ae`

```text
53483a4330323a5354415445010001000000042797b9b5032aaf66af7473dd8f
f50a349c865a5b4f147405446f78dad96c590300000000000000640000000000
0000030000000000000001000000000000000200000000000000010000000101
0200000001010101000000000000000000000004000000000000000101000000
0000000000000000010200000000000000000000000201000000000000000000
00000202000000000000000000000004000000000000001a1cc22bf1041c1984
5bceb790e6761dad0edaf41cb3eca1f90b0ce06f1875b1000000000000000000
0000000000000000000000000000000000000000000000000000000000000084
28cb2ec330ff1b90e3cf06126838cdc7561a39f8ece7a510a89e3fc19af558
```

### Probe save at tick 1

283 bytes. SHA-256 `692b28c34f716c1b011b72567afb7cbc5a414abc7a2888078456108482b5ca19`

```text
53483a4330323a5354415445010001000000042797b9b5032aaf66af7473dd8f
f50a349c865a5b4f147405446f78dad96c590100000000000000640000000000
0000030000000000000001000000000000000200000000000000010000000101
0200000001010000000000000000000400000000000000010100000001000000
0000000001020000000000000000000000020100000000000000000000000202
000000000000000000000002000000000000003b0ee130dab2cc1d830bda3c2e
544bf63b6beb02a9ab4b3b09a73650f32f62a900000000000000000000000000
000000000000000000000000000000000000000000000000000000bd04247026
7682c70324f4d396ad26b34e4da1b803a71da9374e93664bf9de22
```

### Probe final at tick 2

283 bytes. SHA-256 `c6f3cc97711dc04c9d265ef3dcb4f5573a01c37c2937ac8fd3559b7b9f44a88c`

```text
53483a4330323a5354415445010001000000042797b9b5032aaf66af7473dd8f
f50a349c865a5b4f147405446f78dad96c590200000000000000640000000000
0000030000000000000001000000000000000200000000000000010000000101
0200000001010000000000000000000400000000000000010100000002000000
0000000001020000000000000000000000020100000000000000000000000202
00000000000000000000000300000000000000b5dfc10d13778e742283d07e2c
7a2973fc2f399e0bff4211064236b7a579e34400000000000000000000000000
0000000000000000000000000000000000000000000000000000007dc0129a40
52951480504a7de6c40d29dd83b1f20f444b0e2f5065f29bf781fa
```

### Scheduler tick 0

489 bytes. SHA-256 `ec15e2640662f0d9e0637c7ca80f514a99534ea523bece481767831dc0b35f43`

```text
53483a4330323a5354415445010002000000043d3507909cf4718d94488279de
b1943dfa69530f9e86b109e381c3cbd2dd920000000000000000640000000000
000003000000000000000a000000000000000200000000000000010000000101
0200000001010000000000000000000400000000000000010100000000000000
0000000001020000000000000000000000020100000000000000000000000202
00000000000000000000000100000000000000413408f703da057f8b39023e58
0c8c65649376824d1a77be0a4d24aaf048dd0b03000000000000000100000000
0000000200000000000000030000000000000002000000000000000400000000
0000000100000000000000050000000000000001000000000000000200000000
0000000800000000000000040000000000000001000000060000000000000001
0900000000000000050000000000000002000000070000000000000001020000
0000000000060000000000000001000000030000000000000004000000000000
0008000000000000000200000000000000010007000000000000000200000003
0000000000000005000000000000000900000000000000030000000000000001
000000000000000000f50669507ea889904e167578b26fe54991e188afc520bb
4562df29a00055688c
```

### Scheduler tick 1

593 bytes. SHA-256 `b07b490a45dda0b5aee1d7db3738366094d3f281e6ad85bc0bb172c143bfe941`

```text
53483a4330323a5354415445010002000000043d3507909cf4718d94488279de
b1943dfa69530f9e86b109e381c3cbd2dd920100000000000000640000000000
000003000000000000000a000000000000000200000000000000010000000101
0200000001010004000000000000000200000000000000010000000100000000
0000000000000003020200000000000000020000000100000000000000000000
0003020400000000000000010000000100000000000000000000000301040000
0000000000020000000100000000000000000000000301040000000000000001
0100000000000000000000000102000000000000000000000002010000000000
000000000000020200000000000000000000000100000000000000413408f703
da057f8b39023e580c8c65649376824d1a77be0a4d24aaf048dd0b0300000000
0000000100000000000000020000000000000003000000000000000200000000
0000000400000000000000010000000000000005000000000000000100000000
0000000200000000000000080000000000000004000000000000000100000006
0000000000000001090000000000000005000000000000000200000007000000
0000000001020000000000000006000000000000000100000003000000000000
0004000000000000000800000000000000020000000000000001000700000000
0000000200000003000000000000000500000000000000090000000000000003
00000000000000010000000000000000000362da7405c839d0c2e7d908349a3a
7839fa0469969b7a8fc4038c32ab2467be
```

### Scheduler tick 2

541 bytes. SHA-256 `5469e9fb226511967f75875a8ee3386f6db928c4fa612e57fd0531c39495d8e5`

```text
53483a4330323a5354415445010002000000043d3507909cf4718d94488279de
b1943dfa69530f9e86b109e381c3cbd2dd920200000000000000640000000000
000003000000000000000a000000000000000200000000000000010000000102
0200000001020002000000000000000400000000000000010000000100000000
0000000000000003010400000000000000020000000100000000000000000000
0003010400000000000000010100000000000000000000000102000000000000
0000000000020100000000000000000000000202000000000000000000000001
00000000000000413408f703da057f8b39023e580c8c65649376824d1a77be0a
4d24aaf048dd0b03000000000000000100000000000000020000000000000003
0000000000000002000000000000000400000000000000010000000000000005
0000000000000001000000000000000200000000000000080000000000000004
0000000000000001000000060000000000000001090000000000000005000000
0000000002000000070000000000000001020000000000000006000000000000
0001000000030000000000000004000000000000000800000000000000020000
0000000000010007000000000000000200000003000000000000000500000000
00000009000000000000000300000000000000010000000000000000002784ee
0991e9e72002357ba570a1a46453713dcd0ff256895a207b3d76471dc2
```

### Scheduler tick 3

541 bytes. SHA-256 `9f17aa0b61d211a6a57e06b053296233b78abb7532a716b3ed6f10210160df0d`

```text
53483a4330323a5354415445010002000000043d3507909cf4718d94488279de
b1943dfa69530f9e86b109e381c3cbd2dd920300000000000000640000000000
000003000000000000000a000000000000000200000000000000010000000102
0200000001020002000000000000000400000000000000010000000100000000
0000000000000003010400000000000000020000000100000000000000000000
0003010400000000000000010100000000000000000000000102000000000000
0000000000020100000000000000000000000202000000000000000000000003
00000000000000c48936ae6b6e4245f63212280cb07659eab2e9dd2c189c2cf4
6d7e70d3a3dd6703000000000000000100000000000000020000000000000003
0000000000000002000000000000000400000000000000020000000000000005
0000000000000003000000000000000200000000000000080000000000000004
0000000000000001000000060000000000000002090000000000000005000000
0000000002000000070000000000000002020000000000000006000000000000
0001000000030000000000000004000000000000000800000000000000020000
0000000000020207000000000000000200000003000000000000000500000000
0000000900000000000000030000000000000002020000000000000000c71eb0
fe4d6e9ccb5396c51cc18fc924561d43bde30e8eb035b9968c45e8d227
```

### Scheduler tick 4

489 bytes. SHA-256 `bc41da60f5626a252ec520593b3ff8ca50875233d53c134694d5d682ce0bbca6`

```text
53483a4330323a5354415445010002000000043d3507909cf4718d94488279de
b1943dfa69530f9e86b109e381c3cbd2dd920400000000000000640000000000
000003000000000000000a000000000000000200000000000000010000000101
0200000001010000000000000000000400000000000000010100000000000000
0000000001020000000000000000000000020100000000000000000000000202
00000000000000000000000300000000000000c48936ae6b6e4245f63212280c
b07659eab2e9dd2c189c2cf46d7e70d3a3dd6703000000000000000100000000
0000000200000000000000030000000000000002000000000000000400000000
0000000200000000000000050000000000000003000000000000000200000000
0000000800000000000000040000000000000001000000060000000000000002
0900000000000000050000000000000002000000070000000000000002020000
0000000000060000000000000001000000030000000000000004000000000000
0008000000000000000200000000000000020207000000000000000200000003
0000000000000005000000000000000900000000000000030000000000000002
02000000000000000065cc9e6cca4ba583d33d9eb01ca51df9e110150329d71e
a526df6b683684e8c0
```

### Mint save and final

Save, 498 bytes, SHA-256 `0fed0d7f167c0817619528641f4201326f9522a1cf6435b81b8d8c88c8ba04c7`

```text
53483a4330323a5354415445010002000000043d3507909cf4718d94488279de
b1943dfa69530f9e86b109e381c3cbd2dd920100000000000000640000000000
000003000000000000000b000000000000000200000000000000010000000101
0200000001010000000000000000000400000000000000010100000000000000
0000000001020000000000000000000000020100000000000000000000000202
0000000000000000000000020000000000000039cbd921dff575948405544eea
3efa8ce08739359aba260decb5e670c2a65fae03000000000000000100000000
0000000200000000000000030000000000000002000000000000000400000000
0000000100000000000000050000000000000001000000000000000200000000
0000000800000000000000040000000000000001000000060000000000000001
0900000000000000050000000000000002000000070000000000000001020000
0000000000060000000000000001000000030000000000000004000000000000
0008000000000000000200000000000000010007000000000000000200000003
0000000000000005000000000000000900000000000000030000000000000001
0001000000000000000a0000000000000001a93ece36d182f3b54b6f8cd44b0a
164dc02a3c15de0c5b1a3547db562bee5bd2
```

Final, 507 bytes, SHA-256 `c07b19b6ac711b61fa82cb86a7a8173a5e28b89fd24f6f26822872bec4dd5b2a`

```text
53483a4330323a5354415445010002000000043d3507909cf4718d94488279de
b1943dfa69530f9e86b109e381c3cbd2dd920200000000000000640000000000
000003000000000000000c000000000000000200000000000000010000000101
0200000001010000000000000000000400000000000000010100000000000000
0000000001020000000000000000000000020100000000000000000000000202
00000000000000000000000300000000000000c0aedb6c21860638585bd07ad6
e1b3fbc279db0b0b93ba9ce437458cde1cf7da03000000000000000100000000
0000000200000000000000030000000000000002000000000000000400000000
0000000100000000000000050000000000000001000000000000000200000000
0000000800000000000000040000000000000001000000060000000000000001
0900000000000000050000000000000002000000070000000000000001020000
0000000000060000000000000001000000030000000000000004000000000000
0008000000000000000200000000000000010007000000000000000200000003
0000000000000005000000000000000900000000000000030000000000000001
0002000000000000000a00000000000000010b0000000000000001ab8dea2638
1043966ff3c7109a5b207c3686ca436bac5cfc5ffb7c9dfc0fbade
```

### Phase witness

Tick 1, 566 bytes, SHA-256 `35b5104ca23dc7723baffbe76948a044fdda583eb8374604149133986e2f4ac2`

```text
53483a4330323a5354415445010002000000043d3507909cf4718d94488279de
b1943dfa69530f9e86b109e381c3cbd2dd920100000000000000640000000000
000003000000000000000a000000000000000200000000000000010000000101
0200000001010003000000000000000200000000000000010000000100000000
0000000000000004020000000000000001000000010000000100000000000000
0302020000000000000002000000010000000000000000000000030204000000
0000000001010000000000000000000000010200000000000000000000000201
0000000000000000000000020200000000000000000000000100000000000000
413408f703da057f8b39023e580c8c65649376824d1a77be0a4d24aaf048dd0b
0300000000000000010000000000000002000000000000000300000000000000
0200000000000000040000000000000001000000000000000500000000000000
0100000000000000020000000000000008000000000000000400000000000000
0100000006000000000000000109000000000000000500000000000000020000
0007000000000000000102000000000000000600000000000000010000000200
0000000000000400000000000000080000000000000002000000000000000100
0700000000000000020000000200000000000000050000000000000009000000
00000000030000000000000001000000000000000000c6a16ad4d53c34bc560e
15b81243685e282ff1ea096ad1ad90e77993493f3d58
```

Tick 2, 498 bytes, SHA-256 `8f67f35b563d0c8495c83e0377638be27e4839c8264e2ddc045f7662805ed2cb`

```text
53483a4330323a5354415445010002000000043d3507909cf4718d94488279de
b1943dfa69530f9e86b109e381c3cbd2dd920200000000000000640000000000
000003000000000000000b000000000000000200000000000000010000000102
0200000001020000000000000000000400000000000000010100000000000000
0000000001020000000000000000000000020100000000000000000000000202
000000000000000000000004000000000000004dfb4804407daa5cc8bffe951d
11ca8830e2fae4c6aa1b01f9e372956f9dfb9303000000000000000100000000
0000000200000000000000030000000000000002000000000000000400000000
0000000200000000000000050000000000000003000000000000000200000000
0000000800000000000000040000000000000001000000060000000000000002
0900000000000000050000000000000002000000070000000000000002020000
0000000000060000000000000001000000020000000000000004000000000000
0008000000000000000200000000000000020207000000000000000200000002
0000000000000005000000000000000900000000000000030000000000000002
0201000000000000000a0000000000000002e84ed0a2924425308564bad9ef73
6b86d8504650d22e36c8b321b7777517c6ce
```

Witness task 1 bytes: `02000000000000000200000000000000010000000106000000000000000800000000000000040000000000000001000000000000000200000000000000`  
Digest after task 1: `580e89486370bf77e6dcd7b811a0085cd540bbae091c0711efcd39fecf989a9b`  
Witness task 2 bytes: `03000000000000000200000000000000020000000107000000000000000900000000000000050000000000000001000000000000000300000000000000`

### Disabled mint

489 bytes. SHA-256 `d7aef700466ecf34bae79f348f62ab60ef5beba65bdf280724820218cd1a6aa4`

```text
53483a4330323a5354415445010002000000043d3507909cf4718d94488279de
b1943dfa69530f9e86b109e381c3cbd2dd920100000000000000640000000000
000003000000000000000a000000000000000200000000000000010000000001
0200000001010000000000000000000400000000000000010100000000000000
0000000001020000000000000000000000020100000000000000000000000202
0000000000000000000000020000000000000055735de709c23c26eebe840a8c
10b4fb560a19789232a740cdfcb6a3e69e60af03000000000000000100000000
0000000200000000000000030000000000000002000000000000000400000000
0000000100000000000000050000000000000001000000000000000200000000
0000000800000000000000040000000000000001000000060000000000000001
0900000000000000050000000000000002000000070000000000000001020000
0000000000060000000000000001000000030000000000000004000000000000
0008000000000000000200000000000000010007000000000000000200000003
0000000000000005000000000000000900000000000000030000000000000001
0000000000000000000d090a10e3997f1b65ce41338ec627b55ba1d5a78681b9
c90d608ad22701e4c4
```
