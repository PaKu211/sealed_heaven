# Sealed Heaven — G0 Remediation Report

Version 0.2-draft · 17 November 2026

**R1–R3 documentation complete. Ready for independent R4 review. R4 verdict pending.**

## Baseline and authorization

The [independent audit](G0_REVIEW_REPORT.md) concerns `Sealed_Heaven_G0_v0.1.zip`, SHA-256 `98eadf36b4a6066647c6e322995e87f7ec694ca5edab083d64fff6eb08ba4443`. The available project source was that archive, with no checked-out Git repository. I read its README and ten documents in the prescribed order, confirmed the extracted contents matched the archive and ten attached documents, and read the supplied audit and the owner-approved R1–R3 amendments. This is a versioned replacement pack for review. The original audit and archive remain historical inputs.

**RESOLVED** indicates that the documentation correction is present and was checked by the remediation author. **DEFERRED WITH OWNER** indicates that a later design choice, authored fixture or measurement has an explicit owner and executable criterion. **UNRESOLVED** would indicate a remaining documentation problem in the authorized scope. No implementation test was executed. The audit's FAIL verdict remains attached to its original baseline.

## Finding dispositions

Names below are relative to `docs/` unless specified. The [change log](G0_CHANGELOG.md) links every changed file.

| Finding | Original problem | Files changed | Evidence | Status |
| --- | --- | --- | --- | --- |
| B01 | Pre-order validation/reservations and unspecified ties could change the winner | Core, GDD, C0.1 spec/tests, ADRs, M0 tests | Explicit batch boundary, read-only diagnostics, duplicate/late policy, serial order, live revalidation and atomic outcomes. C01-T01/T02/T03 covers competing producers and actors, disabled/occupied events and rollback. Actor ID priority is fixture-only | RESOLVED |
| B02 | Projection teardown could flush queued gameplay work and visibility could select processing modes | LOD, Technical, Core, GDD, ADRs, M0/V0 tests | Queue and processing mode owned by simulation. C02-T02 tests explicit domain triggers. I0-T01 defines paused rebuild with admitted work, full checkpoint equality and identical resumed outcomes | RESOLVED |
| B03 | Hash and snapshot did not fully specify future state or event-prefix continuity | C0.1 spec/tests, Core, Technical, M0 tests, ADRs | Revision 1 encoding names every field/width/discriminant/tag. Independent byte, digest and RNG vectors. C02-T01 specifies event 3 and H3 at resume. M0-T02/T03 defines a non-empty saved prefix and future admitted command across the 15+15 split | RESOLVED |
| B04 | C0.1 scope and its own gate were missing | README, C0.1 spec/tests, Core, Technical, ADRs, Workflow, M0 tests | Six C0.1 test groups with one pure Rust library, explicit exclusions and downstream gate ownership | RESOLVED |
| H01 | Wall time, tick duration and game-day conversion were conflated | Core, GDD, Vision, Technical, M0 tests, ADRs | Canonical logical ticks and checked, scenario-owned `ticks_per_game_day` are defined. I0 host maintainer executes pause/stamping in I0-T02. M1 playtest owner measures exact 10/20/30 rates in M1-T03. C0.1 has no wall clock and can proceed independently after approval | DEFERRED WITH OWNER |
| H02 | Coarse actor progression lacked an accounting oracle | LOD, Core, M0 tests, ADRs | C0.2 scheduler owner runs C02-T02 at ticks 2/3/4 with exact ownership, order and no approximations. M0-T05 extends the test before any production activation/approximation is used. C0.1 contains no scheduler or LOD | DEFERRED WITH OWNER |
| H03 | Reservation expiry, invalidation and recipe atomicity lacked a tie and rollback rule | Core, M0 tests, ADRs | C0.3 ledger owner freezes production priority, keys and diagnostics and runs C03-T01/M0-T06/T07/T08. The spec fixes half-open validity, invalidation before completion and atomic transactions. C0.1 slot contention has no production stock or reservations | DEFERRED WITH OWNER |
| H04 | A deterministic but inert long run could pass, with excessive hashing and tracing | Vision, Core, M0 tests, ADRs | M0 scenario owner writes and reviews the 20-actor fixture and exact causal oracle before collecting goldens. M0-T01/T13/T14 require real need/work/relationship effects and scheduler counts. C0.1 uses checkpoint-only hashes and has no long-run workload | DEFERRED WITH OWNER |
| H05 | V0 mocked presentation and later real bridge evidence were mixed | README, Technical, LOD, M0/V0 tests, Workflow | V0-T10 checks stale/gap/rebuild semantics on mock inputs. I0-T01/T02 own real checkpoint/replay evidence. M0-T04/T12 cite those results while T14 remains headless | RESOLVED |
| M01 | Visual acceptance lacked concrete layout, portrait and input criteria | Art, V0 tests, ADRs | V0 visual owner registers usable tile area and portrait collision/recognition rubric after exploratory measurement and before graded tests. T05 names two changed tile/layer pairs, and T08 gives a keyboard/focus path. Safe to defer measured UI/asset values because C0.1 is headless | DEFERRED WITH OWNER |
| M02 | Optional inheritance and online schema tooling could become requirements | Technical, ADRs, M0 tests | M0-T09 mandates three typed validation cases and conditions cycle/network checks on actual selection. Optional validator must reject unregistered remote URIs without I/O | RESOLVED |
| M03 | M1 talk/trade and death/reload were ambiguous | GDD, Vision, ADRs | M1-T01 requires talk, trade and an explained simulation consequence. M1-T02 tests death ending the attempt and reload restoring the saved history | RESOLVED |
| M04 | Target workspace and extension points risked becoming initial scaffolding | README, Core, Technical, C0.1 spec/tests, ADRs | Single package and fixture. C01-T06 audits scope and graph. Later layout is conditional on named consumers | RESOLVED |
| L01 | FPS percentiles obscured the slow tail | V0 tests | V0-T12 switches to p50/p95 frame time in milliseconds and records low-end hardware availability, memory and provisional FPS targets | RESOLVED |
| L02 | A documentation verdict could be confused with executed milestone evidence | README, Workflow, Technical | R4 verdict and evidence status are separate. Implementation gates require executed results and explicit skip reporting | RESOLVED |

