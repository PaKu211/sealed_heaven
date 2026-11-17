# C0.3 S1 Targeted Remediation Report

**Baseline:** `552ed25` — "Implement the C0.3 S1 resource core."
**Branch:** `fix/c03-s1-independent-audit`
**Date:** 17 November 2026
**Scope:** owner-authorized targeted correction of the independent S1 findings B01 and H01–H10 only.
**Out of scope:** S2, S3, S4, any merge, any push, any acceptance (PASS) declaration.

**Result:** B01 and H01–H08, H10 are corrected and covered by regressions. **H09 remains BLOCKED** by a direct conflict between binding condition 4 and the H09 owner direction; it is documented, not silently resolved. The mandatory verification suite passes in debug and release.

---

## 1. Authorization and correction strategy

The correction follows the owner authorization for S1 remediation: surgical changes to the existing deterministic integer simulation, no rewrite, no dependency or toolchain change, no frozen C0.1/C0.2 vector change, and no checkpoint revision bump. The independent audit corpus in `sealed-heaven-s1-audit-552ed25` is used as the acceptance-style oracle for the corrected behavior.

Two audit findings required contract clarification before code could be judged:

- **H10** (goal-0 transition) was resolved by an explicit normative clarification in `docs/C0_3_IMPLEMENTATION_SPEC.md` §8 and new §8.1. It preserves C03-T01d's event-free tick 2 and T01a's tick-1 oracle.
- **H09** (applied-key history) was stopped because the H09 owner direction conflicts with an existing binding owner condition. Details in §5.9.

No S2/S3/S4 behavior was implemented. No autonomous decision, need, scoring, navigation, or multi-NPC behavior was added.

---

## 2. Status summary

| Finding | Severity | Correction | Final status |
| --- | --- | --- | --- |
| B01 | Blocker | Full semantic restore validator, bounded parsing, no accept-and-normalize | **CLOSED** |
| H01 | High | Count preflight, grid bounds, truncation/parse/content precedence | **CLOSED** |
| H02 | High | Batch-aware BeginTask ownership, execution-time recheck | **CLOSED** |
| H03 | High | Same-phase zero-completion identity, ingress and restore | **CLOSED** |
| H04 | High | Aggregate reservation accounting with checked arithmetic | **CLOSED** |
| H05 | High | Approved first-match admission precedence | **CLOSED** |
| H06 | High | Invalidation ordered by actor, then task | **CLOSED** |
| H07 | High | Complete schema validation at ingress and application | **CLOSED** |
| H08 | High | Preserve and validate scenario content; reject altered content | **CLOSED** |
| H09 | High | None applied | **BLOCKED** — owner conflict |
| H10 | High | Normative goal-0 clarification plus implementation | **CLOSED** |

Independent restore expectations: **57/57 PASS** after correction (baseline: 6 PASS / 51 FAIL).

---

## 3. Preserved invariants

The following were not changed:

- C0.1/C0.2 source behavior, golden vectors, and tests (`src/domain.rs`, `src/tests/c01.rs`, `src/tests/c02.rs` have no diff).
- `Cargo.toml`, `Cargo.lock`, and the pinned toolchain `1.100.0`.
- Checkpoint tag `SH:C03:STATE`, revision `1`, fixture id `3`, the 305-byte scenario block, the 41-byte consumed-registry record, and the actor width `131`.
- T01a tick-1 oracle: body SHA-256 `11f9aae46d7f0f04b6c3efead0a25c3086bc56904877d237c0a79ae032a293d9`, length `1262`, digest `5fdbc5707211fbfc309f5ed75e21eaa83ebf3d7307da60cc442a3cd7f8340908`.
- H0: `SHA-256("SH:C03:EVENT-INIT:v1")` = `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c`.
- Deterministic ordering (`BTreeMap`/`BTreeSet`, explicit sorts, integer arithmetic), fixture isolation, canonical encoding, event digest chain, atomic boundary rollback, shared record cursor, and completion provenance.

---

## 4. Source correction map

