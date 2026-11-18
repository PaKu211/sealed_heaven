# C0.5 executable acceptance requirements

Base `f35bfd3664d4baf2feaf78755b1a18d29cccf795`. These oracles precede implementation
results; changing an oracle to hide a defect is forbidden.

| ID | Required evidence |
| --- | --- |
| C05-T01 | Persistent player ID21, canonical position and inventory; NPC IDs distinct; no Bevy ownership of gameplay fields. |
| C05-T02 | Cardinal moves change exactly one cell; bounds, wall, object and live NPC occupancy reject without changing position. Invalid batches roll back NPC+player together. |
| C05-T03 | Talk/gather/deliver beyond Manhattan 1 reject; unknown NPC/not requester reject; context UI uses live target data and reports failure. |
| C05-T04 | Valid talk then nearby gather debits source1 and credits player1; delivery debits player1, credits shrine1, completes once; sum remains3; insufficient/overflow/duplicate transfers never partially publish. |
| C05-T05 | Same key/payload retry causes no repeated action/event; conflicting identity rejects atomically; sequence exhaustion and bounded queues/registry are tested; retries remain safe after load. |
| C05-T06 | Exact save/load roundtrip and future suffix equivalence at before/after request, gather and delivery splits; malformed/truncated/unknown-revision/bad-checksum/semantic-invalid saves preserve live state and old file. |
| C05-T07 | Repeated stamped command replay matches checkpoint and events at every tick; frame partition tests compare equivalent boundary-stamped input; NPC outputs match unmodified autonomous simulation with empty player commands. |
| C05-T08 | Frozen source implementation/vector/dependency checks pass; old 82 tests+2 doctests and all 26 C0.4 tests still pass in debug/release, plus new tests; fmt/check/strict Clippy and canonical packers pass. |
| C05-T09 | Actual native gameplay: explore, mouse/keyboard target selection, request, herb acquisition and delivery, inventory/status feedback, rejected action, pause/step, camera follow, en/id, save/load and continuing NPCs. Game-only captures/action log; no headless substitution. |
| C05-T10 | Reproducible simulation tick-duration/throughput, CPU/memory, rendering-frame-time/entity-count characterization; 20/100/1000 only where genuine fixtures supported. Explain 2 Hz pacing versus C0.4's 1.44 observed TPS and compute cost. |
| C05-T11 | Separate critical review, reproducible findings/regressions/remediation and independent final acceptance. A review authoring fixes cannot self-label independent PASS. |

Status is recorded in verification/gameplay/performance reports and handoff. A
compiling executable, partial native run or source-only review does not close T09
or independent final acceptance. Owner-requested content remains bounded to herbs.
