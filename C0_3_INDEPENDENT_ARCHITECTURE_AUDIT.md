# C0.3 Independent Architecture Audit (G3)

Version 1.0 · 17 November 2026  
Reviewer role: independent architecture and specification audit.  
Status: read-only. No source, golden vector, or C0.3 specification file was modified. Implementation is not authorized.

## 1. Verdict

**FAIL**

The C0.3 draft is honest about its gaps and does not invent need curves, path costs, or golden bytes. It is not complete, not internally closed, and not implementation-ready. Core §7 requires keys and diagnostics to be frozen before production behavior is written. Those keys are still open. The README deliverable, Technical Architecture, and Core assign needs, jobs, and navigation in language the draft cannot execute. Several acceptance sentences contradict one another. Project-owner decisions D1 through D6, D9, and D11 block the exit named by the README.

A later packet can become reviewable again only after the owner records the decisions in `C0_3_OWNER_DECISION_MATRIX.md` and the specification and acceptance files are revised to match. That revision is a separate authorization. This audit does not apply it.

## 2. Evidence examined

| Document | Use in this audit |
| --- | --- |
| `README.md` gate table | C0.3 deliverable and exit column |
| `docs/CORE_SIMULATION_SPEC.md` §§1–12 | Normative ledger, tasks, navigation, persistence, invariants |
| `docs/SIMULATION_LOD_SPEC.md` §§2–5 | Mode, accounting, deferred activation |
| `docs/ARCHITECTURE_DECISIONS.md` ADR-003, ADR-009, ADR-010, ADR-013, ADR-017, ADR-021 through ADR-024 | Binding scope limits |
| `docs/TECHNICAL_ARCHITECTURE.md` §§2, 3, 6, 8, 10 | C0.3 ownership and serial mutation |
| `docs/GDD_v0.2.md` | Pipeline and fairness caution |
| `docs/M0_ACCEPTANCE_TESTS.md` §§1, 3, 6 | C03-T01 and M0-T06 through M0-T08 |
| `docs/C0_2_IMPLEMENTATION_SPEC.md` §§7–9 | Frozen phases, immutable enabled flags, synthetic completion |
| `C0_2_BASELINE_MANIFEST.md` | Approved archive and golden identities |
| `docs/C0_3_IMPLEMENTATION_SPEC.md` | Proposed design |
| `docs/C0_3_ACCEPTANCE_TESTS.md` | Proposed tests |
| `docs/C0_3_ARCHITECTURE_REVIEW.md` | Author's own readiness claim |

The approved C0.2 archive recorded by the baseline is `Sealed_Heaven_C0_2_Source.zip`, SHA-256 `2804d33a785acb7331a0d82fd1c080bec82d0add348578abb4b1a6807db60a40`, independent audit PASS. This G3 review did not re-run that suite and does not reopen that verdict. R2 remains uncertified.

## 3. What the approved documents actually require

The gate texts do not describe one closed C0.3 scope.

| Source | What it assigns to C0.3 |
| --- | --- |
| README deliverable cell | Needs, jobs, inventory, and resource transactions |
| README exit cell | C03-T01 and the applicable M0-T06 through M0-T08 fixtures |
| Technical Architecture §2 | Needs, jobs, physical inventory and ledger, and initial navigation |
| Technical Architecture §10 | Resource-transaction and navigation fixtures |
| Core preamble | Production actor needs, tasks, and physical resources |
| Core §6 | Eat, rest, work, haul, and return home are C0.3/M0 goals. Stage names and DecisionTrace fields are named. Numeric scores are absent |
| Core §7 | C0.3 owns the production reservation lifecycle in C03-T01. Keys and diagnostics are defined before implementation |
| Core §8 | The two-site route graph is an M0 spatial model. Successor and equal-cost order are required when the first navigation fixture exists |
| M0 §6 C03-T01 | Two claimants, both orders, expiry, cancel, disable, replay, repeat release, failed later recipe leg, then the corresponding M0-T06/T07/T08 cases. Production fairness has its own ordered fixture |
| ADR-013 | One owner, atomic transactions, derived availability. Evidence is C03-T01 and M0-T06/T07/T08 |
| ADR-021 and Core §4 | The C0.1 command key is that fixture's policy. Production fairness is a separate decision |
| LOD §5 and ADR-009 | Production activation and coarse travel stay open until M0-T05 and an ADR-009 amendment |
| C0.2 spec §8 | Fixture 2 still completes a due synthetic task when the actor is disabled. The same section says C0.3 replaces that synthetic completion with the production invalidation rule |

