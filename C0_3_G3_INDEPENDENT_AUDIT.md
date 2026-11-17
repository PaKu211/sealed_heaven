# SEALED HEAVEN — C0.3 INDEPENDENT G3 FINAL SPECIFICATION GATE

Date: 17 November 2026  
Role: Independent Principal Rust Simulation Architect and Deterministic Systems Auditor  
Reviewed specification: `1.0-approved`; approval source: `1.2-proposal`  
**Verdict: FAIL — blocking contradictions and missing canonical requirements.**  
**Implementation remains NOT AUTHORIZED.**

## 1. Executive conclusion and audit boundary

The owner approval of OD-01 through OD-16 is recorded. The historical proposal's NOT APPROVED labels do not invalidate that later approval. However, approving proposal v1.2 does not resolve contradictions inside that proposal, and the revised normative documents still contain the same-tick creation/completion contradiction identified in the requested R1.2.1 check. No R1.2.1 clarification or approval was found in the inspected tree.

Two further blocking canonical defects exist: a deferred requested completion loses its command identity on save, and the transaction leg encoding cannot independently describe the new stock definition/class required by the recipe test. The acceptance procedures do not supply complete initial states and do not consistently account for all five tick phases.

The C0.3 event-init digest is correct. Event wire records can be packed independently. A complete approved gameplay checkpoint golden cannot be established from this packet without inventing missing state or resolving the findings below. A clearly non-normative structural checkpoint probe is provided instead; it is not a substitute for that missing oracle.

This was a separate specification-review pass, not a self-reported implementation pass. No C0.3 implementation exists in the inspected baseline. Only this requested report was added. No specification, Rust source, manifest, lockfile, archive, or existing golden was edited. No cargo build/test, dependency installation, migration, or implementation was performed. Prior C0.2 execution evidence is attributed to its existing audit, not claimed as newly executed here.

## 2. Evidence and integrity

### 2.1 Documents inspected

- `C0_3_OWNER_DECISION_RECORD.md`, `C0_3_OWNER_DECISIONS_PENDING.md` v1.2-proposal, and `C0_3_OWNER_DECISION_MATRIX.md` as historical decision context.
- `docs/C0_3_IMPLEMENTATION_SPEC.md`, `docs/C0_3_ACCEPTANCE_TESTS.md`, and `docs/C0_3_ARCHITECTURE_REVIEW.md`, all v1.0-approved.
- `C0_3_G3_REMEDIATION_REPORT.md`, the v1.2 remediation proposal, scope traceability, and the earlier `C0_3_INDEPENDENT_ARCHITECTURE_AUDIT.md`. The earlier FAIL concerns the superseded draft; it was not reused as this verdict.
- `C0_2_BASELINE_MANIFEST.md`, `C0_2_INDEPENDENT_AUDIT.md`, and the frozen C0.2 normative documents; existing public event/driver interfaces were inspected for compatibility context.
- Approved G0 inputs: README, Game Vision, GDD v0.2, ADR register, Core, Technical Architecture, LOD, M0, Art Direction, V0, workflow, and G0 review/remediation/change records. Stale G0 gate-status paragraphs were treated as historical, not as new C0.3 authority.

The owner record approves OD-01–OD-16 **as written in v1.2-proposal**, not v1.0/v1.1 and not an unstated v1.2.1. Its reference to the proposal's version as its "first line" is imprecise: the title is line 1 and the version is line 3. The actual version is nevertheless unambiguous. A repository-wide text search found no `R1.2.1` or `1.2.1` occurrence before this report. This is an evidence limitation about the supplied tree, not a claim about private owner communications.

### 2.2 Direct archive comparison, without extraction or mutation

| Artifact | Recomputed SHA-256 | Result |
| --- | --- | --- |
| `Sealed_Heaven_C0_2_Source.zip` | `2804d33a785acb7331a0d82fd1c080bec82d0add348578abb4b1a6807db60a40` | Matches manifest; all 29 file members byte-identical to current corresponding files |
| `Sealed_Heaven_C0_1_Source (1).zip` | `9c3b7907845c8cf0ff2e757c9a5d4125a4264ab6977b7513b1300f16aaa51250` | Matches manifest; 5/9 members unchanged; four C0.2-era differences are Cargo.toml, lib.rs, encoding.rs, tests/mod.rs |
| Available `Sealed_Heaven_G0_v0.2-draft.zip` | `e4d3edba436cc8e1dfcda9a426d3ecb0c6b8165037cca714d38f193c3c47ad4e` | All 16 members byte-identical to current corresponding G0 files |

The available G0 archive identity above is reported as observed, not substituted for any differently named/hash-identified historical handoff artifact. The user's approved-G0 authority premise is retained. The C0.2 archive and its prior PASS are directly identifiable.

