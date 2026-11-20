# C0.2 Implementation Specification

Version 0.1-review-r1.1 · 17 November 2026  
Status: G1 conditional-pass polish. Implementation is not authorized.

G1 returned FAIL on two blockers. B01 is closed by the fixture-specific restore rules in §10. B02 is closed by the event API in §4 and §9. M01 and M02 are closed by the checksum precedence tests and the bounded decoder rule in §10. No C0.1 byte and no previously published C0.2 golden blob changes.

This document is the normative C0.2 scope, schema, and runtime contract. [Acceptance tests](C0_2_ACCEPTANCE_TESTS.md) define the executable vectors. [Architecture review](C0_2_ARCHITECTURE_REVIEW.md) records inspection, verification, and the decisions closed here. C0.2 extends the frozen [C0.1 specification](C0_1_IMPLEMENTATION_SPEC.md). It does not replace it.

## 1. Authority and freeze

C0.1 remains the behavior of fixture 1 for command ordering, admission, slot and probe commits, ChaCha12 derivation, consequential-event bytes, the `SH:C01:EVENT:v1` digest chain, and `checkpoint_bytes()` for every state the C0.1 API can reach. The published C0.1 golden vectors stay in place. C0.2 adds a second observation, the domain checkpoint, and a second fixture. It does not rewrite a C0.1 field, tag, or discriminant.

G0 already binds the following, and this specification only makes them executable:

- ADR-003: deterministic persistent IDs, allocated in C0.2, never reused.
- ADR-009: the simulation alone selects active or scheduled mode. Visibility is not an input.
- ADR-017: a versioned domain snapshot plus the admitted pending queue, event cursor, and prefix digest.
- ADR-022: loaded-prefix continuity, including the approved event 3 and its digest.
- Core §11 and Technical Architecture §8: persist every future-affecting field; reject invalid and unsupported saves; do not store Bevy handles.
- LOD §4: mode triggers before due work; work order is `(due tick, actor ID, work ID)` ascending; no approximation in the C0.2 oracle.

Where G0 names a result without a byte layout, the layout in this document is the proposed resolution. Those resolutions are listed in §14. They are part of what review approves. They are not optional implementation choices.

## 2. Scope

C0.2 stays inside the one library package `sealed_sim`. No new dependency is added. Bevy, serde, JSON, file archives, networking, windowing, and OS entropy stay out. Cultivation, factions, personalities, world generation, pathfinding, needs, production fairness, aggregate cohorts, and idle-tick skipping stay out.

In scope:

- A versioned domain checkpoint blob, including file read and write of that exact blob.
- Restore, validation, and typed failure for corrupt, mismatched, and unsupported input.
- A monotonic record-ID cursor with no free list.
- Fixture 2: two named actors, explicit mode commands, two reserved cargo tokens, and tasks that complete exactly once.
- Equivalence of single-step and grouped single-step replay, including resume from a loaded prefix.