| Area | File | Change |
| --- | --- | --- |
| Semantic restore validation | `src/fixture3/validation.rs` (new) | Cursors, links, domains, tables, reservations, transactions, registry, pending, traces |
| Bounded parsing | `src/fixture3.rs` | `Reader::count(min)` and `Reader::check_order`; per-section minimum-size preflight; grid/empty-shape/remaining checks |
| Restore precedence | `src/fixture3.rs` | Truncation before tag; checksum before decode; complete-body consumption before fixture/content semantics |
| Scenario fidelity | `src/fixture3.rs` | Decode and retain the whole scenario block; validate content hash and exact standard scenario |
| Admission | `src/fixture3.rs` | Batch-wide producer/actor/duplicate/pending order; scheduled Begins keyed by command; zero-completion same-phase; full Begin payload shape |
| Reservations | `src/fixture3.rs` | `held()` checked aggregate, including own active reservations; preflight before any event; single shared capacity |
| Invalidation | `src/fixture3.rs` | Sort by `(actor_id, task_id)` |
| Legs and owners | `src/fixture3.rs` | Class domain, non-zero definition, owner existence, checked availability |
| Goal-0 work | `src/fixture3.rs` | Executing goal 0 with empty unconsumed list is a no-op; non-empty list advances to Completing |
| Codec | `src/fixture3.rs` | Typed `PathStep` (no raw step bytes), preserved transaction `applied`, decoded 320-byte traces |
| Normative contract | `docs/C0_3_IMPLEMENTATION_SPEC.md` | §8 rules 9a/9b and §8.1 |
| Acceptance prose only | `docs/C0_3_ACCEPTANCE_TESTS.md` | C03-T01b field wording; expectations unchanged |

No checkpoint revision, registry width, or frozen vector changed.

---

## 5. Findings

### 5.1 B01 — Restore published semantically invalid worlds — CLOSED

**Original defect.** `restore` decoded directly into the maps and returned a `Simulation` without semantic validation. Cursors could be zero/reused, IDs could collide across task/reservation namespaces, duplicate table rows were silently collapsed by `BTreeMap::insert`, dangling task/actor/stock/reservation links survived, registry rows could have `resolved = 0` or duplicate role-1 rows, and a same-checksum body could change bytes on load/save. `missing_task_actor` restored and then panicked during `drive` at `expect("task actor")`.

**Correction.**
- New `src/fixture3/validation.rs` runs after parsing and integrity checks, before any `Simulation` is returned. It validates:
  - all five cursors non-zero with every stored ID `< cursor`;
  - task and reservation namespaces disjoint;
  - sites, holders, edges, stocks, reservations, tasks, transactions, registry, pending, and traces strictly ordered and unique;
  - site flags, grid flag, holder kinds, edge endpoints, and non-zero edge cost;
  - actor enabled/mode/channel domains, locus/position rules, home link, current-task link, and non-terminal ownership;
  - stock class `1|2`, non-zero definition, and existing owner;
  - task goal `0..=5`, recipe `0|1`, stage `1..=8`, creation bound, valid target, path index/step shape, interruption domain, unconsumed ordering/identity, reservation links;
  - active reservation claimant, interval, state, expiry, and checked aggregate `held <= stock.quantity`;
  - transaction `applied == 1`, `Done` task, unique task/key, source-1 registry link, source-2 producer/sequence/no-registry, leg shape and ledger consistency;
  - registry record identity, role 1/2 constraints, one role-1 row per task, source-1 transaction link, and no pending/unconsumed overlap;
  - pending target/producer/actor, payload shape, and same-phase zero-completion dependency;
  - trace actor existence, 320-byte width, ordering, and field domains.
- Duplicate and unsorted rows are rejected, never normalized.
- Planned task legs are deliberately **not** rejected for class/owner/stock/quantity on load: they are declarations that must fail in work with `InvalidLeg`, preserving the approved later-leg rollback case. No registry width or revision changed.

**Regression tests.** `b01_cursors_links_and_domains_are_rejected_before_publication`, `b01_duplicate_and_unsorted_tables_are_not_normalized`, `b01_reservations_and_registry_are_validated`, `b01_completion_registry_transactions_and_all_roles_roundtrip`, `b01_duplicate_stock_registry_pending_and_namespace_collision`, `b01_codec_preserves_variable_width_paths_and_validates_shape_without_navigation`.

