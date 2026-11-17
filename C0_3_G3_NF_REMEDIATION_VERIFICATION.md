# C0.3 G3 NF Remediation Verification

Version 1.1 · 17 November 2026  
Status: documentation verification for NF-01–NF-05. Implementation is not authorized. G3 PASS is not declared.

## Changed files (this remediation, NF-05)

| File | Change |
| --- | --- |
| `docs/C0_3_IMPLEMENTATION_SPEC.md` | Version 1.3-approved. Goal 0 for scripted tasks; work-phase `progress_ticks`; §12 packing fields |
| `docs/C0_3_ACCEPTANCE_TESTS.md` | Version 1.3-approved. Shared packing constructor; C03-T01a and M0-T06 tables and blob hashes |
| `docs/C0_3_ARCHITECTURE_REVIEW.md` | Version 1.3-approved |
| `C0_3_G3_NF_RESOLUTION_MATRIX.md` | NF-05 row; F09 closure |
| `C0_3_G3_NF_REMEDIATION_VERIFICATION.md` | This file |
| `C0_3_IMPLEMENTATION_GATE_CHECKLIST.md` | Points at 1.3-approved |
| `C0_3_G3_NF05_PACKING_VERIFY.txt` | Independent packer digests |

Not modified: Rust sources, `Cargo.toml`, `Cargo.lock`, toolchain, C0.1/C0.2 goldens, C0.2 archive, H0, M0-T07 initial blob.

## NF-05 resolution

| Field | Preinstalled value | Post C03-T01a / M0-T06 winners |
| --- | --- | --- |
| `goal` | 0 (scripted carrier) | unchanged |
| `recipe_id` | 0 | unchanged |
| `interruption` | 0 | unchanged while non-terminal |
| target | locus 1, site 1, x 0, y 0 | unchanged |
| `progress_ticks` | 0 | 1 after Reserved→Executing |
| path / planned legs / unconsumed | empty | empty |

§6.1 promotion does not increment `progress_ticks`. Losers remain at 0.

## Independent packing verification

| Vector | SHA-256 (body‖trailer) | Result |
| --- | --- | --- |
| H0 | `ff18f9506b6c610af37f18d81f10090ec125fba0507f9cbf9c927dfa95cc6b3c` | Pass (unchanged) |
| M0-T07 initial | `bd803a2c4ae4320ce2a1a5ceb49a2941aec48921c4cbc0a60b15c607b62148c3` | Pass (unchanged) |
| C03-T01a post-boundary | `11f9aae46d7f0f04b6c3efead0a25c3086bc56904877d237c0a79ae032a293d9` | Pass (unique; no invented fields) |
| M0-T06 post-boundary | `528bc334bc1f081ccbf06c3247ec46909abd6f1073594dfb13ebd7ae16d640c9` | Pass (unique; body `bccf8e1adf8bc22623202a58fd5d84b9911d8d1e42959331c692fcc55e3f1ed0`) |

Identical inputs reproduced identical bytes on a second pack of each blob.

## Check matrix

| Check | Result |
| --- | --- |
| NF-01–NF-04 unchanged and closed | Pass |
| NF-05: every preinstalled task packing field named | Pass |
| NF-05: no additional required fields found for C03-T01a / M0-T06 oracles | Pass |
| F01–F08 not reopened | Pass |
| H0 / M0-T07 / OC-EDGE / residual travel / creation-tick skip | Pass |
| C0.1/C0.2 freeze | Pass |
| Bevy still prohibited | Pass |
| S1 not started | Pass |
| G3 PASS declared | No |

## Completeness scan (requirement 9)

| Acceptance surface | Packing completeness |
| --- | --- |
| §3 C03-T01a | Complete; blob published |
| §11 M0-T06 | Complete; blob published |
| §12 M0-T07 | Already complete; blob unchanged |
| §§4–8 narrative Completing shapes | Inherit §1 packing fields; no separate published blob required by this remediation |
| §14 needs/jobs/movement | Procedure oracles; no new contention checkpoint required |

## Readiness

**Documentation readiness: READY FOR INDEPENDENT G3 RE-REVIEW of version 1.3-approved.**  
**Implementation NOT AUTHORIZED. S1 NOT STARTED. G3 PASS NOT DECLARED.**
