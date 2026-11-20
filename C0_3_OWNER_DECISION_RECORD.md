# C0.3 Owner Decision Record

Version 2.2 · 18 November 2026
Status: recorded. H09 retains its original limited authorization. RT-01 below authorizes the targeted cancellation-timing documentation and required regression tests, their verification, and commit/push to `feat/c03-s4-integration` only. It does not authorize a merge, C0.4, Bevy, dependency/toolchain changes, or an independent C0.3 PASS.

## 1. Verification

### OD-01 through OD-16

The project owner approved OD-01 through OD-16 as written in `C0_3_OWNER_DECISIONS_PENDING.md`, version **1.2-proposal**, dated 17 November 2026. That approval remains in force.

### AM-01 through AM-08

The project owner approved AM-01 through AM-08 as documented in the C0.3 G3 R2.1-final remediation packet:

- `C0_3_G3_R2_OWNER_AMENDMENTS_PENDING.md` version R2.1-final
- `C0_3_G3_R2_REMEDIATION_PROPOSAL.md` version R2.1-final
- `C0_3_G3_R2_ACCEPTANCE_RECONCILIATION.md` version R2.1-final

**OC-EDGE is REJECTED.** The approved fixture graph remains sites 1, 2, and 9 with edges 1 and 2 only.

The normative copy is `docs/C0_3_IMPLEMENTATION_SPEC.md` together with `docs/C0_3_ACCEPTANCE_TESTS.md`.

## 2. Binding conditions

The owner attached these conditions. They bind the specification.

1. C0.1 and C0.2 canonical behavior and golden vectors remain frozen.
2. The creation-tick work restriction remains in force.
3. Completion replay is a no-op only for an identical `CompleteTask` command identity, including the original submitted task id.
4. Reuse of a durable command identity with a different command type or payload returns `ConflictingIdentity`. For fixture 3, durable identities are the 41-byte consumed completion registry and pending commands. The H09 amendment in §2A supersedes the former requirement to durably recognize every historically applied non-completion key.
5. Reservation accounting, transaction rollback, and completion provenance follow the approved R2.1-final contracts.
6. Residual edge movement preserves travel cost and remains deterministic across save and restore.
7. OC-EDGE is rejected. The approved graph remains unchanged.
8. The full C0.3 acceptance scope remains mandatory, including C03-T01 and M0-T06 through M0-T08.
9. Bevy remains outside the `sealed_sim` kernel.
10. The Balanced Development Strategy remains approved: S1 Resource Core, S2 First Living NPC, S3 Movement and Production, S4 Multi-NPC Integration. An internal slice is not the C0.3 milestone exit.

Implementation requires a later, separate authorization. An independent G3 review of the revised documents is mandatory before that authorization. This record does not start S1, does not declare G3 PASS, and does not begin Bevy integration.

### H09 amendment — fixture 3 (17 November 2026)

The project owner approved **H09 Option B**. This amendment is normative for C0.3 fixture 3 and supersedes the conflicting portion of §2 condition 4. The superseded text required durable recognition of every historically applied non-completion command key. The deterministic replay contract below replaces that requirement.

1. Consumed `CompleteTask` identities remain durable through the existing 41-byte completion registry.
2. Pending command identities retain their existing conflict rules.
3. Duplicate or conflicting commands within one admission are validated according to the normative first-match precedence table.
4. Ordinary non-completion commands that have already been applied do not require a separate persistent historical registry.
5. After such a command has been applied and is no longer present in canonical state, a later submission with an old target follows the deterministic `LateTarget` rule, subject to earlier applicable structural validation.
6. Save and restore must produce identical future behavior from identical canonical checkpoint bytes.
7. No hidden noncanonical replay cache is permitted.
8. `SH:C03:STATE` revision 1, the 41-byte completion registry, and every published canonical vector are preserved.

Consequences for fixture 3:

- A durable identity is either a consumed completion registry record or a pending command. Those identities keep their conflict rules.
- An applied non-completion key that has left canonical state has no persistent identity. A later submission with an old target is `LateTarget` after the earlier structural rows. A later submission with a future target is an ordinary new command, not a historical replay.
- Identical canonical checkpoint bytes produce identical future behavior; no historical distinction is reconstructed outside the body.