**Expected vs actual.**

| Case | Before | After |
| --- | --- | --- |
| `record_cursor_zero` | `OK` | `InvalidState` |
| `duplicate_stock` | `OK`, overwrote stock 1 | `InvalidState` |
| `missing_task_actor` | `OK`, then target panic | `InvalidState` |
| duplicate registry/pending keys | `OK`, collapsed | `InvalidState` |
| unsorted/duplicate actors | accepted except one duplicate check | `InvalidState` |
| valid completion provenance (`pending_completions_restore`) | mismatched/rejected in intermediate validator iteration | `OK`, byte-identical roundtrip, baseline-reproducing events |

**Evidence.** `audit-probe-debug.txt` / `audit-probe-release.txt`; `adversarial-comparison-debug.txt` / `-release.txt`; `restore-expectations.tsv` now 57/57 PASS.

**Status:** **CLOSED.**

---

### 5.2 H01 — Count/grid guards and restore precedence — CLOSED

**Original defect.** Section counts were parsed and allocated before a declared-size check, so `u64::MAX` counts produced `Truncated` (or later `Corrupt`) instead of `Corrupt`. Grids `4097x1` and `0x3` restored successfully; a 12-byte tag returned `UnsupportedFormat` instead of `Truncated`. Fixture/content checks preceded complete-body/trailing-byte validation.

**Correction.**
- `Reader::count(minimum)` rejects `u64::MAX`, checks `count * minimum` against bytes remaining, and returns `Corrupt` before allocation/parse.
- Grid width and height must be `<= 4096` and both zero or both non-zero; `grid_site` must agree with the zero shape; `cells <= remaining`; all cells `<= 1`.
- `restore` checks length before tag, verifies the trailer checksum before decoding, and decode rejects trailing bytes before fixture/content semantics.
- `decode_body` now reads the complete body first and validates fixture id and scenario after structure is consumed.

**Regression tests.** `h01_counts_header_grid_and_parse_before_semantics`, `h01_truncations_bad_framing_and_content_precedence_never_publish`.

**Expected vs actual.**

| Case | Before | After |
| --- | --- | --- |
| `count_max_actors` and eight other max counts | `Truncated` | `Corrupt` |
| grid `4097x1` with 4097 bytes | `OK` | `Corrupt` |
| grid `0x3` empty | `OK` | `Corrupt` |
| 12-byte `SH:C03:STATE` header | `UnsupportedFormat` | `Truncated` |
| checksum-valid body with trailing byte | could reach semantic parse | `Corrupt` |

**Evidence.** Probe cases `count_max_*`, `grid_exceeds_body`, `grid_single_zero`; `tests::h01_*`.

**Status:** **CLOSED.**

---

### 5.3 H02 — Multiple BeginTask commands could orphan tasks — CLOSED

**Original defect.** Batch admission tracked only actor IDs created in the submission phase, and execution did not recheck ownership. Two BeginTask commands for one idle actor in one batch both passed preflight, created two non-terminal tasks, and left one orphan. The future variant created two tasks across later boundaries because no execution guard existed.

**Correction.**
- `drive` scans the whole sorted batch before per-command classification (producer, actor, duplicate/pending).
- Scheduled Begins are keyed by full `CommandKey`, and a Begin is rejected when another Begin for the same actor and same `target_tick` is already scheduled.
- `apply_command` rechecks `actor.task_id == 0` for Begin and all task/reservation target links at execution time.
- `apply_due` rejects a zero completion unless the same phase actually created the current task, and returns `InvalidState` for a second future Begin.

**Regression tests.** `h02_same_phase_and_future_begin_cannot_orphan_tasks`.

**Expected vs actual.** Same-phase duplicate Begins: before two tasks and two kind-7 events; after `InvalidState` and byte-identical rollback. Future variant: first Begin admitted; second execution boundary `InvalidState` with unchanged bytes (probe `future_two_begins` DRIVE-2).

**Status:** **CLOSED.**

---

### 5.4 H03 — CompleteTask(0) did not require same-phase creation — CLOSED

