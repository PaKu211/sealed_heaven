# C2.2.1 Remediation — Information Memory & Consequences

Checkpoint revision remains **13**. No foundation rewrite; behavioural fixes only.

## Fixes

| Priority | Issue | Fix |
| --- | --- | --- |
| P1 | New successful delivery refuted old failures | Later `Delivered` findings set disputes to **`Restored`**, not `Refuted`. The original `Unfulfilled` event stays on record. |
| P1 | Hop-2 could not be upgraded | Stronger provenance (hop-1 / firsthand) upgrades existing knowledge and can open exactly one dispute without multiplying repeats. |
| P1 | Permanent suspension | Suspensions restore via credible later delivery **or** after `SUSPENSION_DURATION` (960 ticks) via `advance_information_recovery`. |
| P2 | Hard stop at 96 events | Settled event slots are recycled (stable `EventId` index reused after purging cold references). Hot disputes still block recycle. |

## Regression coverage

- `later_hop_one_testimony_upgrades_hop_two_knowledge_and_opens_one_dispute`
- `later_successful_delivery_restores_trust_without_erasing_failure_history`
- `aged_suspension_restores_after_review_period_without_refuting_the_case`
- `full_event_ledger_recycles_settled_slots_instead_of_ignoring_new_findings`
- Existing `v13_replays_canonical_worlds_and_keeps_v12_campaign_exact`
- Existing `revision_twelve_audits_never_create_information_or_suspension`

## Verification labels

- Implemented / compiled / tested on Linux rustc 1.100.0
- Windows gate: not claimed
- Human playtest: not claimed