This amendment authorizes only the minimum necessary normative documentation, acceptance-test, and Rust changes to implement H09. It does not authorize S2, S3, S4, or Bevy, does not permit a push or merge, and does not declare an independent S1 PASS.

### RT-01 owner amendment — next-boundary residual movement after cancellation (18 November 2026)

The owner explicitly approved **Option B — next-boundary residual movement** against audited commit `1dbe07e632efa55596cb33e17d6470afa0e0824c`. This supersedes only the conflicting same-boundary cancellation reading of implementation specification §8. Acceptance §14's post-cancel save and next-boundary continuation remain approved.

1. Let `T` be the staged tick of a successful `drive` whose invalidation phase actually cancels the current task (interruption reason 2 under the existing reason precedence). Clear `task_id` and perform the existing cancellation transitions/releases/provenance handling without changing them.
2. Preserve the actor's mid-edge locus, edge id and `traversed_cost`. Clearing the task through that cancellation does not make the actor perform residual movement in tick `T`.
3. Starting with the next successful boundary, staged tick `T+1`, the existing residual work rule may spend one edge-cost unit per tick, after task-owned work, in the existing actor-id order. Remaining edge cost is never discarded. Disabled actors still perform residual travel.
4. Arrival places the actor at the correct destination site, clears `edge_id` and resets `traversed_cost` to 0. Residual progress/arrival emit no event; other approved events remain unchanged.
5. Save/restore at tick `T` preserves the exact on-edge/taskless state and produces identical continuation at `T+1`, without an extra wait, repeated prefix events or hidden readiness state. A failed boundary does not advance the tick or spend cost; a successful boundary advances exactly one tick under the existing `drive` rules.
6. This is a cancellation-only exception. It does not change reason precedence, unrelated task transitions, completion, navigation/path ordering, or other residual-work cases, including other causes of task-id clearing. It does not alter AM-08's cost preservation, the approved graph, checkpoint revision 1, event schema, or any frozen C0.1/C0.2/C0.3 vector.

No conflicting binding condition was found: OD-14 still decides the interruption reason before this exception applies; OD-10 and AM-01 retain creation-tick/one-transition rules; OD-11 and AM-08 retain exact travel cost and position; the save/replay and frozen-vector conditions remain in force. This amendment resolves the authority question identified as F01 in the final independent audit; independent confirmation of closure remains required.

Authorization: record this decision, reconcile §8 and the affected acceptance wording, add focused tests, verify the five required Cargo commands, and commit/push the targeted change to `feat/c03-s4-integration`. Do not modify production Rust unnecessarily. Request a targeted independent re-review; do not declare independent milestone PASS.

## 3. Approved OD contracts

| ID | Approved contract |
| --- | --- |
| OD-01 | Fixture 3. `SH:C03:STATE` revision 1. One non-terminal task per actor. Position, path, reservation list, planned legs, completion provenance, traces, and `BeginTask` are in the body. No migration of `SH:C02:STATE` revision 1 |
| OD-02 | Digest domain `SH:C03:EVENT-INIT:v1` then `SH:C03:EVENT:v1`. Kinds 1 through 7. One event order. Kind 7 on every stage change |
| OD-03 | Fixture-local command key `(target tick, actor ID, producer ID, sequence)`. All-or-nothing reservations. This is not global gameplay fairness |
| OD-04 | A task holds zero or more reservations. A bundle accepts all pairs or none. Class 1 is consumed on completion through planned legs. Class 2 is released and the quantity stays |
| OD-05 | Transaction id allocated only after every leg validates. Credit `stock_id` 0 allocates `next_stock_id`. A failed later leg rolls the boundary back. Source 1 is an admitted `CompleteTask`. Source 2 stores producer 0 as provenance only |
| OD-06 | Producer 1 only. A non-zero cancel must name the claimant and the current task. Reservation id 0 cancels the current task, including Rest and Return Home |
| OD-07 | Producer 1 may set the subject enabled bit to 0 or 1. Fixture 1 and fixture 2 flags stay immutable |
| OD-08 | Repeated release and a matching consumed completion replay emit nothing and do not advance the cursor or digest. A valid boundary may still advance the tick |
| OD-09 | Decision phase, ascending actor id, checked `i64` dot product, no GOAP. Fixture wake interval 4. Fixture weights are diagonal 1000. Relationship term 0 and minimums at the minimum `i64` are the neutral fixture gate |
| OD-10 | One work transition per eligible tick. Creation-tick skip remains. `CompleteTask` records provenance in the command phase and does not apply legs. Legs run in work only if invalidation left the task `Completing` |
| OD-11 | Actor position is authoritative. Graph search and grid search are separate. Edge cost is both path weight and travel duration. Fixture graph is sites 1, 2, and 9 with edges 1 and 2. Fixture grid is 3 by 3 with a blocked center. No pathfinding crate |
| OD-12 | No automatic mode policy. `SetMode` from producer 1 is the only mode input. Active and scheduled actors use the same production rules |
| OD-13 | M0-T06 stays. Twenty actors, one current task each, one bundle, supplies of 5 for beds, tools, food, and workstations. The one-unit case stays separate |
| OD-14 | One reason. Order is disablement, cancellation, then expiry. One release of each active reservation. No legs |
| OD-15 | Fixture 3 phases: mode, command, invalidation, decision, work. Fixtures 1 and 2 do not enter production invalidation, decision, or production work. Fixture 2 keeps its synthetic work phase |
| OD-16 | Fixture 2 still completes a due synthetic task while its actor is disabled. Published C0.2 hashes stay |

