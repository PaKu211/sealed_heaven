# C0.1 Implementation Specification

Version 0.2-draft · 17 November 2026

This is the normative C0.1 scope and byte contract. [Acceptance tests](C0_1_ACCEPTANCE_TESTS.md) define expected results. Independent R4 review and project-owner authorization precede implementation.

## Scope and interface

One pure Rust library package, `sealed_sim`, with in-package tests. The interface accepts a typed fixture configuration and explicit step inputs. It exposes read-only logical state, canonical checkpoint bytes and SHA-256, and each step's consequential events or a typed error. Function names and storage representation are implementation choices.

- The fixture has ID 1, actors 1 and 2 with caller-provided enabled flags, one implicit exclusive slot initially vacant, and a caller-provided `u64` seed. These two stable actor IDs are assigned by the fixture. Actor flags remain fixed for a run. The fixture content identity is SHA-256 of the literal ASCII bytes `SH:C01:FIXTURE:v1` followed by byte `0a`.
- Initial state has tick 0, no pending commands, four zero RNG ordinals, next event ID 1 and the initial event digest documented below.
- C0.1 covers integer ticks, immutable command admission, one phase of ordered commits, one synthetic slot, RNG probes, and replay evidence. It has no Bevy subcrates, renderer, windowing, GPU, runtime LLM, networking or OS-seeded gameplay randomness. World generation, production inventory/reservations, NPC systems, pathfinding, content loading, persistent ID allocation, save/load, LOD and client adapters enter at later milestones. Canonical bytes are an observable test checkpoint. C0.1 neither writes nor reads persistent snapshots.
- No standalone runner, generalized scheduler, ECS, plugin or extension API is required. Grouped replay is a test driver that composes explicit steps.

## Command input and tick boundary

A step input consists of `submission_tick: u64` and one immutable batch, including an empty batch when there are no submissions. All submissions for one boundary are collected into this batch. To step from canonical tick N, the submission tick must equal N. Every submitted command must target a tick greater than N. Admission and advancement belong to one logical step. A rejected attempt leaves N available for a corrected batch. A paused host invokes zero steps. UI input still held by a host has yet to enter the canonical simulation.

| Command field | Type | Rule |
| --- | --- | --- |
| Target tick | `u64` | Strictly greater than submission tick |
| Actor ID | `u32` | An actor in the fixture |
| Producer ID | `u32` | Positive, fixture-assigned |
| Sequence | `u64` | Caller-assigned within the command key, including zero |
| Payload | Tagged | `ClaimSlot` has no argument. `Probe` has stream 1 or 2 and draw count 1 or 2 |

1. **Preflight is read-only.** Check submission tick first. Then validate records in ascending command-key order. Report the lowest invalid key and, for that key, the first applicable diagnostic in this precedence: `LateTarget`, `UnknownActor`, `InvalidProducer`, `InvalidPayload`. A mismatched submission tick is `WrongBoundary` without a command key. Typed inputs may make some malformed payloads unrepresentable. In that case validate that the input type excludes them. Check no slot ownership or enabled flag during preflight.
2. **Admission is batch-atomic.** The identity is `(target_tick, actor_id, producer_id, sequence)` in numeric ascending order. After field checks, compare new records with one another and with canonical pending records. Identical identity and payload collapse to one record. Identical identity with different payloads rejects the entire batch as `ConflictingIdentity`, with the lowest conflicting key. This rule also handles a new batch conflicting with work admitted on an earlier boundary. A consumed command has a closed target tick and is covered by `LateTarget`. No lifetime deduplication table is needed.
3. **Commit is serial.** Merge admitted inputs into the canonical queue. Advance to N+1 and drain only commands due at that tick, by the same ascending key. Before each action, revalidate against the state left by earlier actions. Actor ID comparison is a C0.1 fixture policy. Gameplay priority and fairness need their own later specification and tests. There is no claim of fairness from this ordering.
4. **Apply a complete outcome atomically.** If the actor is disabled, emit `Rejected(DisabledActor)` for either action. An enabled `ClaimSlot` claims a vacant slot and emits `Claimed`; otherwise it emits `Rejected(OccupiedSlot)`, including when the same actor already holds it. An enabled `Probe` consumes one decision ordinal in its actor/stream pair and emits `Probed` with all requested words. An ineligible action updates neither the slot nor RNG. Each due command emits exactly one event with current tick, current cursor as its ID, and its key's actor, producer and sequence. Advance the cursor and digest for each publication. Future commands remain pending.
5. **Bounds failures are atomic.** If an admitted, otherwise valid step cannot advance the tick, event cursor or an enabled probe's ordinal without `u64` overflow, return `CapacityExceeded`. No state, queue, cursor, digest or events from that step become visible. Ingress diagnostics are returned separately and do not enter the consequential-event chain.

