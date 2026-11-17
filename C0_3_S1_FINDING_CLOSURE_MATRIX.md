# C0.3 S1 Finding Closure Matrix

**Baseline:** `552ed25`
**Branch:** `fix/c03-s1-independent-audit`
**Date:** 17 November 2026
**Reference audit:** `sealed-heaven-s1-audit-552ed25/C0_3_S1_INDEPENDENT_AUDIT.md`
**Scope:** B01 and H01–H10 only. No S2/S3/S4, no merge, no push, no S1 PASS declaration.

> **H09 status update (17 November 2026):** H09 was subsequently resolved by the owner's Option B amendment. See `C0_3_S1_H09_RESOLUTION.md` and `C0_3_S1_FINAL_REMEDIATION_REPORT.md`. The H09 row below is historical.

---

## 1. Disposition summary

| Finding | Severity | Status | Closing test(s) |
| --- | --- | --- | --- |
| B01 | Blocker | **CLOSED** | 6 `b01_*` regressions; 57/57 restore expectations |
| H01 | High | **CLOSED** | 2 `h01_*` regressions |
| H02 | High | **CLOSED** | `h02_same_phase_and_future_begin_cannot_orphan_tasks` |
| H03 | High | **CLOSED** | 2 `h03_*` regressions |
| H04 | High | **CLOSED** | 3 `h04_*` regressions |
| H05 | High | **CLOSED** | `h05_producer_and_actor_checks_precede_duplicate_and_late_keys` |
| H06 | High | **CLOSED** | 2 `h06_*` regressions |
| H07 | High | **CLOSED** | `h07_schema_invalid_begin_and_new_stock_class_do_not_publish` |
| H08 | High | **CLOSED** | `h08_supported_scenarios_roundtrip_unsupported_consistent_content_is_rejected` |
| H09 | High | **BLOCKED** | none — owner conflict, no code change |
| H10 | High | **CLOSED** | 4 `h10_*` regressions |

Totals: **10 closed / 1 blocked / 0 unresolved silently**. All 57 unit tests and 2 doctests pass in debug and release.

---

## 2. Matrix

### B01 — Restore publishes semantically invalid worlds

| Field | Content |
| --- | --- |
| **Original defect** | `restore` inserted decoded rows directly into maps and returned a world with no semantic validation. Zero/reused cursors, cross-namespace ID collisions, unsorted/duplicate rows, dangling links, zero/duplicate registry rows, and malformed reservations were accepted. Some blobs changed bytes on load/save; `missing_task_actor` restored and panicked in `drive`. |
| **Source change** | New `src/fixture3/validation.rs` validates cursors, namespaces, ordering, links, domains, reservations, transactions, registry, pending, and traces after parsing and before returning a `Simulation`. `src/fixture3.rs` decode no longer normalizes duplicates; bounds and ordering are tracked during parse. Planned legs remain declarations and are not rejected for work-time class/owner/stock/quantity. No registry width or revision change. |
| **Regression tests** | `b01_cursors_links_and_domains_are_rejected_before_publication`; `b01_duplicate_and_unsorted_tables_are_not_normalized`; `b01_reservations_and_registry_are_validated`; `b01_completion_registry_transactions_and_all_roles_roundtrip`; `b01_duplicate_stock_registry_pending_and_namespace_collision`; `b01_codec_preserves_variable_width_paths_and_validates_shape_without_navigation` |
| **Expected vs actual** | `record_cursor_zero`: expected `InvalidState`, before `OK`, after `InvalidState`. `duplicate_stock`: expected `InvalidState`, before `OK` with stock overwrite, after `InvalidState`. `missing_task_actor`: expected `InvalidState`, before `OK` + panic, after `InvalidState`. `grid_single_zero`: expected `Corrupt`, before `OK`, after `Corrupt`. Valid `pending_completions_restore`: before rejected by no committed rule, after `OK` with byte-identical roundtrip. |
| **Evidence** | `audit-probe-debug.txt`, `audit-probe-release.txt`, `adversarial-comparison-debug.txt`, `adversarial-comparison-release.txt`; `restore-expectations.tsv` 57/57 PASS; `final-verification.txt`. |
| **Final status** | **CLOSED** |

### H01 — Count/grid guards and restore precedence