All 15 labeled C0.2 document blobs were independently rechecked for full length and SHA-256; all 14 C0.2 domain trailers also verify. `src/tests/golden.rs` SHA-256 is `b0ceb7308b623e0eec406096be421d8a7f24f2cab311ce2d5cb66d1023cf71fa`; `src/tests/c02_golden.rs` is `e60244172eb16387ea9ab4e29b0588062a60beadf0cb56692a7ae74d54148c6c`. These files match the frozen archive. The lockfile remains the manifest's `176e4bb5d145739c101c10296ce9f97d4a24917381be057e74134e0ba1779d75` by byte equality.

### 2.3 Reviewed C0.3 input identities

| File | SHA-256 |
| --- | --- |
| `C0_3_OWNER_DECISION_RECORD.md` | `d1c366140cbb6b784398bcafedf51d8c7139124c2c13fdd4a9ac87711db416b6` |
| `C0_3_OWNER_DECISIONS_PENDING.md` | `4446a5b21cd29c743555d0ca7cfd14ca7d9e381f0cabcedc0528876d341587a5` |
| `docs/C0_3_IMPLEMENTATION_SPEC.md` | `89d4aecfae60ccfe95903da324c5cd36ee019cbf9c4e2ba47aa271f627fa6efb` |
| `docs/C0_3_ACCEPTANCE_TESTS.md` | `13e36f8c7b20272adc16c5d2daf06869e94ab5a243efa5a80d271144617f9dac` |
| `docs/C0_3_ARCHITECTURE_REVIEW.md` | `e35f214441af1f22a416dfb57bd374e4866b49bea71d26cc30e698758cd380dd` |
| `C0_3_G3_REMEDIATION_REPORT.md` | `557ef38a6b16ea56fa783e72506b868c23f6b98a3d78c7a5b3439e882e93857c` |

## 3. Approval traceability

The six owner conditions remain binding: frozen predecessors, fixture-only balance constants, retained C03/M0 exit, headless ownership, no scope expansion, and a separate implementation authorization. Approval of an architecture is not proof that its byte contract is complete.

| Decision | Approved requirement and normative location | Independent result |
| --- | --- | --- |
| OD-01 | New fixture/tag, one current task, position/path/trace/legs, §3 | Incorporated structurally; incomplete provenance/leg/restore contracts, F02–F04 |
| OD-02 | Event domains, kinds 1–7, stage events, §4 | H0 and field widths verify; full emission schedules are not closed, F01/F07 |
| OD-03 | Fixture-local command key and atomic contention, §§5–6 | Numeric order retained; duplicate/rejection/admission precedence incomplete, F05/F07 |
| OD-04 | Multi-reservation bundle, consumable/durable distinction, §6 | Retained; completion planning and exact quantity validation incomplete, F03/F06 |
| OD-05 | Validate all legs, transaction identity, generated provenance, §§3/6 | Rollback rule retained; requested key is not persisted and new-stock fields are missing, F02/F03 |
| OD-06 | Producer 1, claimant/current-task cancel, zero-id cancel, §6 | Retained in outline; terminal no-op versus current-task preflight unresolved, F05 |
| OD-07 | Mutable production enable bit only, §6 | Fixture isolation retained; approved later-wake condition omitted, F06 |
| OD-08 | Silent repeated release/completion, §6 | Intended result retained; legal replay admission path is not specified, F05 |
| OD-09 | Integer scorer, actor order, wake 4, diagonal weights, §§7/12 | Scores/ties/trace fields retained; target/planning/due/eligibility details missing, F06 |
| OD-10 | One later-tick transition; completion flags before invalidation, §8 | Universal creation-tick skip retained but contradicted by M0-T07, F01 |
| OD-11 | Separate searches, authoritative position, graph/grid/cost, §§3/9/12 | Ordering and cost rule retained; complete tests and interrupted-edge behavior missing, F08 |
| OD-12 | Explicit mode command only, §§5/11 | Retained; no automatic activation policy approved |
| OD-13 | Twenty actors, one task/bundle, four supplies of 5, §12/tests §11 | Retained; initial cursors/task fields and complete boundary outcomes missing, F04/F07 |
| OD-14 | Disable > cancel > expiry; release once, §5 | Reason priority retained; multi-task invalidation order and reference cleanup unspecified, F07 |
| OD-15 | Mode → Command → Invalidation → Decision → Work, §5 | Production order retained; not enough to reconcile acceptance timing, F01/F07 |
| OD-16 | Frozen disabled synthetic completion, §1 | Explicitly preserved and current archive identical; production invalidation must not enter fixture 2 |

G0 support is clear for stable identities (ADR-003/Core §2), serial atomic commits (ADR-004/Core §4), physical conservation (ADR-013/Core §7), complete snapshots and prefix replay (ADR-017/022/Core §§5/11), five goals and traces (Core §6), and deterministic navigation (Core §8). No need decay, new planner, automatic mode policy, Bevy dependency, migration, R2 certification, or day-length change is approved here. The gaps below cannot be filled by implementer discretion under those authorities.

## 4. Findings