The ledger sentences in Core §7 are normative and do not depend on a utility formula: one owner, reservation is not a second owner, available quantity is on-hand minus active reservations, interval `[acquired tick, expiry tick)`, same-tick disablement, cancellation, and expiry invalidate before completion, a repeated terminal release is a no-op, and a stable transaction identity applies each validated leg once. A failed later leg leaves earlier legs unapplied. Those rules are mandatory C0.3 behavior once a fixture exists. They are not yet an implementable fixture, because the keys and diagnostics Core §7 demands are missing.

Needs, jobs, and navigation are named. They are not specified to the level C0.1 and C0.2 required before coding: no channel, threshold, wake tick, score, stage duration, graph, cost, or successor order appears in any approved document. GDD line 41 names the pipeline and also says gameplay fairness needs its own rules before adoption. Treating the GDD pipeline as an algorithm would be an unapproved mechanic.

## 4. Scope classification

### 4.1 Mandatory C0.3

These are required by a C0.3 owner document and have a stated result, even where the encoding is still missing:

- C03-T01 two-claimant contention with identical results in both submission orders.
- Expiry, cancellation, and disablement at the completion tick transfer no stock and release once.
- Replay of a completion identity and a repeated release apply the effect at most once.
- A transaction whose later leg is invalid leaves balances, reservations, and success events unchanged.
- Core §7 conservation: one owner, derived availability, no negative stock, traceable legs.
- Core invariant 4 for production jobs: a disabled actor does not continue an invalidated job.
- Regression of every published C0.1 and C0.2 golden identity.

### 4.2 Supporting infrastructure the mandatory fixture needs

- A new canonical schema that can represent stock, reservations, tasks, and transaction identity.
- A total order and two diagnostics for the contention fixture.
- A replayable cancellation input.
- A replayable way to disable a claimant at the completion tick after that claimant has already acquired a reservation.
- Restore, rollback, and prefix/suffix replay of that schema.
- An independent oracle for any new golden bytes.

### 4.3 Deferred by an approved owner

| Item | Owner |
| --- | --- |
| First migration of an older save | M0-T11 |
| `ticks_per_game_day` and the 30-day horizon | M0, ADR-024 |
| Automatic processing-mode policy and travel approximation | M0-T05, ADR-009, LOD §5 |
| Anonymous cohorts | ADR-010 |
| Typed content JSON and remote schema checks | M0-T09 |
| Presentation, host cadence, Bevy | V0, I0 |
| Cross-platform bitwise replay | R2 |
| Headless timing budget | M0-T14 |
| Autonomous need/work causal scenario | M0-T13 |

### 4.4 Unapproved expansions

The draft correctly withholds these. They stay unapproved:

- A utility function, need curve, or DecisionTrace numeric vector.
- A pathfinder, route crate, or invented successor order.
- Changing fixture 1 or fixture 2 encodings, or applying invariant 4 retroactively to synthetic tasks.
- A new C0.1 `Outcome` variant used as a production diagnostic.
- Defaulting new fields into `SH:C02:STATE` revision 1.
- Idle-tick skipping.
- The 20-actor, two-site, 30-day world as a silent C0.3 fixture.

## 5. Architecture evaluation

### 5.1 Determinism

