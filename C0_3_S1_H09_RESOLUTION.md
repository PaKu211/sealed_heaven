# C0.3 S1 H09 Resolution

**Finding:** S1-H09 — applied-key rule required owner reconciliation with checkpoint inventory.
**Decision:** H09 Option B, approved by the project owner on 17 November 2026.
**Branch:** `fix/c03-s1-independent-audit`
**Baseline:** `552ed25`
**Status:** **CLOSED** by the owner amendment and verification below. Independent S1 re-audit is requested; no S1 PASS is declared.

---

## 1. Problem being resolved

Spec §5 row 7 and the original binding condition 4 required `ConflictingIdentity` when **any** applied `CommandKey` was reused with a different command type or payload. The revision-1 checkpoint body has no general applied-command identity table. It stores only:

- the 41-byte consumed completion registry,
- pending commands,
- applied completion transactions,
- task, stock, reservation, and completion provenance.

A no-op non-completion command can leave canonical state byte-identical to an empty boundary. For example, `SetEnabled(1)` on an already-enabled actor emits nothing and changes no field. After restore, the two histories are indistinguishable. Requiring durable recognition of every applied non-completion key would therefore require a new body field and a checkpoint revision change, contradicting the owner's H09 constraints.

---

## 2. Options considered

| Option | Description | Result |
| --- | --- | --- |
| A | Add a general durable applied-key registry, bump `SH:C03:STATE` revision, publish new vectors | Rejected: changes the revision-1 schema and published canonical bytes; requires history the owner excluded |
| B | Narrow identity to durable records, define deterministic late-target behavior, and forbid hidden caches | **Approved and implemented** |

Option B preserves revision 1, the 41-byte completion registry, all published vectors, and resume equivalence.

---

## 3. Approved contract

The owner-approved rules are recorded in `C0_3_OWNER_DECISION_RECORD.md` §2A:

1. Consumed `CompleteTask` identities remain durable through the existing 41-byte completion registry.
2. Pending command identities retain their existing conflict rules.
3. Duplicate or conflicting commands within one admission follow the normative first-match precedence table.
4. Ordinary non-completion commands that have already been applied do not require a separate persistent historical registry.
5. After such a command has been applied and is no longer present in canonical state, a later submission with an old target follows `LateTarget`, subject to earlier applicable structural validation.
6. Save and restore must produce identical future behavior from identical canonical checkpoint bytes.
7. No hidden noncanonical replay cache is permitted.
8. `SH:C03:STATE` revision 1, the 41-byte completion registry, and all published canonical vectors are preserved.

Consequences:

- **Durable identity** means a consumed completion registry record or a pending command.
- An applied non-completion key with an **old target** reaches row 10 (`LateTarget`) after rows 2–4 and the durable-identity checks.
- An applied non-completion key with a **future target** is an ordinary new command. There is no historical identity to conflict with, so it is admitted if the ordinary payload and task rules pass.
- A no-op applied command and an empty boundary with identical canonical bytes have identical future behavior.

---

## 4. Normative document changes

| File | Change |
| --- | --- |
| `C0_3_OWNER_DECISION_RECORD.md` | Version 2.1; condition 4 narrowed to durable identities; new §2A records the H09 amendment, the eight approved rules, and the authorization boundaries |
| `docs/C0_3_IMPLEMENTATION_SPEC.md` | §5 row 7 now covers only the consumed registry; a following paragraph defines durable identities, old-target `LateTarget`, future-target new commands, and the no-hidden-cache rule; top note records the H09 clarification |
| `docs/C0_3_ACCEPTANCE_TESTS.md` | New §9A with positive old-target replay, negative structural cases, future-target admission, save/restore equivalence, and the identity-free checkpoint assertion; coverage row added |

No C0.1/C0.2 document or vector was changed.

---

## 5. Rust behavior verification

**No production Rust change was required.** The existing implementation already follows the narrowed contract:

- `drive` performs batch-wide producer, unknown-actor, and duplicate/pending checks before per-command classification.
- `classify` consults only the completion registry for durable command identity, then checks pending, cancel rules, and `LateTarget`.
- `apply_command` applies ordinary commands without writing a non-completion history record.
- `restore` reconstructs only the canonical body; it has no hidden replay cache.

Two focused regressions were added to `src/tests/c03_remediation.rs`:

- `h09_applied_noncompletion_replay_is_late_and_identity_free`
- `h09_pending_duplicate_and_future_target_follow_first_match`

They pass in debug and release.

---

## 6. Case matrix

Initial state: shared two-claimant fixture, tick 0, cursor 1, digest H0, actor 1 enabled, task 1 non-terminal.

| Case | Input | Expected | Observed |
| --- | --- | --- | --- |
| Applied no-op | Boundary 0: target 1, `SetEnabled(1)` | No event; tick 1, cursor 1, digest H0 | No event; tick 1, cursor 1, digest H0 |
| Positive old-target replay | Boundary 1: same key, `CancelReservation(0)` | `LateTarget`; bytes unchanged | `LateTarget`; bytes unchanged |
| Save/restore replay | Save after boundary 0, restore, repeat | Same `LateTarget`; same bytes | Same `LateTarget`; same bytes |
| Identity-free checkpoint | Empty boundary at tick 0 vs applied no-op | Byte-identical checkpoints; same replay result | Byte-identical; same `LateTarget` |
| Producer precedence | Old-target replay with producer 0 | `InvalidProducer`; no mutation | `InvalidProducer`; unchanged bytes |
| Actor precedence | Old-target replay with unknown actor | `UnknownActor`; no mutation | `UnknownActor`; unchanged bytes |
| Admission duplicate | Same key twice in one batch | `ConflictingIdentity`; no mutation | `ConflictingIdentity`; unchanged bytes |
| Pending identity | Future key submitted again before due | `ConflictingIdentity`; no mutation | `ConflictingIdentity`; unchanged bytes |
| Future target | Same key, target 2, `SetMode(Scheduled)` | New command admitted; applies at tick 2; no event | Admitted; mode Scheduled; no event |
| Applied future key | Re-submit that target-2 key at tick 2 | `LateTarget`; bytes unchanged | `LateTarget`; unchanged bytes |
| Durable completion | Consumed `CompleteTask` replay | §7 behavior unchanged | §7 tests pass |

External audit evidence: `applied_noncompletion_key_reuse` restores `OK`, then returns `LateTarget` with unchanged state hash and length. This is the amended contract, not a failure. The prior audit expected `ConflictingIdentity`; that expectation is superseded for fixture 3.

---

## 7. Preservation

| Invariant | Result |
| --- | --- |
| Checkpoint revision | `1` |
| Consumed registry width | 41 bytes |
| Scenario block | 305 bytes |
| Actor width | 131 bytes |
| T01a blob | `11f9aae46d7f0f04b6c3efead0a25c3086bc56904877d237c0a79ae032a293d9`, length 1262 |
| T01a digest | `5fdbc5707211fbfc309f5ed75e21eaa83ebf3d7307da60cc442a3cd7f8340908` |
| H0 | `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c` |
| C0.1/C0.2 vectors | unchanged; 25 tests pass |
| Dependencies / toolchain | unchanged; `Cargo.lock` not modified; `1.100.0` pinned |

---

## 8. Authorization boundaries

This correction:

- does not start S2, S3, S4, or Bevy;
- does not push or merge;
- does not alter dependencies, `Cargo.lock`, or the toolchain;
- does not modify frozen C0.1/C0.2 vectors;
- does not declare an independent S1 PASS.

**Next step:** independent S1 re-audit of the amended contract, the H09 cases, and the unchanged canonical vectors.