### F01 — BLOCKER: R1.2.1 same-tick contradiction is still unresolved

**Evidence:** implementation §§7–8, lines 151 and 155; acceptance §12, lines 108–119; approved v1.2 OD-10, lines 265–280, and its M0-T07 worked sequence in the remediation proposal, lines 167–180.

Work skips a task whose `created_tick` equals the staged tick. M0-T07 nevertheless requires `BeginTask` and `CompleteTask(0)` to create task 401 in `Completing`, apply transaction 1, and reach `Done` on tick 1; it repeats this pattern on ticks 2–4. `CompleteTask` only sets a flag. It does not exempt a task from the skip or apply legs during commands.

For a task created on tick 1, the required work rule leaves it unexecuted on tick 1. Therefore event 2 `LegsApplied`, event 3 `Done`, the stock-11 balance, and clearance of `actor.task_id` cannot all hold as asserted. The following hop cannot then begin on tick 2 with no current task as assumed. `BeginTask` also does not explicitly assign its `created_tick`; that omission is not authority to backdate it.

The contradiction already existed in the approved v1.2 material. The owner record approves that version without recording an exception or an R1.2.1 timing resolution. Merely moving completion legs behind invalidation does not fix this separate timing defect.

**Required closure:** record an explicit owner-approved timing clarification, then reconcile the work rule, all M0-T07 inputs, tick/event tables, failure boundary, and golden expectations. This audit chooses neither a same-tick exception nor replacement completion ticks.

### F02 — BLOCKER: requested completion identity is lost across a saved prefix

**Evidence:** implementation §3 task/transaction/pending layouts, lines 57–65; command phase line 120; source-1 transaction rule line 139; one-transition work rule lines 155–165.

A task stores only `completion_requested`, not the originating `CompleteTask` key. Applied transactions store that key only after completion. Consumed commands are not historical state in the pending queue.

Concrete collision: take the same existing `Executing` task 11, created before tick 5, with no decision wake on tick 5. In one run submit `(target=5, actor=1, producer=1, sequence=7, CompleteTask(11))`; in another use sequence 8. Each command is consumed, sets the same flag, and work changes the stage to `Completing` without legs. The stage event bytes are identical because they contain neither sequence. All specified checkpoint fields can now be identical. On tick 6 the requested completion must store source 1 with the actual sequence 7 or 8. A restore cannot distinguish those required futures.

Multiple distinct completion keys setting the flag before it is consumed are also not assigned a first/last/winner rule. This violates Core §§5/11 and the exact source-1 rule, even if M0-T07 timing is separately fixed.

**Required closure:** an approved persisted identity representation and deterministic selection/consumption rule, with a save between request and completion, including multiple requests. Do not invent a producer/sequence sentinel or recover the missing key from a host log.

### F03 — BLOCKER: transaction legs cannot represent required output stock and validation

**Evidence:** implementation §3, scenario line 47, stock line 53, task line 57, transaction/leg line 63; completion rules lines 129–141 and 168; acceptance §§8/12, lines 90 and 108–119.

A stock requires `definition_id` and `class`. A leg contains only op, stock id, quantity, owner kind, and owner id. A credit/transform-in with stock id 0 creates a stock, but carries neither definition nor class. No task field selects a recipe or an input/output-definition binding. The scenario recipe table cannot solve this by itself: no deterministic recipe selection or leg-to-recipe linkage is defined.

M0-T07 requires credit 11 of definition 1 and later transform-in 14 of definition 2. The invalid-leg test even describes "transform in definition 2", which has no field in that leg encoding. Implementers cannot infer the definition/class by an unapproved previous-leg convention or hardcode definitions from this prose.

Also missing are the full leg legality rules: positive quantities, source sufficiency after earlier legs, debit owner-field meaning, existing-credit compatibility, valid owner references, recipe conservation/declared sinks, and whether reserved consumables are debited by planned legs or additionally by the class-1 rule. Container 20/cargo 30 have no defined owner registry or explicit reference-validation contract. There is no complete algorithm relating reservations, planned legs, newly allocated stocks, and consumption without possible omission or double debit.

**Required closure:** approve representable output identity/class and recipe binding, exact validation and accounting rules, and explicit M0-T07 planned-leg bytes. No schema field or implicit convention is approved by this audit.

### F04 — HIGH: allocation namespaces and restore semantics are not closed

**Evidence:** implementation §§2–3/6/10, especially lines 20, 33–77, 129/133; acceptance §§3/10/11.

Tasks and reservations both allocate from `next_record_id`, but the production schema never specifies its full namespace, initial allocation trace, cursor bounds, cross-table uniqueness, or non-reuse validation. C03-T01a preinstalls tasks 11 and 12 yet expects newly allocated reservation 1. Under the inherited single monotonic record-cursor reading, a cursor already beyond issued task 12 cannot issue 1. No approved separate namespace or constructor rule explains this. M0-T06 likewise omits its reservation cursor and expected reservation IDs, so its event bytes cannot be fully derived.