The package version may become `0.2.0` when implementation is authorized. This documentation change does not edit `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, or `src/`.

## 3. Fixtures

| Fixture | ID | Constructor | Payloads admitted | Event dialect |
| --- | --- | --- | --- | --- |
| Slot | 1 | Existing `Simulation::new(FixtureConfig { seed, actor_enabled })` | `ClaimSlot`, `Probe` | Unchanged C0.1 event bytes and `SH:C01:EVENT:v1` |
| Scheduler | 2 | `Simulation::scheduler(seed, task_due_tick, actor_enabled) -> Result<Simulation, FixtureError>` | `SetMode`, `MintRecord` | C0.2 event bytes and `SH:C02:EVENT:v1` |

`FixtureError::InvalidDueTick` is returned when `task_due_tick` is 0. No simulation is created. Both constructors are pure: the same arguments produce the same initial domain bytes.

Fixture 1 content identity remains SHA-256 of the ASCII bytes `SH:C01:FIXTURE:v1` followed by `0a`. Fixture 2 content identity is SHA-256 of the ASCII bytes `SH:C02:FIXTURE:v1` followed by `0a`, which is `043d3507909cf4718d94488279deb1943dfa69530f9e86b109e381c3cbd2dd92`.

Fixture 2 initialization issues record IDs from `next_record_id = 1` in this order, then leaves the cursor at 10. Both tasks receive `task_due_tick`. Both actors start Active. The slot stays vacant. All four RNG ordinals stay 0. The event cursor starts at 1. No events are emitted during construction.

| Order | ID | Record |
| --- | --- | --- |
| 1 | 1 | Location, the shared origin |
| 2 | 2 | Location, actor 1 destination |
| 3 | 3 | Location, actor 2 destination |
| 4 | 4 | Cargo token owned by location 1 |
| 5 | 5 | Cargo token owned by location 1 |
| 6 | 6 | Pending task for actor 1, cargo 4, reservation 8, destination 2 |
| 7 | 7 | Pending task for actor 2, cargo 5, reservation 9, destination 3 |
| 8 | 8 | Active reservation of cargo 4 by actor 1 for task 6 |
| 9 | 9 | Active reservation of cargo 5 by actor 2 for task 7 |

`next_actor_id` is 3 on both fixtures. C0.2 never allocates an actor and never changes this cursor. Actors 1 and 2 remain the only actor IDs.

## 4. Public interface

Existing C0.1 methods keep their signatures and, on fixture 1 states whose pending payloads are only `ClaimSlot` or `Probe`, their bytes. `checkpoint_bytes()` is that frozen observation. It has no mode, cursor, cargo, or checksum trailer. Calling it on fixture 2, or on any state with another payload, panics. Persistence tests use the domain API below.

`ProcessingMode` must derive `Clone`, `Copy`, `Debug`, `Eq`, and `PartialEq`. `Copy` lets `Payload::SetMode` and `SchedulerCommandOutcome::RecordMinted` contain it and lets `processing_mode()` return it by value. The discriminants stay 1 and 2. No encoding byte changes.

`Event` stays the C0.1 struct: private `id`, `key`, and `outcome`, with `id()`, `key()`, `outcome()`, and `canonical_bytes()` unchanged. `Outcome` gains no variants. `canonical_bytes()` for `Claimed`, `Rejected`, and `Probed` stays the frozen C0.1 encoding. Scheduler results are not stuffed into `Event`, because a task completion has no command producer or sequence. Inventing those fields would publish a command identity the kernel never admitted.

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessingMode {
    Active = 1,
    Scheduled = 2,
}

pub enum Payload {
    ClaimSlot,
    Probe { stream_id: u8, draw_count: u8 },
    SetMode(ProcessingMode),
    MintRecord,
}

pub enum FixtureError {
    InvalidDueTick,
}

pub enum StepError {
    WrongBoundary { expected: u64, received: u64 },
    Command { key: CommandKey, reason: InvalidCommand },
    CapacityExceeded,
    InvalidState,
    /// `step` was called on fixture 2. The simulation is unchanged.
    UseDrive,
}

pub enum RestoreError {
    Truncated,
    UnsupportedFormat,
    UnsupportedRevision { found: u16 },
    ChecksumMismatch,
    Corrupt,
    UnknownFixture { found: u32 },
    ContentMismatch,
    InvalidState,
}

pub enum CheckpointReadError {
    Io(std::io::Error),
    Restore(RestoreError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SchedulerCommandEvent {
    id: u64,
    key: CommandKey,
    outcome: SchedulerCommandOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerCommandOutcome {
    RecordMinted { marker_id: u64, mode_at_mint: ProcessingMode },
    Rejected(SchedulerCommandRejection),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerCommandRejection {
    DisabledActor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratedEvent {
    id: u64,
    tick: u64,
    actor_id: ActorId,
    outcome: GeneratedOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeneratedOutcome {
    TaskCompleted {
        task_id: u64,
        reservation_id: u64,
        cargo_id: u64,
        origin_location_id: u64,
        destination_location_id: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomainEvent {
    Command(Event),
    SchedulerCommand(SchedulerCommandEvent),
    Generated(GeneratedEvent),
}

pub struct DriveOutput {
    events: Vec<DomainEvent>,
}
```

`InvalidCommand` gains no variants. A payload legal on the other fixture is `InvalidPayload`.

Method contracts:

| Method | Contract |
| --- | --- |
| `Event::id(self) -> u64` | Unchanged. The shared cursor value at publication |
| `Event::key(self) -> CommandKey` | Unchanged. The admitted command's own key. Its target tick is the tick the event committed |
| `Event::outcome(self) -> Outcome` | Unchanged. Only `Claimed`, `Rejected`, and `Probed` |
| `Event::canonical_bytes(self) -> Vec<u8>` | Unchanged C0.1 bytes for those three outcomes |
| `SchedulerCommandEvent::id/key/outcome` | `const fn` accessors. `key()` is the real `MintRecord` key. No stand-in producer or sequence |
| `SchedulerCommandEvent::canonical_bytes(self) -> Vec<u8>` | Fixture 2 command-event bytes in §9 |
| `GeneratedEvent::id/tick/actor_id/outcome` | `const fn` accessors. There is no `key()` method |
| `GeneratedEvent::canonical_bytes(self) -> Vec<u8>` | Fixture 2 task-event bytes in §9 |
| `DomainEvent::id(self) -> u64` | The id of the contained event |
| `DomainEvent::canonical_bytes(self) -> Vec<u8>` | The contained event's canonical bytes |
| `DriveOutput::events(&self) -> &[DomainEvent]` | Emission order for that successful transition |
| `Simulation::step(&mut self, u64, &[Command]) -> Result<Vec<Event>, StepError>` | Fixture 1 only, same signature and results as C0.1. Fixture 2 returns `UseDrive` and does not mutate |
| `Simulation::drive(&mut self, u64, &[Command]) -> Result<DriveOutput, StepError>` | Both fixtures. Fixture 1 results are `DomainEvent::Command` values equal to the `Event`s `step` would have returned |
| `domain_checkpoint_bytes(&self) -> Vec<u8>` | Canonical blob in §7, including the trailer |
| `domain_checkpoint_sha256(&self) -> [u8; 32]` | SHA-256 of that entire blob |
| `restore_domain_checkpoint(&[u8]) -> Result<Simulation, RestoreError>` | New value, or an error and no value. Does not panic on any byte string |
| `write_domain_checkpoint(&self, path) -> io::Result<()>` | Writes exactly `domain_checkpoint_bytes`. Does not mutate the simulation |
| `read_domain_checkpoint(path) -> Result<Simulation, CheckpointReadError>` | Reads the file, then uses the same restore rules. Malformed bytes do not panic |
| `processing_mode(&self, ActorId) -> Option<ProcessingMode>` | `None` for an unknown actor |
| `next_actor_id(&self) -> u64` | Always 3 for a valid simulation |
| `next_record_id(&self) -> u64` | Cursor of the next record ID |
| `cargo_ids`, `cargo_owner`, `reservation_state`, `task_view`, `marker_ids` | Read-only views, ascending ID |

`step` and `drive` share one transition implementation. A fixture 1 caller uses one of them for a given boundary, not both. `StepError::InvalidState` and `RestoreError::InvalidState` are distinct types. A failed `step` or `drive` leaves the previous simulation byte-identical. A failed restore does not borrow a caller-supplied simulation.

`write_domain_checkpoint` uses ordinary file replacement. C0.2 does not promise a crash-safe rename. A short or torn file must fail restore. The simulation itself remains unchanged if the write returns an I/O error.

## 5. Tick phases

`drive` keeps the C0.1 preflight and batch-atomic admission rules. On fixture 2, `step` returns `UseDrive` before those checks and before any mutation. Diagnostic precedence stays `UseDrive` for that misuse, then `WrongBoundary`, then `LateTarget`, `UnknownActor`, `InvalidProducer`, `InvalidPayload`, then `ConflictingIdentity`. Preflight does not read the slot, the enabled flag, the mode, or cargo. Fixture 1 rejects `SetMode` and `MintRecord`. Fixture 2 rejects `ClaimSlot` and `Probe`.

After a successful preflight the transition clones the simulation and then:

1. Admit the batch into the staged pending map. Identical key and payload collapse. Identical key and different payload returns `ConflictingIdentity` and discards the stage.
2. Advance the staged tick with checked addition. Overflow returns `CapacityExceeded` and discards the stage.
3. **Mode phase.** Remove `SetMode` commands whose target equals the staged tick, in command-key order, and set that actor's mode. This phase emits no event, draws no RNG, and allocates nothing.
4. **Command phase.** Remove remaining commands whose target equals the staged tick, in command-key order. `ClaimSlot` and `Probe` use the frozen C0.1 commit and append `DomainEvent::Command`. `MintRecord` uses §6 and appends `DomainEvent::SchedulerCommand`. Each published event extends that fixture's digest.
5. **Work phase.** If any pending task has `due_tick` less than the staged tick, return `InvalidState` and discard the stage. Then take pending tasks with `due_tick` equal to the staged tick, ordered by `(due_tick, actor_id, task_id)` ascending, complete each one exactly once using §8, and append `DomainEvent::Generated`.