## Canonical encoding revision 1

All integers are unsigned and fixed-width, little-endian. `u8` occupies one byte. Tags are exact case-sensitive ASCII with no terminator or length prefix. Hashes are raw 32 bytes. Lists carry a `u64` count. All records use the listed field order with no alignment, padding, or framing bytes beyond the specified discriminants. Hex in the acceptance document is a display format.

| Order | Checkpoint field | Encoding |
| --- | --- | --- |
| 1 | Domain tag and revision | ASCII `SH:C01:STATE`, then `u16 = 1` |
| 2 | Fixture/content identity | `u32 = 1`, then 32 content-hash bytes |
| 3 | Logical tick and world seed | `u64 tick`, then `u64 seed` |
| 4 | Actor table | Count 2, then records by ascending ID. Each is `u32 actor_id, u8 enabled`, with 0 false and 1 true |
| 5 | Slot state | `u8 0` vacant, or `u8 1` followed by `u32 owner_actor_id` |
| 6 | Pending admitted commands | Count, then records sorted by the numeric command key. A record is `u64 target_tick, u32 actor_id, u32 producer_id, u64 sequence`, and payload below |
| 7 | Next decision ordinals | Count 4, then records by ascending `(stream_id, actor_id)`. Each is `u8 stream_id, u32 actor_id, u64 next_ordinal`. Encode zero ordinals too |
| 8 | Event prefix | `u64 next_event_id`, then 32-byte consequential-event digest |

Command payloads: `u8 1` for `ClaimSlot`, or `u8 2, u8 stream_id, u8 draw_count` for `Probe`. The checkpoint checksum is SHA-256 of the entire record above. The future-state inventory is exhaustive for this fixture. No queue-order artifact, replay log, emitted event buffer, ingress diagnostic, wall-clock data, render data or RNG generator internals enter the checkpoint. The event prefix contributes through its cursor and digest.

## Events and named RNG streams

Event bytes begin with `u64 event_id, u64 tick, u32 actor_id, u32 producer_id, u64 sequence`, then an outcome:

| Outcome | Encoding |
| --- | --- |
| Claimed | `u8 1` |
| Rejected | `u8 2, u8 reason`. Reason 1 is disabled actor, reason 2 is occupied slot |
| Probed | `u8 3, u8 stream_id, u8 draw_count`, then that number of `u32` words in draw order |

Use `H0 = SHA-256(ASCII "SH:C01:EVENT-INIT:v1")`. After event bytes E, replace H with `SHA-256(ASCII "SH:C01:EVENT:v1" ∥ H ∥ E)`. The next event ID starts at 1. A step with zero events leaves H and cursor unchanged. An event log is compared in emission order separately from the checkpoint digest.

For a probe, derive the 32-byte key as `SHA-256(ASCII "SH:C01:RNG:v1" ∥ LE64(seed) ∥ U8(stream_id) ∥ LE32(actor_id) ∥ LE64(ordinal))`. The generator uses Bernstein ChaCha12 with that key, 64-bit block counter 0 and 64-bit stream/nonce 0. Read the first one or two 32-bit words of block 0 in little-endian word order. Increment the named `(stream_id, actor_id)` ordinal once per successful probe regardless of draw count. Other ordinals remain unchanged. No unit conversion, probability distribution or global stream is involved. [ChaCha reference](https://cr.yp.to/chacha/chacha-20080128.pdf) · [Published ChaCha12Rng behavior](https://docs.rs/rand_chacha/latest/rand_chacha/struct.ChaCha12Rng.html)

## Replay and dependencies

Replay starts from identical fixture configuration, then consumes the same explicit boundary-indexed successful batches, including empty batches, on the same pinned build/platform. Rejecting an attempted batch cannot change replay state. An equivalent replay can permute records within each batch or group step invocations without changing their boundaries. Compare checkpoint bytes, SHA-256 and all ordered event bytes. C0.2 defines external checkpoint I/O and equivalence across a loaded prefix. Checkpoint emission in C0.1 is on request.

Provisional build candidates are Rust 1.100.0, `rand_chacha = "=0.10.0"` and `sha2 = "=0.11.1"`. The deterministic algorithm and vectors are normative for fixture revision 1. Implementers verify dependency versions, lock resolution and vectors at C0.1 using C01-T04/T06. Keep a committed lockfile and an exact toolchain once selected. Bevy 0.20.1 is the accepted later client baseline under ADR-001. [Rust release](https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/) · [rand_chacha](https://docs.rs/crate/rand_chacha/0.10.0) · [sha2](https://docs.rs/sha2/0.11.1/sha2/) · [Bevy release](https://github.com/bevyengine/bevy/releases/tag/v0.20.1)