C0.1 admission remains a sound base: read-only preflight, whole-batch admit or a typed error, then commit in command-key order. C0.2 adds mode, command, and work. The draft's invalidation phase before work is a coherent reading of Core §7's same-tick rule. It is not yet an approved phase, and it is not bound to a per-fixture dispatch table. Fixture 1 and fixture 2 must keep their published event bytes. C03-R01 would detect a hash change. It does not by itself specify that the new phase is absent or a no-op for those fixtures.

Contention order is not frozen. ADR-021 and Core §4 forbid treating the C0.1 key as production fairness. C03-T01 still requires one order that is stable under reversal. Reusing the key only inside that fixture is a viable recommendation. It is not a specification until the owner accepts it and names the diagnostics.

Replay equivalence for the new fixture is described in C03-T01g and C03-T01i and cannot be executed. Rollback of a failed boundary is required and matches C0.2 publication discipline. The overflow error class is only "the same rollback class as C0.2", which covers more than one C0.2 error.

### 5.2 Inventory and ledger

Single ownership, derived availability, and atomic legs match ADR-013 and Core §7. The draft correctly says a reservation does not move the owner and does not create stock. Completion is the transfer.

Missing ledger rules that an implementer would have to guess:

- Whether a reservation may be a partial quantity of a stack larger than one.
- Whether quantity zero or an empty interval `[T, T)` is rejected.
- How a `u64` sum of reservations overflows.
- Whether a rejected reservation emits an event.
- Which owner location among actor inventory, container, site stockpile, in-transit cargo, and explicit reserve the first fixture uses.
- The debit and credit of a successful recipe, as opposed to the failed-later-leg case.

### 5.3 Production job lifecycle

Core §6 names `Queued → Reserved → Travelling → Executing → Completing → Done`, plus cancellation, interruption, and failure. The draft records those names and assigns no transition, duration, or initial stage. C03-T01 can be stated with a reservation and a completion without travel. That reduced lifecycle is not written down, so an implementer can either block on the full chain or invent stages.

Exactly-once release is normative. The event consequence of a second release is not, because the acceptance file disagrees with the spec. See G3-H04.

Disablement at the completion boundary cannot be represented by the C0.2 enabled flag. That flag does not change after construction (`C0_2_IMPLEMENTATION_SPEC.md`, persistence rules). A claimant who starts disabled is also forbidden, by the draft itself, from acquiring the reservation the tie test needs. See G3-B03.

### 5.4 Actor needs

No approved document defines a channel, threshold, decay, priority weight, or wake interval. DecisionTrace field names are mandatory when scoring exists. Scoring does not exist. C03-N01 is correctly blocked. It is not evidence that the README deliverable cell has been met. See G3-B02.

### 5.5 Navigation

No graph, tile cost, blocked-edge rule, movement duration, or failure diagnostic is approved. Core §8 places the two-site graph at M0 and requires successor tests when a navigation fixture exists. Technical Architecture §10 says C0.3 owns the navigation fixtures. M0-T08 includes "Unreachable targets fail" in the same row C03-T01 tells C0.3 to run before promotion. The draft blocks the path tests and omits the unreachable-target case. See G3-B02 and G3-H03.

Resource movement across locations is separately required by M0-T07 (`actor → store → cargo → site`). That chain is an ownership transfer, not a path search. The draft does not specify it. See G3-H02.

### 5.6 Persistence

`SH:C02:STATE` revision 1 is a closed encoding. C0.2 requires a new revision for a new field and assigns the first migration to M0-T11. A production ledger therefore needs a new schema. The draft leaves the tag, revision, and field order open. That is the right refusal and an implementation blocker.

C03-T01g states the C0.2 prefix rule for a future production blob and says a revision 1 blob must not be read as a production save. It does not list truncation, checksum, revision, content-hash, or corrupt-count failures for the new format. Event identity for production events has no dialect, no canonical bytes, and no digest domain string.

## 6. Acceptance coverage