**Original defect.** A zero completion was accepted whenever the actor was idle and no earlier Begin existed in the batch, even without any Begin at all, and even when the Begin targeted a different tick. Admission also ignored whether an actual task existed.

**Correction.**
- Admission requires an earlier scheduled Begin for the same actor **and** the same `target_tick` as the zero completion.
- Execution records the task created in the current phase per actor; a zero completion must resolve to that task.
- `apply_command` requires a current non-terminal task; a non-zero completion must equal `actor.task_id`.
- Restore validates that a pending zero completion has an earlier same-tick Begin for the same actor and no invented `target_tick == created` rule (so valid preinstalled completion-provenance saves round-trip).

**Regression tests.** `h03_zero_completion_requires_an_actual_same_phase_begin`, `h03_future_same_phase_dependency_survives_restore_and_requires_successful_creation`.

**Expected vs actual.** `complete_zero_without_begin`: before `OK` plus provenance on the old task; after `InvalidPayload` with unchanged bytes. `zero_complete_cross_tick_begin`: before `OK`; after `InvalidPayload`. Valid create-tick zero completion still completes on a later tick and survives save/restore.

**Status:** **CLOSED.**

---

### 5.5 H04 — Repeated own reservations could exceed capacity — CLOSED

**Original defect.** `held_by_others` excluded all reservations owned by the requesting task, and only one command's pairs were checked. Two own reservations could jointly exceed on-hand stock, and two `u64::MAX` requests could overflow the aggregate without a checked preflight. The state then survived restore only because B01 accepted it.

**Correction.**
- `held()` sums all active reservations for a stock with `checked_add`, optionally excluding one task.
- `reserve` preflights every requested aggregate **before** emitting any success or rejection event: each request must fit on-hand, and `held + requested` must not overflow.
- Availability uses `stock.quantity - held` with checked subtraction, so own earlier reservations consume the same capacity.
- `apply_leg` uses `held(..., Some(task_id))` with checked arithmetic.

**Regression tests.** `h04_own_reservations_share_capacity_and_checked_aggregate`, `h04_multiple_own_holds_fit_then_debit_and_release_exactly_once`, `h04_bundle_contention_has_one_rejection_no_partial_allocation`.

**Expected vs actual.**

| Case | Before | After |
| --- | --- | --- |
| two 1-unit reserves on 1-unit stock, same task | `OK`, two active holds, total 2 | one hold; second rejected by kind-2 event; events `[1,7,2,7]`; one reservation remains |
| `u64::MAX` stock, two `u64::MAX` requests | accepted (overflow) | `CapacityExceeded`, full rollback |
| valid multiple holds within capacity | not covered | fit, debit once, release once |

**Status:** **CLOSED.**

---

### 5.6 H05 — Admission precedence — CLOSED

**Original defect.** The duplicate scan ran before producer/actor checks, so duplicate producer-0 or duplicate unknown-actor batches returned `ConflictingIdentity` instead of `InvalidProducer`/`UnknownActor`. A lower-key late command plus a higher-key producer-0 command returned `LateTarget` instead of `InvalidProducer`.

**Correction.** `drive` performs batch-wide checks in the approved order before per-command classification: any producer ≠ 1 → `InvalidProducer`; any unknown actor → `UnknownActor`; duplicate/pending key → `ConflictingIdentity`. Fixture 1 and 2 preflight are untouched.

**Regression test.** `h05_producer_and_actor_checks_precede_duplicate_and_late_keys`.

**Expected vs actual.** Duplicate producer 0: before `ConflictingIdentity`; after `InvalidProducer`. Producer-0 plus late target: before `LateTarget`; after `InvalidProducer` on the producer-0 key. Both roll back.

**Evidence.** Probe `producer_precedence_across_keys` → `InvalidProducer`; `invalid_producer_duplicate` changed accordingly.

**Status:** **CLOSED.**

---

### 5.7 H06 — Invalidation order — CLOSED

**Original defect.** Invalidation iterated task IDs only, so an actor owning a higher task ID could be invalidated before an actor owning a lower task ID. Existing fixtures hid this because task and actor IDs ascended together.

