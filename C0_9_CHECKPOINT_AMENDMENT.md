# C0.9 checkpoint amendment — local world memory

**Status:** Implemented locally on `feat/c09-world-memory`, pending Rust execution, owner acceptance and independent review. Do not call this a passed milestone.

## Contract

`SH:C05:STATE` revision 1 remains the settlement payload. Revision 2 remains the C0.8 hostile encounter payload; neither encoder nor content hash is changed. New social-memory state is offered only by `World::conflict_with_memory()`, revision **3**, using a distinct content hash seeded with the exact revision-2 hash.

All integer fields are little-endian and the outer SHA-256 covers every preceding byte. Revision 3 writes the identical common body and the identical revision-2 combat tail first, then:

| Field | Type | Meaning |
| --- | --- | --- |
| incident-present | `u8` boolean | Strict 0/1 |
| death-event ID | `u64`, conditional | Authored by player 21 against foe 22; nonzero |
| death tick | `u64`, conditional | Original witnessed fatal hit tick |
| witness ID | `u32`, conditional | Must be NPC 2 |
| report tick | `u64`, conditional | Must equal death tick + 2 |
| reported | `u8` boolean | Strict 0/1 |
| healer standing | `i8` | Exactly 0 when unreported, -1 when reported |
| outer digest | `[u8;32]` | SHA-256 of all preceding bytes |

`Incident` and the report marker are authoritative; the two memory events are recomputed during complete replay and included in the canonical event digest and event ID highwater. A valid but re-signed payload must still match exact replay. No new save sidecar is added. Existing session loads remain two-file operations with the inherited C0.7 crash-consistency limitation.

## Phase order and event identity

1. Frozen C0.3 NPC phase commits one tick.
2. The admitted player action resolves; combat may emit Hit and Death with shared C0.5 event cursors.
3. Existing hostile response resolves.
4. C0.9 examines only a player-caused foe-22 Death **from this tick**. If NPC 2 has a known position at Manhattan distance <= 4 from the death tile, it records one incident, then emits one `WitnessedDeath` memory event.
5. Exactly two canonical ticks later, it changes healer standing to -1 and emits one `ReportDelivered` memory event. Empty world ticks count; UI-only commands and real elapsed time do not.

The report is not visible to the player while pending. Only after it is reported does the ASCII panel reveal the witness and causal death event. A death without a qualifying witness creates no incident and never changes healer standing. No global rumor path, line-of-sight raycasting or second hostile actor is claimed.

## Compatibility tests

- Historical C0.7 revision-1 fixture and C0.8 revision-2 histories must round-trip unchanged.
- Revision-3 save/load at the pending and reported stages must exactly match uninterrupted replay and rebuild both memory events only once.
- Re-signed revision-3 payloads with contradictory standing/report fields must be rejected.
- Source files implementing the frozen C0.1/C0.3 kernel remain unchanged.

**Limit:** The 4,096-tick horizon remains unchanged; a report scheduled beyond that limit remains pending. Persistent World & Legacy successor control is not implemented here.