Production restore has only selected local checks. It lacks an exhaustive ordered semantic contract for actor IDs/cursors, stock IDs/cursors, transaction IDs/cursors, sorted uniqueness, references, task↔actor and reservation↔task correspondence, reserved sums, active intervals, terminal/completion consistency, path index/endpoints/passability, due and wake validity, trace consistency, pending payload legality, and `next_event_id >= 1`. Zero progress/flag/discriminant rules are also incomplete. Recipe-list canonical order is not stated.

**Required closure:** approve namespaces and constructors plus a numbered, fixture-3 validation matrix. A checksum is not validation of these invariants. Do not silently import fixture-1/2 semantic rules into a structurally different save.

### F05 — HIGH: total ingress/restore precedence and idempotent replay conflict

**Evidence:** implementation lines 20–22, 61, 113–115, 129, 133–141; acceptance §§5/7/10/14; approved OD-05/06/08/10.

The C0.3 document does not give a complete first-match restore precedence. Revision-before-checksum and some `Corrupt` cases are explicit; short header versus wrong tag, checksum versus parse/fixture/content failures, exact exhaustion, and semantic failures are not fully ordered. Naming distinct classes in acceptance §10 is insufficient. The C0.2 first-match contract applies to its decoder, not automatically to this tag.

Ingress lacks a total diagnostic contract for wrong boundary, late target, unknown actor, producer, malformed payload, duplicate keys, and collisions with already pending commands. A duplicate key in a batch is rejected, but the error and ordering against other malformed records are not given. Stateful current-task/claimant checks need defined evaluation time and terminal no-op precedence.

In particular, a nonzero cancel must name the current task, while a terminal task is supposed to be a silent no-op after its actor reference has been cleared. Replaying a previously applied key uses an old target tick; no precedence is stated against late-input rejection. Replaying a generated key would use producer 0, which explicitly fails admission, while acceptance §14 directs it to the successful replay/no-op rule. No separate legal replay API is specified to reconcile those statements.

**Required closure:** one ordered typed error/no-op contract for each API, including terminal cancellation, old applied keys, generated provenance, and pending duplicates. Acceptance must use the actual approved ingress path, not a hidden mutation or guessed override.

### F06 — HIGH: autonomous task planning and enable/wake semantics remain incomplete

**Evidence:** implementation §§6–8, lines 137 and 145–168; acceptance §14; approved proposal OD-07 line 222 and OD-09 lines 234–261.

The integer scoring formula, actor order, diagonal weights, relationship gate, and tie winner are specified. The rest of a winning decision is not fully determined: food/tool/workstation/cargo definition selection, choice among multiple eligible stocks/targets, goal destination, reservation expiry, task due tick, planned legs, and initial path/progress fields lack complete rules. There is no ordering for competing ineligibility reasons. Consequently the five candidates and their rejection bytes, and the resulting task checkpoint, cannot be independently predicted for general approved inputs.

The eat sequence assumes completion two ticks after selection, but the task's due tick is never assigned by the decision contract; `Completing` only applies legs when requested or exactly due. The five goals are therefore not fully executable merely from the scorer.

The approved OD-07 says that after enabling, a following decision can run only on a later wake. The new normative text sets the bit during commands and evaluates whenever `next_wake_tick == staged_tick`. With an initially disabled idle actor enabled on that exact wake, it permits a same-tick decision. No suppression/rescheduling rule or approved amendment resolves this divergence.

**Required closure:** complete approved goal-specific planning and eligibility rules, with exact five-goal/trace oracles, and an explicit enable-on-wake decision consistent with the recorded approval.

### F07 — HIGH: task transitions and whole-boundary event tables are not reconciled

**Evidence:** implementation §§4–8, lines 99–109, 119–125, 129–168; acceptance §§3/6/11/13.

The work table has no complete behavior for `Queued`, `Reserved` with no path while not at target, `Completing` neither requested nor due, or overdue due ticks. The value of terminal `interruption` after successful `Done` is not explicitly assigned. Invalidation does not explicitly clear `actor.task_id`, although only a non-terminal task may be referenced and decisions run immediately afterward. Invalidation order across multiple actors/tasks is absent; per-task reservation sorting and reason priority do not establish a total event order across the world.

The command-acceptance tables also omit possible later work in the same boundary. C03-T01a and M0-T06 assert only reservation and Reserved-stage events for preinstalled tasks. If such a task was created earlier and is at its target, the specified work phase adds an Executing-stage event that their "only legal sequences" omit. A stored path would instead permit a Travelling-stage event. No exact initial target/path/creation tick or explicit work eligibility rule establishes why neither happens. The disable-after-acquire table similarly asserts no tick-2 event without giving the fields that would make work silent.