| Approved result | Proposed test | Coverage judgment |
| --- | --- | --- |
| C0.1 and C0.2 goldens unchanged | C03-R01 | Sufficient as a regression command. It is not a C0.3 pass |
| Two claimants, both orders, winner and loser diagnostic | C03-T01a | Procedure exists. Diagnostic, encoding, and fixture are blocked. A third claimant is added without a required diagnostic |
| Expiry at the completion tick, one release, repeat is a no-op | C03-T01b | Procedure matches C03-T01. "Emits nothing" conflicts with C03-T01e |
| Cancel at the completion tick | C03-T01c | Blocked. No command contract |
| Disable at the completion tick | C03-T01d | Blocked, and the construction-time flag cannot express the tie |
| Replay completion and repeat release | C03-T01e | Blocked. Event-cursor rule is conditional |
| Failed later recipe leg | C03-T01f | Matches the last C03-T01 sentence only. It is not M0-T07 |
| Save and continue | C03-T01g | Happy path only. No corrupt or wrong-schema matrix |
| Failed call then a later success | C03-T01h | Intent is right. Error variants are unnamed |
| Grouped versus single-boundary drive | C03-T01i | Matches the C0.2 equivalence pattern. Schema is missing |
| M0-T06 20 NPCs and four resource classes | D11, not a test | Absent |
| M0-T06 included ordering case | C03-T01a | Present only as the one-unit case |
| M0-T07 four-location trace and declared recipes | None | Missing |
| M0-T08 unreachable targets | None | Missing |
| Needs, jobs, navigation | C03-N01, C03-J01, C03-P01 | Explicitly blocked, so they are not passes |
| Mode does not change production accounting | None | Missing |
| Disabled actor cannot acquire | Spec §6 only | Missing |
| Combined invalidation causes | None | Missing |
| New golden hashes | Section 12 withholds them | Correct. Withholding is not a substitute for the schema |

The proposed milestone does not satisfy the README exit. Every new C0.3 row except C03-R01 is blocked, and the M0 rows cited by that exit are only partly represented.

## 7. Findings

### G3-B01 — Production keys and canonical schema are not frozen

| | |
| --- | --- |
| Severity | BLOCKER |
| Source | Core §7; Core §5; C03-T01 first sentence; C0.3 spec §12 D1, D2, D5 |
| Description | C03-T01 says to freeze command priority, reservation keys, and transaction IDs before writing production behavior. Core §7 repeats that the C0.3 spec defines keys and diagnostics before implementation. The draft lists those items as open and publishes no tag, field order, event dialect, or diagnostic bytes. |
| Impact | An implementer cannot admit a reservation, hash a checkpoint, or compare a replay without inventing the encoding the frozen C0.1 and C0.2 process forbids. |
| Remediation | Owner approves the schema and diagnostics. Revise the specification with the byte layout. Pack no golden vector until an independent oracle uses that layout. |
| Tests | C03-T01a through C03-T01i |

### G3-B02 — The C0.3 gate name and the executable exit describe different milestones

| | |
| --- | --- |
| Severity | BLOCKER |
| Source | README gate table; Technical Architecture §§2 and 10; Core preamble, §6, and §8; C0.3 spec §§2 and 7 |
| Description | The deliverable cells name needs, jobs, inventory, resource transactions, and initial navigation. The exit cell names C03-T01 and applicable M0-T06 through M0-T08. Core calls the actor goals a C0.3/M0 joint list and places the route graph on M0. No document defines the missing formulas or the graph. The draft blocks C03-N01, C03-J01, and C03-P01. |
| Impact | Implementing the ledger alone leaves the README and Technical Architecture deliverable unmet. Inventing needs or navigation to close the deliverable violates the "define before implementation" rule. Either reading can be chosen only by the project owner. |
| Remediation | Record one scope. Either narrow the C0.3 deliverable to the ledger fixture and move needs, jobs, and navigation to a later authorization, or supply those models before any C0.3 coding starts. |
| Tests | C03-N01, C03-J01, C03-P01, M0-T08 unreachable clause |

### G3-B03 — Completion-tick disablement has no legal input

