# C2.2 Acceptance Matrix

| # | Requirement | Evidence | Status |
| --- | --- | --- | --- |
| 1 | Firsthand information acquisition | `firsthand_audit_creates_named_event_with_firsthand_provenance` | Implemented + tested |
| 2 | No omniscience for unwitnessed events | `unwitnessed_remote_failure_is_not_omniscient_before_audit` | Implemented + tested |
| 3 | Face-to-face information transfer | `face_to_face_testimony_preserves_origin_and_hops` | Implemented + tested |
| 4 | Physical delay before distant knowledge | `distant_knowledge_waits_for_physical_delivery_delay` | Implemented + tested |
| 5 | Provenance preservation | face-to-face + firsthand tests | Implemented + tested |
| 6 | No duplicate effects from repeated reports | `repeated_testimony_does_not_multiply_dispute_effects` | Implemented + tested |
| 7 | Conflicting information | `conflicting_delivery_testimony_can_refute_an_open_dispute` | Implemented + tested |
| 8 | Dispute initiation | firsthand suspension + hop-1 open path | Implemented + tested |
| 9 | Dispute resolution | investigation promote + refute tests | Implemented + tested |
| 10 | Consequential institutional decisions | suspended provider excluded; Mentor can refuse sole Hall offerings bid | Implemented + tested |
| 11 | Reporter/witness death | `dead_reporter_blocks_pending_testimony_delivery` | Implemented + tested |
| 12 | Reports interrupted in transit | dead reporter + dead recipient tests | Implemented + tested |
| 13 | Multiple simultaneous information events | `multiple_audit_events_propagate_independently` | Implemented + tested |
| 14 | Bounded propagation and memory | `information_memory_stays_bounded` | Implemented + tested |
| 15 | Save/load with unresolved disputes | digest membership + stepped replay tests | Implemented + tested |
| 16 | Deterministic replay on seeds 0 and 1 | `v13_replays_canonical_worlds_and_keeps_v12_campaign_exact` | Implemented + tested |
| 17 | Revision-12 isolation | same test + ASCII `c22_seeding_reset_and_c21_legacy…` | Implemented + tested |
| 18 | Legacy checkpoint protection | rev 11 isolation test | Implemented + tested |
| 19 | English/Indonesian ASCII presentation | `c22_infolog_is_witness_only_localized_and_read_only` | Implemented + tested |
| 20 | Player-facing visibility restrictions | witnessed-only history + ASCII privacy + absent-player history test | Implemented + tested |
| 21 | Hop-2 allegation does not open disputes | `hop_two_allegations_are_remembered_without_opening_disputes` | Implemented + tested |
| 22 | `--c21` / rev 12 has no information ledger | `revision_twelve_audits_never_create_information_or_suspension` + ASCII c21 flag test | Implemented + tested |

## Verification labels

- **Implemented:** present in source
- **Statically verified:** reviewed for causality / privacy / cascade wiring
- **Compiled:** builds on rustc 1.100.0 (Linux cloud)
- **Tested:** automated suites named above
- **Runtime/playtested:** scripted ASCII smoke optional; human playtest pending on Windows
- **Pending verification:** Windows host re-run of the same cargo gates