The approved v1.2 contention rule distinguishes lost contention (4) from on-hand insufficient (6); the normative reservation algorithm does not fully restate this choice or identify which failed bundle pair supplies the rejection payload. There is also no exact rule for the stage event after additional accepted reservations on an already Reserved task. Only actual stage changes are approved to emit kind 7.

**Required closure:** complete the task transition/cleanup/overdue contract and produce full-boundary event sequences, including command-induced work and multi-task invalidation, from fully assigned initial states. Do not suppress the work phase merely to match a table.

### F08 — HIGH: navigation contract and required navigation tests are incomplete

**Evidence:** implementation §§3/9/12, lines 49–57 and 172–176/194; acceptance §§13–14, lines 125/145.

Separate graph/grid search, positive integer costs, successor order, and lexicographic equal-cost comparison are approved. But no exact search endpoints or full expected path are supplied for the grid or tie tests. The approved fixture has only the two cost-1 route edges; the cost-2 resume and equal-cost alternative-route sentences do not declare an additional graph fixture. They cannot be treated as authorization to alter the fixed graph silently.

The contract also leaves failed/cancelled travel while on an edge unresolved: the authoritative actor can remain in edge locus, but new route search starts from site targets/nodes without defining routing from that intermediate position. Stored-path validation, edge-locus site-field meaning, and failure discovery/event ordering relative to invalidation/decision/work need explicit rules. Acceptance's unreachable case does not fully initialize actor position, task stage, target/path state, wake, or IDs to establish when it fires.

**Required closure:** exact approved reachable, equal-cost, blocked-grid, unreachable, and mid-edge save/cancel scenarios, with positions, paths, ticks, events, and deterministic validation outcomes.

### F09 — HIGH: acceptance coverage is not an executable exit oracle

**Evidence:** acceptance §§3–14; Core §§6–8/11; M0 §6; detailed matrix below.

No shared complete fixture constructor/default block supplies the omitted canonical fields. Generic "same bytes", "replay", and "returns an error" sentences cannot define the initial state, exact error, or expected checkpoint. Several rows are only sketches; work and haul selection are not tested as autonomous goals, and Rest/Return Home are tested only from preinstalled Completing states. Hostile-save cases do not provide checksum-repaired concrete edits and single-error precedence witnesses. Score/route overflow cases have no exact approved overflow inputs.

**Required closure:** exact initial states, boundary-indexed commands, full event records, final canonical state, single named errors, and explicit split/replay points for every mandatory case. Resolve F01–F08 before freezing the resulting gameplay goldens.

## 5. Canonical and tick-contract verification

| Requested check | Result |
| --- | --- |
| Complete `SH:C03:STATE` revision 1 | FAIL: field order exists, but missing completion identity and new-stock information; full semantic validation absent |
| Complete `SH:C03:EVENT:v1` | Wire widths/H0 PASS; exact emission schedule FAIL/partial |
| Deterministic command ordering | Numeric tuple PASS; full admission/duplicate/diagnostic contract incomplete |
| Reservation/transaction identity | Partial: cursor names exist; namespace, replay precedence, and deferred request identity incomplete |
| Bounded decoding | Partial PASS: checked count×minimum-size and grid bounds are explicit; full discriminator/exhaustion/semantic matrix absent |
| Error precedence | FAIL: no complete C0.3 first-match specification |
| Atomic rollback | Whole-boundary intention PASS, including earlier staged transactions; executable invalid-leg/error cases incomplete |
| Exactly-once completion/release | Intention PASS; valid replay ingress, terminal reference cleanup, and key persistence incomplete |
| Actor/task cardinality | At-most-one current non-terminal task PASS in intent; exact actor constructors and load/cursor/link rules incomplete |
| Navigation/position persistence | Fields present; full valid-state and interrupted-route behavior incomplete |
| DecisionTrace persistence | Field inventory and wake suppression present; exact candidate eligibility/task creation/trace examples incomplete |
| Save/restore replay equivalence | FAIL: F02 gives an actual future-state collision; complete gameplay vectors unavailable |

Fixture 3's declared order is exactly **Mode → Command → Invalidation → Decision → Work**. `CompleteTask` has no approved path to apply legs before invalidation. Disabled production jobs must fail before legs. The rule to skip a creation-tick task is explicit; no approved same-tick exception was found. M0-T07 violates it.

Fixture 1 retains its own commit path. Fixture 2 retains **Mode → Command → synthetic Work**, including disabled synthetic completion. The statement that old fixtures do not enter the new last three phases must mean the *production* phases, not removal of fixture 2's frozen synthetic work. OD-16, the explicit C0.3 freeze sentence, and the archive baseline prohibit that removal. No production invalidation or scorer is authorized in fixture 2.

## 6. Acceptance coverage matrix

Every row below also needs a pinned seed/scenario identity and all omitted canonical fields, either directly or by one explicit shared constructor. A symbolic tick can be valid if every dependent value is fixed relative to it; these procedures do not yet provide that completeness.