**Correction.** Invalidation sorts task IDs by `(actor_id, task_id)` before releasing reservations, moving unconsumed records, setting terminal stages, emitting events, and clearing `actor.task_id`. Release order within a task remains ascending reservation ID.

**Regression tests.** `h06_invalidation_orders_actors_not_record_ids`, `h06_disable_cancel_expiry_preempt_completion_and_replays_remain_silent`.

**Expected vs actual.** Cross-owned tasks: before actor-2 pair preceded actor-1 pair; after events are actor 1 kind 7, kind 5, then actor 2 kind 7, kind 5 — probe `invalidation_actor_order` shows `1:1:1:7`, `2:1:1:5`, `3:1:2:7`, `4:1:2:5`.

**Status:** **CLOSED.**

---

### 5.8 H07 — Out-of-schema records could be published — CLOSED

**Original defect.** `BeginTask` shape checking covered stage, leg count, and the actor's locus only. Goal 255, recipe 99, and target locus 9/site 99 were accepted and stored. New-stock credit validated the owner but not the class, so a class-9 stock could be minted by a successful completion.

**Correction.**
- `payload_shape_ok` validates goal `0..=5`, recipe `0|1`, target locus/site/cell against the actual world, leg count, and each declared leg's op/class/definition/quantity shape.
- `apply_command` revalidates payload shape and current task links at execution.
- `apply_leg` rejects class other than `1|2` and definition `0`; new-stock credit is bounded by the legal class domain, and existing-stock credit still requires class/definition/owner equality.
- A bad planned leg stays restorable as a declaration and fails the applying boundary with `InvalidLeg` and byte-identical rollback.

**Regression test.** `h07_schema_invalid_begin_and_new_stock_class_do_not_publish`.

**Expected vs actual.** Begin goal 255 / recipe 99 / target 9: before accepted; after `InvalidPayload` with unchanged bytes (probe `invalid_begin_goal_target_recipe`). Class-9 new credit: before completion succeeded, stock minted, kinds 4/7; after restore `OK`, boundary `InvalidLeg`, bytes unchanged (probe `new_credit_bad_class`: `RESTORE OK`, `DRIVE-0 InvalidLeg`, state hash equals input).

**Status:** **CLOSED.**

---

### 5.9 H08 — Scenario fields silently discarded — CLOSED

**Original defect.** Decode validated only the scenario content hash while skipping the recipe table, replaced the recipe and weights with hardcoded constants, and preserved only `wake_interval`. A checksum/content-consistent altered blob was accepted and rewritten on save, changing content identity.

**Correction.**
- Decode retains the entire scenario block bytes and requires the content hash to match.
- The scenario must equal the supported standard block for `wake_interval` `4` or `100`; a consistent altered recipe or weight matrix returns `ContentMismatch`.
- Supported standard images round-trip byte-identically; no accept-and-replace path remains.

**Regression test.** `h08_supported_scenarios_roundtrip_unsupported_consistent_content_is_rejected`.

**Expected vs actual.** `scenario_recipe_lost` and `scenario_weights_lost`: before `OK` with rewritten recipe/weights; after `ContentMismatch`. Supported wake 4 and wake 100 images: `restore` then `domain_checkpoint_bytes()` equals input.

**Status:** **CLOSED.**

---

### 5.10 H09 — Applied non-completion key reuse — BLOCKED

**Original defect / specified behavior.** Spec §5 row 7 and owner binding condition 4 require `ConflictingIdentity` when an applied `CommandKey` is reused with a different command type or payload. The baseline returned `LateTarget` for a reused `SetEnabled(1)` key because only the consumed completion registry is durable.

**Why no correction was applied.** The H09 owner direction requires all of:

1. preserve the 41-byte consumed-registry record;
2. preserve checkpoint revision `1`;
3. no durable history for non-completion applied keys;
4. non-completion reuse follows the ordinary late-target rules.