Mode commands are not eligible for the command phase. Tasks are not commands and are not eligible for either command phase. Publishing the stage is the only mutation visible to the caller. Any `CapacityExceeded` or `InvalidState` during phases 2 through 5 discards every event and every staged change from that call.

An empty batch is valid. A later empty transition does not rerun a completed task or a consumed command.

C0.2 has no API that skips idle ticks. A grouped driver is a test loop that calls `drive` once per explicit boundary. It is not a second simulator. Fixture 1 tests may keep calling `step`; those calls remain one boundary each.

## 6. Identity allocation

There is one record cursor, `next_record_id`, and no free list. Completion, release, and mode changes do not return IDs. Issued IDs are the IDs present in the location, cargo, reservation, task, and marker tables. They are unique across those tables. The cursor is always strictly greater than every issued ID.

`MintRecord`, in the command phase:

- If the actor is disabled, append `SchedulerCommandOutcome::Rejected(DisabledActor)`, leave the cursor unchanged, and insert no marker.
- If the actor is enabled, let `id = next_record_id`. Checked addition advances the cursor. Overflow returns `CapacityExceeded` for the whole step. Otherwise append a marker `{ id, mode_at_mint }` where `mode_at_mint` is the actor's mode after the mode phase of this same tick, and append `SchedulerCommandOutcome::RecordMinted`. The event key is that command's key.

The approved non-reuse proof is: fixture 2 construction consumes IDs 1 through 9; the first enabled mint returns 10; the second returns 11; IDs 1 through 9 remain occupied by the constructed records.

## 7. Domain checkpoint revision 1

All integers are unsigned little-endian. Tags are exact ASCII with no length prefix and no terminator. Lists begin with a `u64` count. Records use the order below with no padding. Tables are sorted ascending by their stated key. Hashes are raw 32 bytes.

The body is:

| Order | Field | Encoding |
| --- | --- | --- |
| 1 | Tag and revision | ASCII `SH:C02:STATE`, then `u16 = 1` |
| 2 | Fixture identity | `u32` fixture ID, then 32 content-hash bytes |
| 3 | Clock and seed | `u64` tick, `u64` seed |
| 4 | Allocation | `u64 next_actor_id`, `u64 next_record_id` |
| 5 | Actors | Count 2. Each record is `u32 actor_id`, `u8 enabled` (0 or 1), `u8 mode` (1 active, 2 scheduled), ascending actor ID |
| 6 | Slot | `u8 0` vacant, or `u8 1` plus `u32 owner_actor_id` |
| 7 | Pending commands | Count, then command-key order. Each record is `u64 target_tick`, `u32 actor_id`, `u32 producer_id`, `u64 sequence`, then the payload below |
| 8 | RNG ordinals | Count 4, then `(stream_id, actor_id)` ascending. Each record is `u8 stream_id`, `u32 actor_id`, `u64 next_ordinal`, including zeros. The pairs are exactly `(1,1), (1,2), (2,1), (2,2)` |
| 9 | Event prefix | `u64 next_event_id`, then the 32-byte digest |
| 10 | Locations | Count, then `u64 id` ascending |
| 11 | Cargo | Count, then `u64 id`, `u64 owner_location_id` ascending by ID |
| 12 | Reservations | Count, then `u64 id`, `u64 cargo_id`, `u32 actor_id`, `u64 task_id`, `u8 state` ascending by ID. State 1 is active. State 2 is released |
| 13 | Tasks | Count, then `u64 id`, `u32 actor_id`, `u64 due_tick`, `u64 cargo_id`, `u64 reservation_id`, `u64 destination_id`, `u8 state`, `u8 mode_at_completion` ascending by ID. State 1 is pending. State 2 is completed. `mode_at_completion` is 0 while pending, otherwise 1 or 2 |
| 14 | Markers | Count, then `u64 id`, `u8 mode_at_mint` ascending by ID |

Payload discriminants, unchanged for the two C0.1 payloads:

| Payload | Bytes |
| --- | --- |
| `ClaimSlot` | `u8 1` |
| `Probe` | `u8 2`, `u8 stream_id`, `u8 draw_count` |
| `SetMode` | `u8 3`, `u8 mode` (1 or 2) |
| `MintRecord` | `u8 4` |

The blob is `body || SHA-256(body)`. `domain_checkpoint_sha256` hashes the blob, trailer included. The trailer is integrity framing, not an independent gameplay field. The checkpoint does not contain emitted event bytes, rejected ingress diagnostics, the host's historical batch log, wall-clock data, file paths, Bevy identifiers, ChaCha state, or container compression.