| Field | Content |
| --- | --- |
| **Original defect** | Declared counts were allocated/parsed before minimum-size validation, so `u64::MAX` produced `Truncated` instead of `Corrupt`. Grid `4097x1` and `0x3` restored. A 12-byte header returned `UnsupportedFormat`. Fixture/content checks ran before complete-body/trailing-byte validation. |
| **Source change** | `Reader::count(minimum)` rejects `u64::MAX`, overflow, and counts whose minimum size exceeds remaining bytes. Grid dimensions must be `<= 4096`, both zero or both non-zero, consistent with `grid_site`, and fit in the remaining body. `restore` checks truncation before tag and checksum before decode; decode checks trailing bytes before fixture/content semantics. |
| **Regression tests** | `h01_counts_header_grid_and_parse_before_semantics`; `h01_truncations_bad_framing_and_content_precedence_never_publish` |
| **Expected vs actual** | `count_max_actors` (and 8 other sections): expected `Corrupt`, before `Truncated`, after `Corrupt`. Grid `4097x1`: expected `Corrupt`, before `OK`, after `Corrupt`. Grid `0x3`: expected `Corrupt`, before `OK`, after `Corrupt`. 12-byte header: expected `Truncated`, before `UnsupportedFormat`, after `Truncated`. |
| **Evidence** | Probe `count_max_*`, `grid_exceeds_body`, `grid_single_zero`; `tests::h01_*`; `final-verification.txt`. |
| **Final status** | **CLOSED** |

### H02 — Multiple BeginTask commands bypass single-task ownership

| Field | Content |
| --- | --- |
| **Original defect** | Batch admission tracked only actor IDs created in the phase, and execution did not recheck ownership. Two BeginTask commands for one idle actor both created tasks; the future variant admitted two later Begins. |
| **Source change** | Batch-wide pre-scan precedes per-command checks. Scheduled Begins are keyed by `CommandKey`; a Begin for the same actor and `target_tick` is rejected. `apply_command` rechecks `actor.task_id == 0`; `apply_due` rechecks created-task identity. |
| **Regression test** | `h02_same_phase_and_future_begin_cannot_orphan_tasks` |
| **Expected vs actual** | Same-phase duplicate: expected rejection and no illegal publication; before two tasks/kind-7 events; after `InvalidState` and unchanged bytes. Future two Begins: expected no second task; after first Begin admitted, second execution `InvalidState` with unchanged bytes (probe `future_two_begins` DRIVE-2). |
| **Evidence** | `tests::h02_*`; probe `two_begins_same_phase` DRIVE-0 `InvalidState`; probe `future_two_begins` DRIVE-2 `InvalidState`. |
| **Final status** | **CLOSED** |

### H03 — CompleteTask(0) does not require same-phase creation

| Field | Content |
| --- | --- |
| **Original defect** | Zero completion was accepted without a Begin, and a Begin targeting another tick could authorize it. Admission ignored whether a task actually existed and later existed. |
| **Source change** | Admission requires an earlier scheduled Begin for the same actor and same `target_tick`. Execution tracks the task created in the phase and requires the zero completion to resolve to it. `apply_command` requires a current non-terminal task. Restore validates the pending same-phase dependency without inventing a `target_tick == created` rule. |
| **Regression tests** | `h03_zero_completion_requires_an_actual_same_phase_begin`; `h03_future_same_phase_dependency_survives_restore_and_requires_successful_creation` |
| **Expected vs actual** | `complete_zero_without_begin`: expected `InvalidPayload`, before `OK` + provenance on old task, after `InvalidPayload` + unchanged bytes. `zero_complete_cross_tick_begin`: expected `InvalidPayload`, before `OK`, after `InvalidPayload`. Valid create-tick zero completion still applies on a later tick and reloads. |
| **Evidence** | `tests::h03_*`; probe `complete_zero_without_begin`, `zero_complete_cross_tick_begin`; `pending_completions_restore` roundtrip. |
| **Final status** | **CLOSED** |

### H04 — Repeated own reservations exceed on-hand and overflow

| Field | Content |
| --- | --- |
| **Original defect** | Reservation availability excluded all own-task holds, so two own reservations could total more than on-hand. Aggregate overflow was unchecked. Invalid state survived only because B01 accepted it. |
| **Source change** | `held()` sums all active holds with `checked_add` and an optional task exclusion. `reserve` preflights each requested stock against on-hand and `held + requested` before events, then uses checked subtraction for availability. `apply_leg` uses the same checked aggregate. |
| **Regression tests** | `h04_own_reservations_share_capacity_and_checked_aggregate`; `h04_multiple_own_holds_fit_then_debit_and_release_exactly_once`; `h04_bundle_contention_has_one_rejection_no_partial_allocation` |
| **Expected vs actual** | Two own 1-unit holds on 1 unit: expected at most one active hold; before two holds totaling 2; after one hold, second rejected by kind-2 event, events `[1,7,2,7]`, one reservation. Two `u64::MAX` requests: expected `CapacityExceeded`; before accepted overflow; after `CapacityExceeded` with full rollback. Valid multiple holds still fit, debit, and release once. |
| **Evidence** | `tests::h04_*`; probe `same_task_double_reserve` events `[1,7,2,7]`; probe `same_task_reservation_sum_overflow` `CapacityExceeded` unchanged. |
| **Final status** | **CLOSED** |