Binding condition 4 and spec §5 row 7 require the opposite: durable knowledge of every applied key, including type and payload, so a later reuse can return `ConflictingIdentity`. The revision-1 body has no such history. The counterexample is irreducible: after tick 1, an empty boundary and a `SetEnabled(1)` boundary that changed nothing have **identical canonical bytes**; no restore-time validator can distinguish their histories. Adding a hidden cache would violate resume equivalence, and adding a body field would change the revision-1 schema and published blobs, contrary to the H09 direction and the frozen-byte requirement.

Therefore H09 is stopped, as authorized, without changing code or documents. Current behavior remains: valid save/restore, then reused key returns `LateTarget` on the ordinary late-target rule (`applied_noncompletion_key_reuse`: `RESTORE OK`, `DRIVE-1 LateTarget`, bytes unchanged).

**Required owner resolution (one of):**
- narrow binding condition 4 and §5 row 7 to keys whose history is durably represented (consumed completion registry, pending, applied completion transaction); or
- approve a checkpoint schema/revision change carrying general applied-key identity history, with new published vectors and a revised H09 constraint.

**Status:** **BLOCKED — no unilateral code or contract change.**

---

### 5.11 H10 — Goal-0 transition inconsistency — CLOSED

**Original defect.** Spec §8 rule 9 said every `Executing` task advances to `Completing`, while C03-T01d requires goal-0 tick 2 to remain event-free. The implementation selected the event-free policy for all goal-0 tasks with no normative authorization and no way to ever complete a scripted goal-0 task with an admitted completion request.

**Correction (normative and code).**
- `docs/C0_3_IMPLEMENTATION_SPEC.md` §8 rule 9 is split:
  - **9a:** `Executing`, goal 0, empty unconsumed list → no change, no event.
  - **9b:** `Executing`, goals 1–5, or goal 0 with a non-empty unconsumed list → `Completing`, one kind-7 event, no legs.
- New §8.1 states that goal 0 is scripted, not autonomous; completion commands only append provenance in the command phase; a request promotes the future transition once, then rule 10 applies on a later tick; no `due_tick` extension; creation-tick skip and overdue rules precede; invalidation may preempt; goals 1–5 keep the unconditional transition; no hidden fields and no checkpoint-byte change.
- `step_task` implements 9a/9b exactly. Rule 10 stays: request or due tick, with creation-tick skip first.

**Regression tests.** `h10_goal_zero_request_advances_once_then_applies_after_reload`, `h10_deadline_request_has_one_transition_no_extension_and_invalidation_can_win`, `h10_goal_zero_waiting_and_goals_one_through_five_have_distinct_rules`, `h10_creation_skip_due_path_and_late_request_do_not_bypass_deadline`.

**Expected vs actual.**

| Case | Before | After |
| --- | --- | --- |
| goal-0, no request, tick 2 | no event (correct for T01d) but never authorized | no event; stated by 9a/§8.1 |
| goal-0, request, first Executing tick | request sat forever | one kind-7 transition to Completing; no legs that tick |
| goal-0, Completing, later tick | could not be reached | rule 10 applies legs, releases, Done; survives reload |
| request at due tick | untested | advances to Completing, cannot complete same tick; next overdue boundary `InvalidState` |
| invalidation vs request | ambiguous | invalidation wins first, releases once, role-2 provenance |
| goals 1–5 | transitioned | still unconditional |
| T01a tick-1 blob | `11f9aae4…` | unchanged |
| C03-T01d tick 2 | no event | unchanged |

**Evidence.** Probe `goal0_continuation` still shows event-free tick 2/tick 3 from the T01a image and `InvalidState` at the overdue tick; probe `pending_completions_restore` shows the request-driven completion path after reload. T01a forward/reverse state hash unchanged.

**Status:** **CLOSED.**

---

## 6. Documentation changes

`docs/C0_3_IMPLEMENTATION_SPEC.md`:
- Adds the S1 H10 clarification authorization note at the top.
- Replaces §8 rule 9 with 9a/9b.
- Adds §8.1 (goal-0 scripted semantics, deadline behavior, invalidation precedence, persistence, no new fields).

`docs/C0_3_ACCEPTANCE_TESTS.md`:
- C03-T01b initial-state wording now says `progress_ticks 2` is preinstalled and that this is an explicitly installed snapshot. Fields, cursors, events, and expected checkpoint values are unchanged.