### Persistence rules

Every future-affecting field is in the body.

| Field | Rule |
| --- | --- |
| Fixture ID, content hash, seed, enabled flags | Restored verbatim and checked. Flags do not change after construction |
| Tick | Resume boundary. The next submission uses this tick. Fixture 1 may continue with `step` or `drive`. Fixture 2 must continue with `drive`. `step` on fixture 2 returns `UseDrive` and does not mutate |
| `next_actor_id` | Persisted. Remains 3 |
| `next_record_id` | Persisted. Load continues from this value |
| Mode | Persisted per actor. A `SetMode` still pending is persisted as a command, not applied early |
| Slot owner | Persisted. Fixture 2 leaves it vacant |
| Pending commands | Persisted in key order, including commands due on a future tick |
| Four RNG ordinals | Persisted even when zero. Fixture 1 probes continue at the stored ordinal |
| `next_event_id` and digest | Persisted. New events continue the chain. Old events are not emitted again |
| Locations, cargo owners, reservations, tasks, markers | Persisted, including completed tasks and released reservations |
| `mode_at_completion`, `mode_at_mint` | Persisted witnesses of the mode after the mode phase on the commit tick |

## 8. Scheduled work and conservation

A pending task completes only in the work phase of its due tick. Completion is one staged commit:

- The actor's mode for the witness is the mode already written by phase 3 of this tick.
- The linked reservation must be active, the cargo must exist, and the cargo owner must be a location other than the destination.
- Set the cargo owner to the destination, the reservation to released, the task to completed, and `mode_at_completion` to the actor's current mode.
- Emit one `TaskCompleted` event.
- Do not create or destroy a cargo record, a location, or a reservation record. Do not change the record cursor or any RNG ordinal.

The primary fixture therefore keeps exactly two cargo tokens. Before tick 3 both are at location 1 and both reservations are active. At tick 3 each token moves once, to locations 2 and 3 respectively, and each reservation releases once. Later ticks do not move them again.

Disabled actors still complete an already installed synthetic task. That is an intentional C0.2 boundary, not a waiver of Core §12 invariant 4.

Core §12 says each invariant starts at its owning milestone. Invariant 4 is a production job rule. Core §6 places production task lifecycle, cancellation, and failure at C0.3/M0. C03-T01 owns disablement at a completion boundary for real reservations. The C0.2 task is the LOD accounting fixture: two tokens, two reservations, and one exact transfer. LOD §4 requires that a mode transition neither creates nor loses reserved cargo. Completing the due synthetic task preserves that conservation. Refusing it because an actor flag is false would leave the token reserved and would add a job-eligibility rule this fixture does not specify.

`MintRecord`, `ClaimSlot`, and `Probe` are commands. A disabled actor rejects those and does not mutate the slot, the ordinal, or the record cursor. C0.3 replaces this synthetic completion with the production rule that disablement and expiry invalidate a reservation before completion. Until that specification exists, C0.2 does not guess its keys or diagnostics.

No field is approximate. Active and scheduled actors execute the same task transition. Mode is observable state, not a license to skip, coalesce, or estimate the transfer.

## 9. Events

One cursor numbers every published event. It starts at 1. Each publication, of either dialect, takes the current cursor and then advances it. `Event::id`, `SchedulerCommandEvent::id`, and `GeneratedEvent::id` return that value. `next_event_id` after the transition is the next unused value.

Fixture 1 events are only `DomainEvent::Command`. Their bytes and the chain `H0 = SHA-256("SH:C01:EVENT-INIT:v1")`, then `SHA-256("SH:C01:EVENT:v1" || previous || event)`, are unchanged. The approved continuation is event 3, bytes `03000000000000000300000000000000020000000100000001000000000000000202`, resulting digest `1a1cc22bf1041c19845bceb790e6761dad0edaf41cb3eca1f90b0ce06f1875b1`, next cursor 4. `Event::key()` is the admitted claim or probe key. `Event::outcome()` is the C0.1 outcome. `Event::canonical_bytes()` is the C0.1 encoding.

Fixture 2 uses `H0 = SHA-256("SH:C02:EVENT-INIT:v1")` = `413408f703da057f8b39023e580c8c65649376824d1a77be0a4d24aaf048dd0b`, then `SHA-256("SH:C02:EVENT:v1" || previous || canonical_bytes)`. The digest input is the canonical bytes of the `DomainEvent` in emission order. Fixture 2 does not emit `DomainEvent::Command`.

