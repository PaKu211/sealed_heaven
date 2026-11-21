# C2.3 acceptance matrix

| # | Requirement | Result |
| --- | --- | --- |
| 1 | Firsthand observation | `player_beside_an_audit_learns_it_firsthand` |
| 2 | Hidden information stays unknown | `a_distant_player_does_not_learn_an_audit` |
| 3 | Physical investigation | `examining_the_site_records_checked_evidence` |
| 4 | Reporting known information | `telling_checked_evidence_updates_the_institution_with_one_hop` |
| 5 | Unknown evidence rejected | `unknown_testimony_is_refused_and_grants_nothing` |
| 6 | Face-to-face testimony | `testimony_requires_a_face_to_face_meeting` |
| 7 | Duplicate testimony | `repeating_testimony_does_not_open_a_second_dispute` |
| 8 | Provenance preserved | hop-1 examined evidence; `asking_a_witness_queues_their_account_without_borrowing_its_weight` stays hop 2 and opens no mentor dispute |
| 9 | Competing accounts | `a_later_success_restores_cooperation_without_erasing_the_failure` |
| 10 | Decision before and after intervention | mentor bid delta moves from 0 to negative after delivery of player testimony |
| 11 | Non-intervention | `telling_and_withholding_diverge_from_the_same_world` |
| 12 | Player death leaves the finding | `player_death_leaves_the_finding_in_the_world` |
| 13 | Listener death drops the report | `a_dying_listener_never_receives_queued_testimony` |
| 14 | Pending testimony in checkpoint state | `pending_testimony_is_in_the_digest_and_matching_copies_agree` |
| 15 | Both authored seeds replay | `both_seeds_replay_a_player_agency_campaign` |
| 16 | Event ids stay unique past 96 | `player_agency_keeps_event_ids_unique_past_the_ledger_cap` |
| 17 | Revision 13 isolation | `revision_thirteen_rejects_player_agency_actions` |
| 18 | Legacy revision 12 has no information ledger | same test, `contract_accountability` |
| 19 | English and Indonesian ASCII | `c23_know_is_read_only_in_both_languages` |
| 20 | Bounded long run | `a_long_campaign_stays_within_the_information_caps` |

The divergence test clones one world after the same audit and the same examine, then one copy testifies and the other waits. After the report delay, mentor 33's bid toward hall 36 is negative only in the copy that spoke. The unfulfilled finding is the same in both. Further waiting does not create that dispute by itself.

Public commands `examine`, `tell`, and `know` are covered by `c23_public_commands_examine_and_can_withhold`. The institutional consequence of those commands is the same intent path as the simulation test. A short public script does not wait for a naturally opened contract; those audits are still produced by the existing contract engine, including the shared audit fixture used since C2.1.