## 4. Approved AM contracts

| ID | Approved contract |
| --- | --- |
| AM-01 | `created_tick` is the staged tick of creation. Work skips that tick. M0-T07 applies each hop on the following tick. Scripted tasks carry `due_tick` in the command. Task 405 fails on its later leg after a successful first leg against stock 14 |
| AM-02 | Every accepted completion stores the submitted payload id and the resolved task id. The lowest key is the only source-1 transaction. Other keys remain in the consumed registry. Identical submitted replay is a no-op |
| AM-03 | Legs carry definition and class. Holders validate container, cargo, and reserve owners. `recipe_id` binds a task to one scenario recipe. Available quantity excludes other tasks' active reservations. Own class-1 reservations match the debit and are released once. A later-leg failure rolls the whole boundary back |
| AM-04 | Tasks and reservations share `next_record_id` and cannot reuse an id across those tables. C03-T01a starts at 3. M0-T06 starts at 21. M0-T07 starts at 401 |
| AM-05 | Fixture-3 first-match ingress. A consumed completion key replays as a no-op only when the submitted payload id matches. A different submitted id is `ConflictingIdentity`. Terminal cancel uses the stored claimant. Producer 0 is not a command |
| AM-06 | The five goal plans, the reason order, and same-tick enable rescheduling are fixture rules. Weights and the wake interval stay the approved fixture configuration |
| AM-07 | `Queued` may be overdue without failing the boundary. Other non-terminal stages may not. Creation-tick skip is first. Same-boundary work events are kept. M0-T07 tick 10 is a later-leg `InvalidLeg` |
| AM-08 | Mid-edge progress is preserved. After `task_id` is cleared, residual work-phase travel spends the remaining edge cost one tick at a time until the destination site. Cancellation timing is qualified by RT-01 above. The approved graph is unchanged. OC-EDGE is rejected |

## 5. Fixture configuration

These numbers are approved for the C0.3 fixtures. They are not a balance pass for the shipped game.

| Item | Fixture value |
| --- | --- |
| Wake interval | 4 ticks |
| Goal weights | Eat `(1000,0,0,0,0)`, rest `(0,1000,0,0,0)`, work `(0,0,1000,0,0)`, haul `(0,0,0,1000,0)`, return home `(0,0,0,0,1000)` |
| Role bonuses | 0 |
| Relationship term | 0 |
| Relationship minimums | the minimum `i64` value, `-9223372036854775808`, for every goal |
| Route | Sites 1 and 2, edges `(1,2,1,cost 1)` and `(2,1,2,cost 1)`, site 9 with no edge. No edge 4. No edge 5 |
| Grid | Site 1, width 3, height 3, center cell blocked |
| M0-T06 supplies | 5 beds, 5 tools, 5 food, 5 workstations |

## 6. What this record does not do

It does not modify Rust source, `Cargo.lock`, or a C0.1 or C0.2 golden vector. It does not start S1 or C0.4. It does not admit Bevy into `sealed_sim`. It does not declare G3 PASS. It does not replace the mandatory independent review of the revised specification.
