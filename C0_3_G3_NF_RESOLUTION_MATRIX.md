# C0.3 G3 NF-01–NF-05 Resolution Matrix

Version 1.1 · 17 November 2026  
Status: documentation remediation. Implementation is not authorized. G3 PASS is not declared.

Source audits: `C0_3_G3_FINAL_INDEPENDENT_AUDIT.md` (NF-01–NF-04 FAIL) and `C0_3_G3_V1_2_INDEPENDENT_AUDIT.md` (NF-05 FAIL). Normative copy after this remediation: `docs/C0_3_IMPLEMENTATION_SPEC.md` version **1.3-approved** and `docs/C0_3_ACCEPTANCE_TESTS.md` version **1.3-approved**.

| ID | Severity | Root cause | Correction | Normative location | Acceptance | Owner approval |
| --- | --- | --- | --- | --- | --- | --- |
| NF-01 | BLOCKER | `Queued` tasks with `due_tick` 0 became `Reserved` then hit work rule 3 before `Executing` | On successful reservation, atomically set `due_tick = staged_tick + travel_ticks + 2` before work. Do not change `created_tick` | Spec §6.1 | §§3, 6, 11 | Closed in 1.2-approved. Unchanged in 1.3 |
| NF-02 | HIGH | M0-T06 stocks lacked `definition_id` | Fixture table: beds 3/class 2, tools 4/class 2, food 1/class 1, workstations 5/class 2 | Spec §12 | §11 | Closed in 1.2-approved. Unchanged in 1.3 |
| NF-03 | HIGH | Work goal could not distinguish tool from workstation | Tool = class-2 definition 4. Workstation = class-2 definition 5. Lowest stock id each | Spec §7 Work row | §§11, 14 | Closed in 1.2-approved. Unchanged in 1.3 |
| NF-04 | HIGH | Credit to existing `stock_id` unspecified | Non-zero stock must exist; definition, class, and owner must match; checked add; overflow/`InvalidLeg` | Spec §6.2 | §8 | Closed in 1.2-approved. Unchanged in 1.3 |
| NF-05 | HIGH | Preinstalled C03-T01a / M0-T06 tasks omitted `goal`, `recipe_id`, `interruption`, target, `progress_ticks` | Shared packing-field constructor; `goal` 0 for scripted carriers; work-phase-only `progress_ticks`; explicit initial and final tables; independent post-boundary blob hashes | Spec §§3, 8, 12 | §§1, 3, 11 | Documentation remediation under owner authorization. Fixture packing fields, not new gameplay rules |

| Prior finding | Post-remediation |
| --- | --- |
| F01–F08 | Remain closed. Not reopened |
| F07 | Closed via §6.1 (NF-01). Unchanged |
| F09 | Closed for NF-01–NF-05 packing gaps; C03-T01a and M0-T06 post-boundary blobs are uniquely derivable |

## Preserved contracts

Creation-tick skip, H0, completion registry, reservation accounting, residual edge travel, M0-T07 stock-14 later-leg rollback, OC-EDGE rejection, C0.1/C0.2 goldens, S1–S4 plan, Bevy exclusion, NF-01–NF-04 resolutions, and approved phase ordering are unchanged.

## Pending owner decisions

None required to apply this remediation. `goal` 0 for scripted preinstalled tasks and the packing-field tables are C0.3 fixture configuration.