The canonical bytes do not repeat `producer_id` or `sequence`. Those fields stay on the `CommandKey` returned by `SchedulerCommandEvent::key()`. The published mint and rejection vectors therefore stay unchanged. A scheduler command event's bytes are `u64 event_id`, `u64 target_tick`, `u32 actor_id`, then:

| Outcome | Tag | Tail |
| --- | --- | --- |
| `RecordMinted` | 2 | `u64 marker_id`, `u8 mode_at_mint` (1 active, 2 scheduled) |
| `Rejected(DisabledActor)` | 3 | `u8 1` |

A generated event is a task completion. It has a tick and an actor, and it has no command key. Bytes are `u64 event_id`, `u64 tick`, `u32 actor_id`, `u8 1`, `u64 task_id`, `u64 reservation_id`, `u64 cargo_id`, `u64 origin_location_id`, `u64 destination_location_id`.

Mode changes do not emit events. Their effect is the stored mode and, when a later mint or task commits on that same tick, the witness byte. This is a closed C0.2 choice. A later milestone may add a mode event only by revising the event schema.

Within one tick the emission order is scheduler command events in command-key order, then generated events in `(due tick, actor ID, task ID)` order. On fixture 1 the command-phase order is the C0.1 key order and the work phase emits nothing. The phase witness requires a same-tick `SetMode(Scheduled)` and `MintRecord` whose command keys would mint first if they shared one queue. After the phased transition the mint witness is scheduled, and a task due on that same tick records scheduled as well. The mint event precedes the task events.

## 10. Restore and failure

Restore reads a blob as follows. The first matching rule wins. No rule panics, and none allocates from a count that has not been checked against the remaining bytes.

1. Fewer than 14 bytes: `Truncated`. Fourteen bytes is the 12-byte tag plus the `u16` revision.
2. Any other tag: `UnsupportedFormat`. This fires before the checksum. A flipped tag byte of an otherwise valid blob, with the original trailer left in place, is `UnsupportedFormat`. The frozen `SH:C01:STATE` observation takes this path and is not migrated.
3. Revision other than 1: `UnsupportedRevision { found }`. This also fires before the checksum. Changing the revision of a valid blob to 2 and keeping every later byte, including the original trailer, is `UnsupportedRevision { found: 2 }`, not `ChecksumMismatch`. Do not parse further fields.
4. Fewer than 32 bytes after the 14-byte header, or SHA-256 of all bytes except the final 32 does not equal those final 32: `ChecksumMismatch`. A body byte at offset 14 or later, with the original trailer preserved, fails here. An extra trailing byte shifts the trailer window and fails here.
5. Bounded parse of the body, below. An unknown discriminant, a count that does not fit, a leftover body byte, or an overflow while measuring records: `Corrupt`.
6. Fixture ID other than 1 or 2: `UnknownFixture { found }`.
7. Fixture ID 1 or 2 whose content hash is not that fixture's canonical hash: `ContentMismatch`.
8. Semantic rules, in the numbered order below. The first failure is `InvalidState`.

### Bounded parse

The decoder treats the body as untrusted. For every list it reads the `u64` count, then accepts the count only when all of the following hold:

- `count` multiplied by that record's minimum byte size does not overflow `u64`.
- That product is less than or equal to the number of body bytes still unread.
- The count converts to `usize` with a checked conversion.

If any check fails, restore returns `Corrupt` and allocates nothing for that list. In particular, a checksum-valid body whose actor count, pending-command count, or cargo count is `u64::MAX` returns `Corrupt`. A count of zero allocates an empty list. After a count is accepted, the capacity of the list is that count, which is bounded by the body length because every record is at least one byte. Unknown discriminants and unconsumed body bytes are `Corrupt`. Enabled bytes other than 0 or 1, mode bytes other than 1 or 2, and unknown payload or state tags are `Corrupt` at this step.

### Semantic order

These rules apply only after the checksum and the parse succeed. They reject states that the fixtures cannot reach, and they accept every state the approved transitions can reach, including the published golden blobs.

Shared rules, in order:

1. `next_event_id` is 0.
2. `next_actor_id` is not 3.
3. The actor table is not exactly actors 1 and 2 in that order.
4. A table is not strictly sorted, or it contains a duplicate key.
5. A pending command targets a tick at or before the checkpoint tick, names an actor other than 1 or 2, or has producer 0.
6. The ordinal section is not exactly the pairs `(1,1), (1,2), (2,1), (2,2)`.
7. Any record ID is 0, any ID is repeated across the five record tables, or `next_record_id` is not strictly greater than every issued ID.