### H05 — Admission precedence is not the approved first-match table

| Field | Content |
| --- | --- |
| **Original defect** | Duplicate-key scan ran before producer/actor checks; lower-key late command could hide a higher-key producer 0. Error identity was wrong although rollback worked. |
| **Source change** | `drive` performs batch-wide `InvalidProducer` → `UnknownActor` → `ConflictingIdentity` checks before per-command replay/late classification. Fixture 1/2 behavior untouched. |
| **Regression test** | `h05_producer_and_actor_checks_precede_duplicate_and_late_keys` |
| **Expected vs actual** | Duplicate producer 0: expected `InvalidProducer`, before `ConflictingIdentity`, after `InvalidProducer`. Producer-0 plus late target: expected `InvalidProducer`, before `LateTarget`, after `InvalidProducer`, rollback unchanged. |
| **Evidence** | `tests::h05_*`; probe `producer_precedence_across_keys` → `InvalidProducer`; probe `invalid_producer_duplicate`. |
| **Final status** | **CLOSED** |

### H06 — Invalidation ordered by task ID, not actor then task

| Field | Content |
| --- | --- |
| **Original defect** | Invalidation iterated task IDs only, so actor order could invert when task IDs crossed actors. |
| **Source change** | Invalidation sorts by `(actor_id, task_id)`; release order within a task remains ascending reservation ID. |
| **Regression tests** | `h06_invalidation_orders_actors_not_record_ids`; `h06_disable_cancel_expiry_preempt_completion_and_replays_remain_silent` |
| **Expected vs actual** | Cross-owned Queued tasks, both actors disabled: expected actor-1 kind 7/kind 5 then actor-2 kind 7/kind 5; before actor-2 first; after exactly `[(1,7),(1,5),(2,7),(2,5)]`. |
| **Evidence** | `tests::h06_*`; probe `invalidation_actor_order` event order `1:1:1:7`, `2:1:1:5`, `3:1:2:7`, `4:1:2:5`. |
| **Final status** | **CLOSED** |

### H07 — Successful commits publish out-of-schema records

| Field | Content |
| --- | --- |
| **Original defect** | BeginTask checked only stage/leg-count/actor locus; goal 255, recipe 99, target 9 were stored. New-stock credit validated owner but not class, so class 9 could be minted. |
| **Source change** | Full Begin payload shape validation (goal, recipe, target, leg count, declared legs) at admission and execution; class `1|2` and non-zero definition for every applied leg; owner existence; checked availability. Planned declarations remain restorable and fail the applying boundary with `InvalidLeg`. |
| **Regression test** | `h07_schema_invalid_begin_and_new_stock_class_do_not_publish` |
| **Expected vs actual** | Invalid Begin: expected rejection, before stored, after `InvalidPayload` with unchanged bytes (probe `invalid_begin_goal_target_recipe`). Class-9 new credit: expected `InvalidLeg` and no publication, before success with class-9 stock, after restore `OK` + boundary `InvalidLeg` + byte-identical state. |
| **Evidence** | `tests::h07_*`; probe `new_credit_bad_class` (`RESTORE OK`, `DRIVE-0 InvalidLeg`, state hash equals input). |
| **Final status** | **CLOSED** |

### H08 — Restore silently discards scenario fields

| Field | Content |
| --- | --- |
| **Original defect** | Decode skipped the recipe table, used hardcoded recipe/weights, and preserved only `wake_interval`. A consistent altered scenario was accepted and rewritten on save. |
| **Source change** | Decode retains the full scenario block and requires content hash match plus exact equality to the supported standard block for wake `4|100`. Altered consistent content returns `ContentMismatch`. |
| **Regression test** | `h08_supported_scenarios_roundtrip_unsupported_consistent_content_is_rejected` |
| **Expected vs actual** | Altered recipe: expected rejection or faithful preservation; before `OK` + rewritten identity; after `ContentMismatch` (probe `scenario_recipe_lost`). Altered weights: before `OK` + rewritten matrix; after `ContentMismatch` (`scenario_weights_lost`). Valid wake 4 and 100 images round-trip byte-identically. |
| **Evidence** | `tests::h08_*`; probe `scenario_recipe_lost`, `scenario_weights_lost`. |
| **Final status** | **CLOSED** |

### H09 — Applied-key rule requires owner reconciliation