| | |
| --- | --- |
| Severity | BLOCKER |
| Source | C03-T01; M0-T08; C0.2 spec persistence rules ("Flags do not change after construction"); C0.3 spec §6 and acceptance C03-T01d |
| Description | The tie requires a claimant who already holds a reservation and is disabled on the completion tick. C0.2 enabled flags are immutable after construction. The draft also says a disabled claimant does not acquire a reservation. A construction-time false flag therefore cannot set up the tie. No disable command exists. |
| Impact | C03-T01d is not executable. A hidden mutation of the C0.2 flag would also change a frozen persistence rule. |
| Remediation | Approve a replayable production-fixture input that changes the enabled bit in the command phase of the completion tick, and state that fixture 1 and fixture 2 flags stay immutable. |
| Tests | C03-T01d, C03-R01 |

### G3-H01 — C0.2 text can be read as an order to change fixture 2

| | |
| --- | --- |
| Severity | HIGH |
| Source | C0.2 spec §8, sentence beginning "C0.3 replaces this synthetic completion"; baseline manifest §6; C0.3 spec §1 |
| Description | The C0.2 specification both preserves disabled synthetic completion and says C0.3 replaces that completion with the production invalidation rule. The baseline freezes the synthetic behavior and its hashes. The C0.3 draft preserves both rules by isolation. |
| Impact | A literal replacement edits published scheduler blobs and breaks C02-T02. Leaving the sentence unresolved lets a later implementer "fix" fixture 2 in the name of invariant 4. |
| Remediation | Owner records that published fixture 2 stays, and that the production rule applies only to the new fixture. Do not edit the frozen C0.2 archive to clarify the sentence. |
| Tests | C03-R01, C02-T02, the fixture 2 clause of C03-T01d |

### G3-H02 — The M0-T07 transfer is not specified

| | |
| --- | --- |
| Severity | HIGH |
| Source | M0-T07; C03-T01 last sentences; Core §7 ownership locations; C0.3 acceptance §1 |
| Description | M0-T07 requires a traced transfer actor → store → cargo → site, declared recipes, and a later invalid leg. C03-T01f specifies only the invalid later leg. No owner identifiers, recipe inputs, outputs, or expected balances are given. |
| Impact | "Run the corresponding M0-T07 case" has no executable expected result beyond rollback. |
| Remediation | Owner decides whether the four-location trace is inside C0.3. If it is, approve the owners, quantities, and signed deltas before coding. If it is M0, say so in the exit text. |
| Tests | No row covers the four-location trace |

### G3-H03 — Unreachable-target failure is required and unspecified

| | |
| --- | --- |
| Severity | HIGH |
| Source | M0-T08; Technical Architecture §10; Core §8; C0.3 acceptance matrix |
| Description | M0-T08's required result includes unreachable targets failing with a deterministic reason and no invalid mutation. The acceptance matrix has no such case. Navigation data does not exist. |
| Impact | Promoting C0.3 on the current matrix drops a sentence of the test C03-T01 says to run. |
| Remediation | Owner either removes that sentence from the C0.3 promotion set until a navigation fixture exists, or supplies the graph and the failure diagnostic. |
| Tests | None |

### G3-H04 — The repeat-release event rule contradicts itself

| | |
| --- | --- |
| Severity | HIGH |
| Source | C0.3 spec §4; acceptance C03-T01b and C03-T01e |
| Description | Section 4 and C03-T01b say a second release emits nothing and does not advance the event cursor. C03-T01e allows the cursor to advance when the schema requires a rejection event, and says the test will accept the approved choice. |
| Impact | Two conforming readings produce different event streams and digests. |
| Remediation | Choose one rule and use it in every repeat-release test. The recommendation in the decision matrix is a silent no-op with no cursor advance. |
| Tests | C03-T01b, C03-T01e |

### G3-H05 — Simultaneous invalidation causes have no reason order