`C0_3_OWNER_DECISION_RECORD.md` was not modified. It is the source of the H09 conflict and still records binding condition 4 as written.

---

## 7. Fixture and test adjustments

1. **C03-T01b/T01c fixture.** `install_reservation_stock` now inserts the stock referenced by the test's reservation. B01 correctly rejects a dangling reservation, so the old setup was not a valid revision-1 image. Event expectations and asserted reasons are unchanged.
2. **`pending_completions_restore`.** The audit's tick-1 save contains task 1 goal 0 stage 5, created 0, due 3, and two unconsumed completion keys. It is a valid image under the clarified contract. The validator does not invent a `target_tick == created` rule for submitted-0 provenance, so this image now restores and reproduces the baseline event stream.
3. **`new_credit_bad_class`.** Restore is intentionally `OK` because the class-9 leg is a planned declaration; the applying boundary returns `InvalidLeg` and leaves every byte unchanged. This preserves the approved later-leg rollback test while preventing publication of a class-9 stock.
4. **New suite.** `src/tests/c03_remediation.rs` adds 26 focused regressions covering B01 and H01–H08/H10, plus rollback and reload invariants. `src/tests/mod.rs` registers it.

---

## 8. Independent audit corpus re-run

The external audit harness was run against the corrected tree in both profiles.

| Metric | Result |
| --- | --- |
| Adversarial cases executed | 99 in debug, 99 in release |
| Unchanged vs baseline observation | 31 |
| Changed vs baseline observation | 68 |
| Debug/release divergence | none |
| Named restore expectations | **57/57 PASS** (baseline 6/57) |
| Target panics in corrected run | 0 |
| Baseline target panics | 1 (`missing_task_actor`) |

The 68 behavioral changes are the intended corrections and the accompanying contract clarification; they are enumerated case-by-case in `adversarial-comparison-debug.txt` and `-release.txt`, which are byte-identical.

---

## 9. Mandatory verification

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | exit 0 |
| `cargo check --locked --offline` | exit 0 |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0 |
| `cargo test --locked --offline` | 57 unit + 2 doctests passed |
| `cargo test --release --locked --offline` | 57 unit + 2 doctests passed |

Test composition: 14 C0.1 tests + 11 C0.2 tests + 6 C03 S1 tests + 26 remediation tests = 57 unit tests. Full transcript: `C0_3_S1_REMEDIATION_VERIFICATION_LOG.txt`.

---

## 10. Changed files

| File | Status |
| --- | --- |
| `src/fixture3.rs` | modified |
| `src/fixture3/validation.rs` | added |
| `src/tests/c03_remediation.rs` | added |
| `src/tests/c03_s1.rs` | modified (valid fixture only) |
| `src/tests/mod.rs` | modified (module registration) |
| `docs/C0_3_IMPLEMENTATION_SPEC.md` | modified (H10 clarification) |
| `docs/C0_3_ACCEPTANCE_TESTS.md` | modified (C03-T01b prose only) |
| `C0_3_S1_REMEDIATION_REPORT.md` | added (this report) |
| `C0_3_S1_REMEDIATION_VERIFICATION_LOG.txt` | added |
| `C0_3_S1_FINDING_CLOSURE_MATRIX.md` | added |

Not modified: `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `src/domain.rs`, C0.1/C0.2 tests, `C0_3_OWNER_DECISION_RECORD.md`.

---

## 11. Residual risk and limits

- **H09 is not closed.** Reused non-completion keys still return `LateTarget`; this is a deliberate stop, not an oversight. S1 sign-off should not be declared until the owner resolves the history requirement.
- The correction validates fixture-3 S1 states only. It does not implement S2 decisions, needs, scoring, route search, autonomous goal selection, or S4 actor matrices.
- The codec now preserves typed `PathStep` values, transaction `applied`, and 320-byte trace tails, but S1 still does not produce traces or navigate paths. That future work remains outside S1.
- The 41-byte registry and revision `1` were preserved. No new field was added to the checkpoint body.
- The remediation is committed locally on `fix/c03-s1-independent-audit`; it was not pushed and not merged. `/main` is untouched.
