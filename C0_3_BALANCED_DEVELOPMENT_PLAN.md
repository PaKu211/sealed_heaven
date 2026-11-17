# C0.3 Balanced Development Plan

Version R2.1-final-approved · 17 November 2026  
Status: planning. AM-01 through AM-08 are approved. Implementation is not authorized.

## 1. Baseline

| Gate | State |
| --- | --- |
| C0.1 | Implemented. Independent tests passed on the preserved archive |
| C0.2 | Implemented. Independent audit PASS. Archive SHA-256 `2804d33a785acb7331a0d82fd1c080bec82d0add348578abb4b1a6807db60a40` |
| Toolchain | Rust 1.100.0, `rand_chacha` 0.10.0, `sha2` 0.11.1. Recorded in `C0_2_BASELINE_MANIFEST.md` |
| C0.3 decisions | OD-01 through OD-16 approved from proposal v1.2. Record: `C0_3_OWNER_DECISION_RECORD.md` |
| Contract repair | R2.1-final approved. AM-01 through AM-08 recorded in `C0_3_OWNER_DECISION_RECORD.md` version 2.0. OC-EDGE rejected |
| Spec status | Normative docs revised to version 1.1-approved. Independent G3 re-review required. Prior audit FAIL is historical |

Frozen surfaces: fixture 1 and fixture 2 encodings, C0.1 and C0.2 golden vectors, fixture 2 synthetic completion while disabled, and H0 `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c`. Production behavior belongs in fixture 3. Bevy does not enter `sealed_sim`.

## 2. Mandatory C0.3 exit

These stay in the milestone. An internal slice does not close C0.3.

| Source | Obligation |
| --- | --- |
| README gate | Needs, jobs, inventory, resource transactions. Exit: C03-T01 and the applicable M0-T06, M0-T07, and M0-T08 procedures |
| Technical Architecture §§2 and 10 | Needs, jobs, physical ledger, initial navigation, resource-transaction fixture |
| Core §§6–8 and §12 invariant 4 | Five goals, DecisionTrace, reservation interval, invalidation before completion, one-shot legs, disabled actor cannot continue an invalid production job |
| ADR-013 | One owner, atomic transactions, derived availability |
| OD-01–OD-16 | Fixture 3 schema and phases as approved, including the creation-tick work skip |
| M0 §6 | C03-T01, and the written 20-actor, four-location, and invalidation procedures |

Already owned elsewhere: automatic mode policy, travel approximation, cohorts, day length, the 30-day run, save migration, JSON content, Bevy, and a later milestone. Fixture weights, wake interval 4, and supply count 5 are fixture configuration, not final game balance.

## 3. How the work proceeds

1. The owner approved AM-01 through AM-08. Normative documents are version 1.1-approved.
2. An independent G3 re-review examines that text. This plan does not declare that review passed.
3. After G3 PASS, the owner may authorize S1, then S2, then S3, then S4.
4. The milestone exits only when S4's full matrix passes, together with the frozen C0.1 and C0.2 suites.

## 4. Slice summary

| Slice | What it adds | Still incomplete after it |
| --- | --- | --- |
| S1 Resource core | Ledger, reservations, consumed completion registry, cancel, disable, expiry, exactly-once release, rollback | Needs, movement, the four-location chain, 20 actors |
| S2 First living NPC | One actor, DecisionTrace, Eat, save and replay | Rest, work, haul, return home, navigation, contention |
| S3 Movement and production | The other four goals, graph and grid, edge progress, M0-T07, unreachable site 9 | The 20-actor integration |
| S4 Integration | Five goals together, M0-T06, and the full C03 and M0 matrix | Nothing inside the C0.3 exit. C0.4 is not started |

S1 through S3 are not milestone exits. S4 is the first slice that can meet the exit, and a passing S4 is evidence for the owner rather than an independent sign-off.

## 5. Isolation

Fixture 3 uses `SH:C03:STATE` revision 1 and `SH:C03:EVENT:v1`. It does not extend the C0.1 observation, the C0.2 save, the C0.1 outcome enum, or fixture 2's synthetic rule. `step` on fixture 3 returns `UseDrive` and does not mutate. Production phases are mode, command, invalidation, decision, and work. Fixture 2 keeps its own work phase.

The approved graph stays sites 1, 2, and 9 and edges 1 and 2. Mid-edge progress is a locus, an edge id, and a traversed cost. Work spends that cost. It does not relocate the actor to start a search.

Likely new modules, after authorization, live beside the frozen files: a fixture-3 world, ledger, tasks, needs, and navigation. No new crate.

## 6. Readiness

The normative specification is revised and ready for independent G3 re-review. This plan does not declare G3 PASS. Implementation is not authorized. S1 is not started.