| | |
| --- | --- |
| Severity | HIGH |
| Source | Core §7; M0-T08 "deterministic reason" |
| Description | Disablement, cancellation, and expiry are each specified alone at the completion tick. The documents do not say which reason is recorded when two or three apply together, or whether one release or several are attempted. |
| Impact | The diagnostic byte and the event count are not functions of the state alone. |
| Remediation | Define a total reason priority and one release. Add one combined-cause acceptance case after the diagnostics exist. |
| Tests | C03-T01b, C03-T01c, C03-T01d do not cover the combination |

### G3-H06 — "Applicable" M0-T06 is two different scenarios

| | |
| --- | --- |
| Severity | HIGH |
| Source | README exit cell; M0-T06; M0 §1; C03-T01; C0.3 spec D11 |
| Description | M0-T06 is a 20-NPC contention for beds, tools, food, and workstations, and it also says to include the C03-T01 ordering case. The 20-NPC world is the M0 content scenario, which §1 still treats as hand-authored later. The draft's D11 leaves the choice open. |
| Impact | One reading makes C0.3 depend on unauthored content and `ticks_per_game_day`. The other reading exits on the one-unit fixture only. |
| Remediation | Owner selects the promotion set. The recommendation is the one-unit ordered fixture now, and the 20-NPC scenario at M0. |
| Tests | C03-T01a |

### G3-M01 — Task stages have names and no transitions

| | |
| --- | --- |
| Severity | MEDIUM |
| Source | Core §6; Core invariant 7; C0.3 spec §5 |
| Description | The six stages and the interruption branch have no guard, duration, or release rule. Save/restore of "next eligible action" is required. C03-T01g restores a reservation and does not restore a stage or progress tick. |
| Impact | Job implementation would invent the lifecycle the ledger fixture may not need. |
| Remediation | Approve an explicit reduced stage map for the ledger fixture, or defer stage timing with the job catalog. |
| Tests | C03-J01, C03-T01g |

### G3-M02 — Reservation quantity edges are unspecified

| | |
| --- | --- |
| Severity | MEDIUM |
| Source | Core §7; Core invariant 2; acceptance C03-T01h |
| Description | The one-unit case is specified. Zero quantity, an empty `[acquired, expiry)` interval, partial reservations, and a checked overflow of the reserved sum are not. Partial fill versus whole rejection is not stated. |
| Impact | Availability can be implemented with more than one deterministic behavior. |
| Remediation | State rejection rules for those inputs in the schema decision. Add cases beside C03-T01h. |
| Tests | C03-T01h covers only "would exceed on-hand" |

### G3-M03 — Production restore validation is a happy path

| | |
| --- | --- |
| Severity | MEDIUM |
| Source | Core §11; Technical Architecture §8; C0.2 spec restore precedence |
| Description | C03-T01g checks a valid round trip and says a C0.2 blob is not a production save. It does not name the error, and it does not require truncated, bad revision, checksum, corrupt count, or content-mismatch cases for the new format. |
| Impact | A new decoder can accept a hostile length or a swapped fixture and still satisfy the written test. |
| Remediation | Carry the C0.2 rejection classes forward by name once D1 chooses the tag. |
| Tests | C03-T01g |

### G3-M04 — Active and scheduled production behavior is untested

| | |
| --- | --- |
| Severity | MEDIUM |
| Source | Core §4; LOD §§2 and 4; C0.2 spec §8 |
| Description | C0.2 proves that active and scheduled actors complete the same synthetic transfer. Core §4 says production fairness is a separate decision. The draft has no case where a scheduled claimant and an active claimant contend, or where a mode change during a live production reservation preserves the reservation. |
| Impact | A new phase can treat mode as eligibility and still pass the current matrix. |
| Remediation | Add one production case: mode does not change the winner, the reserved quantity, or the owner. Keep automatic mode policy deferred. |
| Tests | None. C02-T02 is the synthetic proof only |

### G3-M05 — The invalidation phase is proposed, not dispatched