| Required case | Existing exactness | Missing or conflicting evidence | Result |
| --- | --- | --- | --- |
| C03-T01a | Two command keys, quantity/expiry, three event payloads | Full initial state/cursors/paths/wakes; allocation conflict; whole-boundary work events | BLOCKED |
| C03-T01b | Expiry 3 beats due 3; release→Failed→TaskFailed | Exact saved boundary, full stock/task/actor state, empty inputs, repeat-release key and final bytes | PARTIAL |
| C03-T01c | Claimant cancel and cancellation event order | Sequence/boundary/full initialization; named non-claimant error and terminal-no-op precedence | PARTIAL |
| C03-T01d | Disable after acquire; disable beats completion | Task timing makes ticks 1/2 underdetermined; exact disable/complete keys and full final state | BLOCKED |
| C03-T01e | Silent repeat intended | Applied transaction fixture and legal replay path/late/producer precedence | BLOCKED |
| C03-T01f | Invalid later leg must roll boundary back | Definition cannot be encoded; full planned bytes, start stage/state, inputs and named error absent | BLOCKED |
| C03-T01g | Active-reservation prefix/suffix intention | No pinned split, future pending command, request provenance, complete initial/final bytes | BLOCKED |
| C03-T01h | Some capacity/corrupt classes named | Exact inputs/edits and precedence; valid next-boundary control; fixture-safe overflow construction | PARTIAL |
| C03-T01i | Grouped explicit boundaries intended | No complete retained log or per-boundary reference states/events | PARTIAL |
| M0-T06 | 20 actors, supplies 5, 40 declared events/cursor 41 | Exact new reservation IDs, task fields, wakes, allocation cursor, work suffix and final task state | BLOCKED |
| M0-T07 | Owner chain, quantities, asserted transactions/events | Same-tick contradiction; incomplete command payloads, recipe/leg encoding and initial state | BLOCKED |
| M0-T08 | Single reason priority and terminal sequences | Exact simultaneous keys/IDs, no-reservation cancel, unreachable setup and legal repeated-key ingress | PARTIAL/BLOCKED |
| Needs/DecisionTrace | Eat score can be 1,000,000; wake +4; one winner intended | Exact stock/task/reservation IDs, all five candidate eligibility/reasons/terms, due/expiry and full trace bytes | PARTIAL |
| Eat | Executing→Completing→Done intent | Goal plan/due/food definition and complete state needed | PARTIAL |
| Rest / Return Home | Terminal success/disable sequences | Autonomous selection/target/route, zero-id cancel keys, complete lifecycle/replay missing | PARTIAL |
| Work | Durable release described | Complete goal-selection fixture, atomic tool+workstation failure, due/plan and completion lifecycle absent | BLOCKED |
| Haul | Scripted ownership chain retained | Autonomous selection/target plan absent; scripted chain contradicts timing | BLOCKED |
| Navigation | Successor/cost rules | Complete graph/grid starts/targets/path/tick/events; cost-2/tie subfixtures and interrupted edge | BLOCKED |
| C0.1/C0.2 regression | Commands and frozen goldens explicit; archive identity verified | Not re-executed in this read-only specification gate; required again after authorized implementation | REQUIREMENT RETAINED |

No mandatory row is considered passed solely because it is named in the coverage table or remediation report.

## 7. Independent packing and golden-vector results

### 7.1 Method and event-init digest

Calculations used Python standard-library `struct`, `int.to_bytes`, and `hashlib`, directly from the document's widths/tags; no Rust encoder, simulation run, or dumped snapshot supplied an expected value. Fixed event examples were cross-checked using independent byte-by-byte integer assembly. The structural checkpoint was independently walked with `int.from_bytes`, checking field exhaustion, scenario identity, and trailer.

Exact 20-byte preimage, no newline or terminator:

```text
ASCII: SH:C03:EVENT-INIT:v1
HEX:   53483a4330333a4556454e542d494e49543a7631
SHA256 ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c
```

This matches all three normative C0.3 documents and the remediation report. **H0: PASS.**

### 7.2 Wire-size verification

| State record | Bytes, excluding outer table count |
| --- | --- |
| Scenario | `256 + 49 × recipe_count` |
| Actor, including position | 131 |
| Site / edge | 9 / 32 |
| Grid descriptor | `16 + width × height` |
| Stock / reservation | 34 / 53 |
| Task | `89 + route_step_count×9 + grid_step_count×2 + reservation_count×8 + planned_leg_count×26` |
| Applied transaction | `50 + leg_count×26` |
| Leg | 26 |
| DecisionTrace, five candidates | 324 |
| Pending command | 24-byte key plus tagged payload; BeginTask's empty-leg tagged payload is 36 bytes |

Every table count adds 8 bytes; hashes are 32 raw bytes. These are structural size results, not approval that the represented gameplay state is complete. In particular, a 26-byte leg lacks the definition/class binding identified in F03.