Fixture 1 then applies:

1. Either actor's mode is not active.
2. Any location, cargo, reservation, task, or marker is present, or `next_record_id` is not 1.
3. A pending payload is `SetMode` or `MintRecord`.
4. A pending probe has a stream other than 1 or 2, or a draw count other than 1 or 2.
5. A slot owner is present and is not actor 1 or 2, or the owner actor is disabled.

Fixture 2 then applies:

1. The slot is occupied.
2. Any RNG ordinal is not 0.
3. A pending payload is `ClaimSlot` or `Probe`.
4. `next_record_id` is less than 10, or the issued IDs are not exactly the integers from 1 through `next_record_id - 1` inclusive.
5. The records are not exactly locations 1, 2, and 3; cargo 4 and 5; tasks 6 and 7; reservations 8 and 9; and marker IDs from 10 through `next_record_id - 1` inclusive, with no gaps. When the cursor is 10, the marker table is empty.
6. The task, reservation, and cargo links differ from the construction table in §3. Destinations stay 2 and 3. Reservation 8 stays on task 6 and cargo 4. Reservation 9 stays on task 7 and cargo 5.
7. The two tasks have different due ticks.
8. One task is pending and the other is completed.
9. A pending task has `mode_at_completion` other than 0, a due tick at or before the checkpoint tick, a reservation that is not active, or a cargo owner other than location 1.
10. A completed task has `mode_at_completion` outside 1 and 2, a due tick greater than the checkpoint tick, a reservation that is not released, or a cargo owner other than that task's destination.

Every published C0.2 golden blob satisfies these rules and restores to equal domain bytes. A legal non-zero fixture 1 ordinal, a legal fixture 1 slot owner who is enabled, a fixture 2 pair of pending tasks, a fixture 2 pair of completed tasks, and fixture 2 markers issued by mint all remain acceptable. The event digest is not recomputed on load: the blob does not contain the event log. The trailer detects accidental corruption. Agreement with a reference run detects a substituted digest.

Restore does not repair, default, or drop unknown fields. There is no C0.2 migrator.

`drive` returns `InvalidState` with full rollback if the work phase observes a pending task whose due tick is already behind the staged tick. Load rejection is the supported external way to produce that input. The in-crate tests may also assign such a due tick directly, as the C0.1 overflow tests assign private cursors, and must observe the same rollback.

Checked `u64` overflow of the tick, event cursor, probe ordinal, or record cursor remains `CapacityExceeded` with full rollback.

## 11. Replay

A resume at tick N loads the domain blob, checks it, and continues with an explicit batch for boundary N. Fixture 1 submits that batch through `step` or `drive`. Fixture 2 submits it through `drive`. Calling `step` on a restored fixture 2 simulation returns `UseDrive` and leaves the loaded bytes unchanged. The restore format and the tick phases are the same either way. Commands admitted before the save and due after it are already in the blob. The host must not replay them from an external log unless they are absent from the blob. The kernel does not read a historical command log on restore.

The test driver keeps emitted event bytes outside the blob. Comparison for a saved prefix is: loaded domain bytes equal the uninterrupted bytes at the split; the saved event prefix is unchanged; the next emitted event uses the stored cursor; the concatenated prefix and suffix equal the uninterrupted stream; the final domain bytes, cursor, and digest match.

Fixture 1 RNG replay continues a probe at the stored `(stream, actor)` ordinal. The generator, domain separator `SH:C01:RNG:v1`, and one-ordinal-per-successful-probe rule are unchanged. Fixture 2 does not draw.

R1 is the requirement: same pinned build, platform, fixture bytes, and boundary inputs. R2 cross-platform certification remains deferred. The little-endian layout is the candidate encoding for a later R2 test, not evidence that R2 has passed.

## 12. Compatibility boundary

| Input | Result |
| --- | --- |
| Domain revision 1 with a valid checksum and valid state | Restored simulation |
| `SH:C01:STATE` revision 1, including every published C0.1 vector | `UnsupportedFormat` |
| `SH:C02:STATE` revision other than 1 | `UnsupportedRevision` |
| Torn, empty, flipped, or trailing-byte blob | `Truncated` or `ChecksumMismatch`, never a partial world |
| Checksum-valid body with a bad discriminant or leftover byte | `Corrupt` |
| Checksum-valid known fixture with the wrong content hash | `ContentMismatch` |
| Checksum-valid body that breaks an invariant, including overdue work | `InvalidState` |