| | |
| --- | --- |
| Severity | MEDIUM |
| Source | C0.3 spec §8; C0.2 spec phases |
| Description | The draft asks the owner to accept a phase after commands and before work, and says fixture 1 and fixture 2 events must not move. It does not say whether those fixtures execute the phase. |
| Impact | The phase is the right Core §7 reading and is still an unapproved control-flow change. |
| Remediation | On approval, state that fixtures 1 and 2 do not run production invalidation. Keep C03-R01 as the byte proof. |
| Tests | C03-R01 |

### G3-L01 — Acquisition by a disabled claimant is untested

| | |
| --- | --- |
| Severity | LOW |
| Source | C0.3 spec §6; Core invariant 4 |
| Description | The spec forbids a disabled claimant from acquiring a reservation. The matrix tests disablement only at completion. |
| Impact | The acquire path can ignore the flag. |
| Remediation | Add one rejected acquisition once D4 exists. |
| Tests | None |

### G3-L02 — Overflow rollback does not name the C0.2 error

| | |
| --- | --- |
| Severity | LOW |
| Source | Acceptance C03-T01h; C0.2 `CapacityExceeded` and `Corrupt` split |
| Description | The test says to use the same rollback class as C0.2 for tick, event-cursor, and record-cursor overflow. C0.2 uses different errors for arithmetic capacity and for a corrupt declared count. |
| Impact | A test can accept the wrong error and still claim rollback. |
| Remediation | Name the error for each overflow site when the production API is approved. |
| Tests | C03-T01h |

### G3-I01 — The draft refuses unapproved mechanics

| | |
| --- | --- |
| Severity | INFORMATIONAL |
| Source | C0.3 spec §§7, 11, and 12; acceptance §12 |
| Description | Need curves, pathfinding, Bevy, serde, revision-1 field defaults, and encoder-dumped goldens are withheld. |
| Impact | The packet is safe to review and unsafe to implement. That is the correct pairing for this verdict. |
| Remediation | Keep the refusal in the revised spec. |
| Tests | C03-N01, C03-J01, C03-P01 |

### G3-I02 — Performance and R2 stay outside this gate

| | |
| --- | --- |
| Severity | INFORMATIONAL |
| Source | Core §1; Technical Architecture §6; M0-T14; baseline manifest §7 |
| Description | Canonical mutation is serial. No C0.3 timing budget exists. The audited host does not certify R2. |
| Impact | None for this specification verdict. |
| Remediation | Do not add a budget or a parallel schedule in the C0.3 schema revision. |
| Tests | C03-R01 does not claim R2 |

### G3-I03 — This review did not re-execute C0.2

| | |
| --- | --- |
| Severity | INFORMATIONAL |
| Source | `C0_2_BASELINE_MANIFEST.md`; `C0_2_INDEPENDENT_AUDIT.md` |
| Description | G3 relied on the frozen manifest and the prior PASS. It did not extract the archive or run `cargo test`. |
| Impact | The C0.2 verdict is unchanged and is not re-proven here. |
| Remediation | Re-run C03-R01 only when an authorized tree claims to preserve the baseline. |
| Tests | C03-R01 |

## 8. Traceability summary

The draft's own review is accurate when it says the packet is reviewable and not authorized. It is not accurate as an account of exit coverage: blocking needs, navigation, M0-T07, and the unreachable-target sentence, then still describing the ledger as "specified as far as the documents go", leaves Core §7's "before implementation" precondition unmet. The ledger rules are known. The fixture that would prove them is not.

Frozen C0.1 and C0.2 contracts are protected by the draft's regression section and by the decision to keep a separate fixture. That protection holds only if G3-H01 is resolved by preserving fixture 2.

## 9. Readiness

| Question | Result |
| --- | --- |
| C0.2 baseline intact in this review | Yes. No source or golden file was edited |
| C0.3 specification internally consistent | No. G3-H01, G3-H04, G3-B02 |
| Every C0.3 exit requirement has an executable test | No |
| Unapproved product decisions remain | Yes. They are listed in the decision matrix and are not closed here |
| Implementation-ready | No |

**FAIL.** Implementation requires a revised specification after project-owner decisions, and then a separate implementation authorization.