Event header is 21 bytes; complete kinds 1–7 are respectively **45, 38, 30, 37, 30, 78, 30 bytes**.

### 7.3 Proposed event codec vectors, not acceptance of an inconsistent scenario

For each independent row: event id 1, tick 1, actor 1, starting digest H0. Tuples below are the complete payload inputs in §4 field order. These test wire packing only; they are not a new approved fixture.

| Kind | Payload tuple | Bytes | SHA-256 of complete event |
| --- | --- | --- | --- |
| 1 | `(1,1,1)` | 45 | `2b2d2d6a37cd667a27328ee045c9e3007536946c1d39a777165ab2a7f3c7fd20` |
| 2 | `(1,1,4)` | 38 | `c7f04fedcd59d84421269ea30ea239c8d72a32d3e667e54e03e7bd4d8ade9fbc` |
| 3 | `(1,3)` | 30 | `0cf87591b1cb19feaab659fc253d0832121bc2fc1716538f931597db070bc62f` |
| 4 | `(1,11)` | 37 | `5d122547903aacf9a7a51c8278210cccd609985ac649ff5859d2eda1bc0a996b` |
| 5 | `(11,3)` | 30 | `fe995a9abd3ed9d98bbbc07fb9b97ddc3003ab07e6e524fed7507cc816f65175` |
| 6 | `(1,13,5,1000000,0,0,0,0)` | 78 | `e9a8f7158d4ccafb2035206ddbeea25d028c03a3957402f88f349189ed247290` |
| 7 | `(11,2)` | 30 | `0c7d845291947eacf394525dbd07409b30d9aed13d01aebb9e4b96078e6bb4a7` |

The three **declared** C03-T01a event records pack as follows:

```text
E1 (45 bytes):
010000000000000001000000000000000100000001010000000000000001000000000000000100000000000000
E2 (30 bytes):
0200000000000000010000000000000001000000070b0000000000000002
E3 (38 bytes):
0300000000000000010000000000000002000000020100000000000000010000000000000004
```

| Result | Value |
| --- | --- |
| Chain after E1 | `9de4bcd4d3f9b8085306bbfe10e73de325657bce0cd433a5f87e95a98fd40d0f` |
| Chain after E2 | `2f8d5d9b65de0b762f86fd342ebef769a64d8826b22877092a4ed8264a90569b` |
| Chain after E3 | `a96fedf53095473dd1fea416dbe02a83b7c834ac7743cfa393931d801fc92a13` |
| E1∥E2∥E3 length / next cursor | 113 bytes / 4 |
| SHA-256 of E1∥E2∥E3 | `e8791a9c04befb7baff43d5a55e1f4d2a747a0dd93bae2a903fadf65015c1661` |

These hashes prove the published event *table's packing*, not its compatibility with work/identity rules. F04/F07 must be resolved before using it as a complete boundary oracle.

For the contradictory M0-T07 **declared tick-1 table**, the three events have lengths 30/37/30 (97 total), stream SHA-256 `feef9602eeedb82dda9634a20205c28d7ba8eedd38fc8e3d91fe8d29355cced7`, and final chain `f7848581cb87d505b554517df6ab63299d7f988908353a93f5fdfc4830570890`. Those numbers do **not** authorize same-tick completion; the table remains rejected under F01.

### 7.4 Proposed structural checkpoint probe — explicitly NOT an approved gameplay golden

No complete valid acceptance checkpoint can be packed without missing defaults and resolutions. To separate that failure from the ability to pack the envelope, the independent packer assigned this diagnostic input: tick 0; seed 100; all four allocation cursors 1; approved weights/minimums/wake; zero recipes; no actors/sites/edges/stocks/reservations/tasks/transactions/pending commands/traces; absent grid; event cursor 1 and H0. Empty world and zero recipes are **audit probe inputs only**, not approved fixture defaults or a claim that restore must accept them. No alternative gameplay behavior is selected.

| Component | Length | SHA-256 |
| --- | --- | --- |
| Scenario block | 256 | `32643d2f2dbf0b8753b9ebc69d8e7a0e0232b4758c17d4a5105747a5c8e645cd` |
| Complete body | 482 | `853250ea8c5d516fc25d7208db8686a612fa9f63d5a762a20493a235d60b35f8` |
| Body plus trailer | 514 | `52ae864931f1fbfa9ad75f64b98ad79fc456775660ed5b014b7364f735bcad53` |

The trailer is the body hash in the second row, not the whole-blob hash in the third. A separate field walk consumed exactly 482 body bytes plus 32 trailer bytes and verified the scenario hash and trailer. This probes the envelope only; it deliberately cannot close the missing gameplay checkpoint gate.

### 7.5 Reproducible independent packer

This code is included as audit evidence inside this report; no packer/module was added to the Rust package. It prints complete vector hex, lengths, hashes, and event chains. It imports no simulation implementation.