Revision 1 has no predecessor and no successor. M0-T11 is the owner of the first real migration. That migration will be a new revision with an explicit transform, retained IDs, and tested defaults. A revision-1 reader must keep rejecting every other revision. Adding `ticks_per_game_day`, event-log bytes, or a mode-change event requires that new revision. C0.2 does not reserve silent defaulting as a compatibility strategy.

`checkpoint_bytes()` remains available so existing C0.1 tests, and the fixture 1 save state in C02-T01, still have the frozen observation. Those bytes are not a save file.

## 13. Dependencies and layout

Pinned toolchain and crates stay Rust 1.100.0, `rand_chacha = "=0.10.0"` with default features disabled, and `sha2 = "=0.11.1"`. The committed lockfile stays authoritative. Implementation adds no direct dependency.

New modules inside `sealed_sim` are an implementation choice. Suggested names are `persist` and `scheduler`. They must not change the bytes produced by `encoding::checkpoint_bytes` or `encoding::event_bytes` for C0.1 outcomes.

## 14. Decisions closed by this specification

These were underspecified in G0. They are normative if this document is approved.

| Decision | Choice | Rejected for C0.2 |
| --- | --- | --- |
| Save envelope | `SH:C02:STATE` revision 1 plus a SHA-256 trailer | Reusing `SH:C01:STATE`; serde or JSON |
| C0.1 observations | Remain readable through `checkpoint_bytes` and unloadable as saves | Silent upgrade into revision 1 |
| Migration | None in C0.2. M0-T11 owns the first migrator | Inventing a synthetic revision 0 or defaulting new fields |
| Fixtures | Slot and scheduler share one schema and reject each other's payloads | One combined payload set and one event dialect |
| Mode input | Admitted `SetMode` commands | Camera, visibility, or a hidden policy |
| Phase order | Tick advance, mode, commands, tasks | One queue ordered only by command key |
| Mode events | No consequential event. Witness bytes record the mode that was already applied | Leaving same-tick order unobservable |
| Record IDs | One monotonic cursor, no free list, actors fixed at 1 and 2 | Slotmap reuse or a single ID space that allocates actors |
| Cargo | Two durable tokens, one owner each, reservation is not a second owner | Counts without identities, or approximate transfer |
| Idle time | Repeated `step` only | A skip API, even behind a flag |
| Event history | Cursor and digest in the blob; bytes retained by the driver | Embedding the event log or requiring it to resume |
| Scheduler event API | `Event` stays the C0.1 command event. Mints use `SchedulerCommandEvent` and their real command key. Tasks use `GeneratedEvent` and have no key. Fixture 2 callers use `drive` | Putting task completions in `Event` by inventing a producer and sequence |

## 15. Out of scope

Production reservation expiry, recipe rollback, and fairness keys remain C03-T01. Needs, jobs, factions, relationships, navigation, world generation, cultivation, and client projection remain at their G0 gates. Aggregate cohorts remain deferred under ADR-010. The production rule for when an actor becomes scheduled without an explicit command remains open until M0-T05 and an ADR-009 amendment. Coarse travel approximation remains forbidden until that same evidence exists. Host cadence and cross-platform R2 certification are not C0.2 exit criteria.

`ticks_per_game_day` is not a C0.2 field. Core §3 and ADR-024 introduce it for scenarios that measure days. The C0.2 fixtures schedule work at absolute ticks 2, 3, and 4 and have no day horizon, so the parameter would not change any accepted result. Putting it in revision 1 and defaulting it would invent a migration. The first scenario that measures days adds it in a later revision under M0-T11.

The synthetic disabled-task rule in §8 is the same kind of boundary. It is explicit so C0.2 does not silently adopt, or silently contradict, the C0.3 job rule.

## 16. Implementation sequence after approval

1. Add the domain encoder and decoder. Keep every existing C0.1 test green without editing its golden literals.
2. Implement fixture 1 restore, the approved tick-2 resume, probe-ordinal resume, the checksum precedence cases, the bounded-count cases, and the fixture-specific rejection matrix.
3. Implement the record cursor and the mint continuity scenario.
4. Implement fixture 2 construction, the three phases, and the tick 2/3/4 oracle.
5. Implement the same-tick witness, disabled mint, overdue rollback, and cursor overflow rollback.
6. Run the locked format, lint, and test commands. Do not regenerate C0.1 vectors to match a new encoder.