| Field | Content |
| --- | --- |
| **Original defect** | Spec §5 row 7 and binding condition 4 require `ConflictingIdentity` on reuse of any applied key with a different type/payload. The implementation only has durable completion provenance, so a reused non-completion key returns `LateTarget`. |
| **Source change** | **None.** Applying the specified behavior would require durable general applied-key history; applying the H09 direction preserves the status quo. |
| **Regression test** | None added. The external audit case `applied_noncompletion_key_reuse` remains the evidence. |
| **Expected vs actual** | Specified (condition 4/row 7): `ConflictingIdentity`. H09 owner direction: ordinary late-target behavior, no durable history. Current actual: `RESTORE OK`, reused key `DRIVE-1 LateTarget`, bytes unchanged. |
| **Evidence** | Probe `applied_noncompletion_key_reuse`; audit finding S1-H09; `C0_3_OWNER_DECISION_RECORD.md` §2 condition 4; spec §5 row 7. |
| **Final status** | **BLOCKED — owner resolution required; no code or document change made.** |

**Exact blocking conflict.**

- `C0_3_OWNER_DECISION_RECORD.md` §2 condition 4: "Reusing an applied `CommandKey` with a different command type or payload returns `ConflictingIdentity`." This is unconditional and binds the specification.
- `docs/C0_3_IMPLEMENTATION_SPEC.md` §5 row 7: the same requirement, additionally covering "any applied command key".
- The authorized H09 direction requires: preserve the 41-byte registry, preserve checkpoint revision 1, no durable history for non-completion applied keys, and ordinary late-target handling for those keys.
- The revision-1 checkpoint body has no general applied-command identity table. An empty boundary and a no-op `SetEnabled(1)` boundary produce identical canonical bytes, so the required history cannot be reconstructed after restore. A hidden cache violates resume equivalence; a new body field violates the H09 direction and changes revision-1 blobs.

Because the two binding sources cannot both be satisfied without an owner decision, H09 was stopped alone. B01 and H10 were not used to reinterpret it, and no registry or revision change was made.

### H10 — Goal-0 transition limitation not authorized

| Field | Content |
| --- | --- |
| **Original defect** | Spec §8 rule 9 required every Executing task to advance to Completing; C03-T01d required goal-0 tick 2 to stay event-free. The code chose event-free for all goal-0 tasks, with no way for an admitted request to ever complete the task. The audit declined to choose unilaterally. |
| **Source change** | `docs/C0_3_IMPLEMENTATION_SPEC.md`: rule 9 split into 9a/9b and new §8.1. `step_task`: goal-0 Executing no-ops only with an empty unconsumed list; a non-empty list advances once to Completing with one kind-7 event. Rule 10 and creation-tick skip unchanged. No new fields, no checkpoint byte change, no `due_tick` extension. |
| **Regression tests** | `h10_goal_zero_request_advances_once_then_applies_after_reload`; `h10_deadline_request_has_one_transition_no_extension_and_invalidation_can_win`; `h10_goal_zero_waiting_and_goals_one_through_five_have_distinct_rules`; `h10_creation_skip_due_path_and_late_request_do_not_bypass_deadline` |
| **Expected vs actual** | Goal-0 no request: expected no event; before no event (unauthorized); after no event under 9a/§8.1. Goal-0 request in Executing: expected one transition; before request never applied; after one kind 7, then rule 10 on a later tick, surviving reload. Request at due tick: expected no same-tick completion; after transition only, next overdue boundary `InvalidState`. Goals 1–5: unchanged unconditional transition. T01a tick-1 blob and C03-T01d tick 2 unchanged. |
| **Evidence** | `tests::h10_*`; probe `goal0_continuation` (tick 2/3 event-free, tick 4 `InvalidState`); probe `pending_completions_restore` (request-driven completion after reload); T01a state `11f9aae4…`. |
| **Final status** | **CLOSED** |

---

## 3. Preservation checks

| Check | Result |
| --- | --- |
| C0.1/C0.2 golden vectors | No source/test diff; all 25 C0.1/C0.2 tests pass in debug and release |
| `Cargo.toml` / `Cargo.lock` | No diff |
| Pinned toolchain | `1.100.0`, unchanged |
| Checkpoint revision | `1`, unchanged |
| Consumed registry width | 41 bytes, unchanged |
| T01a tick-1 body | `11f9aae46d7f0f04b6c3efead0a25c3086bc56904877d237c0a79ae032a293d9`, length 1262 |
| T01a digest | `5fdbc5707211fbfc309f5ed75e21eaa83ebf3d7307da60cc442a3cd7f8340908` |
| H0 | `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c` |
| Determinism | Integer-only sim, `BTreeMap`/`BTreeSet`, explicit sorts; no float/clock/random/thread dependence |
| Merge/push | None performed |

---

## 4. Blocked item register

| Item | Blocking reason | Required decision | Work performed |
| --- | --- | --- | --- |
| H09 | Binding condition 4 / spec §5 row 7 require durable applied-key identity; revision-1 body cannot represent non-completion history without a schema change, and the H09 direction forbids that change | Narrow condition 4/row 7 to durably represented keys, **or** approve a revision/schema change with general applied-key history and new vectors | Documented exact conflict; left current late-target behavior; made no code or document change |