```python
import hashlib
import struct

sha = lambda b: hashlib.sha256(b).digest()
H0 = sha(b"SH:C03:EVENT-INIT:v1")
DOMAIN = b"SH:C03:EVENT:v1"
formats = {1: "QQQ", 2: "QQB", 3: "QB", 4: "QQ",
           5: "QB", 6: "BQQ5q", 7: "QB"}

def event(eid, tick, actor, kind, *payload):
    return struct.pack("<QQIB" + formats[kind],
                       eid, tick, actor, kind, *payload)

def show_chain(name, events):
    h = H0
    for e in events:
        h = sha(DOMAIN + h + e)
        print(name, len(e), e.hex(), sha(e).hex(), h.hex())
    stream = b"".join(events)
    print(name, "stream", len(stream), sha(stream).hex())

samples = {1: (1, 1, 1), 2: (1, 1, 4), 3: (1, 3),
           4: (1, 11), 5: (11, 3),
           6: (1, 13, 5, 1000000, 0, 0, 0, 0), 7: (11, 2)}
for kind, args in samples.items():
    e = event(1, 1, 1, kind, *args)
    codes = list(("QQIB" + formats[kind]).replace("5q", "qqqqq"))
    widths = {"Q": 8, "I": 4, "B": 1, "q": 8}
    independent = b"".join(
        value.to_bytes(widths[code], "little", signed=(code == "q"))
        for value, code in zip((1, 1, 1, kind, *args), codes))
    assert e == independent
    show_chain("kind" + str(kind), [e])

show_chain("declared-C03-T01a", [event(1, 1, 1, 1, 1, 1, 1),
    event(2, 1, 1, 7, 11, 2), event(3, 1, 2, 2, 1, 1, 4)])
show_chain("REJECTED-M0-T07-tick1", [event(1, 1, 1, 7, 401, 5),
    event(2, 1, 1, 4, 1, 401), event(3, 1, 1, 7, 401, 6)])

# NON-NORMATIVE structural probe, not an approved gameplay fixture.
weights = [1000 if i == j else 0 for i in range(5) for j in range(5)]
scenario = struct.pack("<Q25q5qQ", 4, *weights, *([-2**63] * 5), 0)
body = (b"SH:C03:STATE" + struct.pack("<HI", 1, 3) + sha(scenario)
        + struct.pack("<6Q", 0, 100, 1, 1, 1, 1) + scenario
        + struct.pack("<3Q", 0, 0, 0)       # actors, sites, edges
        + struct.pack("<QII", 0, 0, 0)      # absent grid
        + struct.pack("<6Q", 0, 0, 0, 0, 0, 0)  # remaining tables
        + struct.pack("<Q", 1) + H0)
blob = body + sha(body)
assert len(scenario) == 256 and len(body) == 482 and len(blob) == 514
print("scenario", len(scenario), sha(scenario).hex())
print("body", len(body), sha(body).hex())
print("NON-NORMATIVE-probe", len(blob), sha(blob).hex(), blob.hex())
```

The packer does not revise a normative constant. Event-init verification is complete; event codec vectors are proposed as explicitly identified; valid acceptance checkpoint golden generation is **BLOCKED**, not silently filled with this probe or implementation-derived values.

## 8. Remediation claim disposition and final recommendation

The remediation report correctly records the owner decision version, retains the written exit, protects fixture 2, and withholds implementation authorization. Its claims that the schema is complete and that each retained test has one executable sequence are not supported by the current normative text. Stage-event tables and ordered phases alone do not supply omitted task fields, provenance, or transaction definitions.

Before a future independent re-review, the owner/specification author must explicitly resolve F01–F03, close F04–F08, and publish the complete acceptance fixtures required by F09. Any changed timing, schema, or enable/wake behavior must be traceable to an approved clarification/amendment rather than inferred from v1.2. The revised packet must then be independently packed into valid checkpoint and complete event goldens, preserving all C0.1/C0.2 bytes. That is a future documentation action, not an edit performed or authorized here.

**Final recommendation: FAIL — do not authorize C0.3 implementation from this packet.** The evidence is sufficient for FAIL rather than INCONCLUSIVE because the retained M0-T07 timing contradicts the normative work rule and the deferred-completion checkpoint demonstrably loses required future state. C0.2's existing PASS is unchanged. No C0.4 or implementation work is authorized by this audit.

## 9. Non-modification verification

Before this report, the workspace contained 46 files. A sorted content manifest used each relative POSIX path, NUL, SHA-256 hex, and newline; its SHA-256 was `382507fbf22cb8d83ddcfb00cf4786eebe8a8ffd4832eacbd838981e976f1d96`. After report creation, recomputing the same manifest with only `C0_3_G3_INDEPENDENT_AUDIT.md` excluded matched that value and the count of 46: **all pre-existing files unchanged**. The embedded independent packer was also executed from the report and its stream/checkpoint assertions passed. The available archives were read directly with `zipfile`, never extracted or rewritten. This scope check does not claim a new cargo test pass or R2 certification.