Totals: 10 RESOLVED, 5 DEFERRED WITH OWNER, 0 UNRESOLVED in this documentation review. All four blockers have a documented correction and test owner. R4 independently checks closure.

## C0.1 requirement traceability

| C0.1 requirement | Future implementation owner | Executable evidence |
| --- | --- | --- |
| Stable fixture config, typed IDs, deterministic start, tick/batch boundary, pause and pending work | `sealed_sim` fixture and clock | C01-T01, C01-T03 |
| Immutable preflight/admission, key precedence, duplicate/late handling and atomic rollback | `sealed_sim` boundary and queue | C01-T02, C01-T03 |
| Serial commit, live eligibility, deterministic conflict results, singleton slot and event outcomes | `sealed_sim` commit and event outputs | C01-T02, C01-T03, C01-T04 |
| Named seed derivation, fixed raw words, one ordinal per successful decision and stream isolation | `sealed_sim` RNG probe | C01-T02, C01-T04, C01-T05 |
| Complete byte encoding, ordered prefix digest and cursor, stable checkpoint and overflow | `sealed_sim` checkpoint and incremental event hash | C01-T03, C01-T04, C01-T05 |
| Same build/platform replay and independently specified outcomes | `sealed_sim` in-package test driver | C01-T05 |
| Single library, dependency/exclusion audit, locked tooling and headless evidence | C0.1 build maintainer | C01-T06 |

## Documentation verification

- Compared all 11 source Markdown files against the 0.1 archive and the ten attached specifications against the archive. Preserved the original audit byte for byte.
- Recomputed SHA-256 with Python and OpenSSL for all canonical-vector preimages, including four complete checkpoints, two event branches and the C02-T01 continuation. Cross-checked all four full 64-byte ChaCha12 blocks using separate Python and JavaScript calculations from the published algorithm. Verified the published hex again from the delivered Markdown.
- Audited local navigation, acceptance identifiers and milestone ownership. Preserved M0-T01 through M0-T15 and V0-T01 through V0-T12. Added six C0.1 groups and named C0.2/C0.3/I0/M1 cases. Checked that each C0.1 normative field/rule has executable acceptance coverage.
- Reviewed ownership and timing across Core, Technical, GDD and LOD. Projection teardown has no command admission or mutation path. Configurable world-day conversion and wall-clock host cadence have distinct contracts. C0.1 exposes in-memory bytes, while C0.2 adds file persistence.
- Compared ADR dispositions with the 0.1 register. ADR-001 keeps Bevy 0.20.1 for the later client. Rust/cryptographic crate version candidates are provisional until C01-T04/T06 verifies actual configuration and lock resolution. No Cargo project, engine code, renderer, assets or narrative import was created.

## R4 review and outstanding evidence

Please review the [C0.1 specification](docs/C0_1_IMPLEMENTATION_SPEC.md), its [acceptance vectors](docs/C0_1_ACCEPTANCE_TESTS.md), and the [LOD ownership contract](docs/SIMULATION_LOD_SPEC.md) against B01–B04. Record an independent R4 result before any C0.1 launch authorization.

The largest remaining risks are checkpoint completeness once production fields are added, production fairness and activation policy, and real graphical cadence/hardware behavior. Their evidence gates are C02-T01, C03-T01/M0-T05, and I0-T02/V0-T12/M1-T03 respectively. Candidate dependencies have published support but no project lock/build result. The planned documentation asserts no implementation milestone pass.
