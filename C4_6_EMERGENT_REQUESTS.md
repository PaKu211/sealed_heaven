# C4.6 — Emergent requests and persistent consequences

Baseline: C4.5 commit 0a68c0d4e6a7bc5962d3d01706d80f852205a246. New deterministic rules revision 33, request model v1; all previous campaigns remain available. This is a bounded regional player-facing request slice, not an arbitrary quest generator, imported canonical lore, or replacement for autonomous NPC contracts.

## State and timing

Two institutional notice boards, Qinghe shrine and Willowford market. The shrine publishes when actual medicine stock is at most one and below target. The market publishes when actual herb stock is at or below its reserve, below target, and it has a coin available for a completion bonus. Each episode asks for min(2, current shortfall) real herbs. IDs are monotonic across both boards. Notices last up to 240 turns (deadline exclusive), with a 96-turn cooldown after closure. No new offers open at the maximum horizon; deadlines are clamped to the existing horizon. The board is institutional, not a fabricated named NPC interaction. Existing NPC needs, logistics, social reactions, and institutional contracts continue without requiring player participation.

Before player action and after NPC commits, resolve expired or no-longer-needed requests, then open eligible new notices and observe local state. A known accepted request that closes remotely remains available as a board record until the player learns its disposition or withdraws; no new notice overwrites that outstanding remembered commitment.

## Choices, physical work, reward

Discover and accept/decline a notice within reach and sight of its site. Acceptance requires a known still-offered ID. Decline closes the player-facing offer with no penalty or charge. Acceptance does not force work or change NPC priorities. Shrine contributions use the ordinary deliver action, market contributions use ordinary sell (daytime trade rules). A successful physical transfer counts exactly once only toward the active accepted request for that site. Complete after the requested units or if the player's last successful transfer reaches the site's target. Donations and sales remain possible without accepting any request. No instant travel, free cargo, forced quest, or global reputation grant.

The market pays its usual trade price plus a single completion bonus of one coin, reserved from its existing treasury at acceptance (which also checks the current sale budget). The ordinary price remains subject to existing market availability. The bonus is a separate physical escrow, included in the coin ledger; never minted. On completion transfer escrow to player; on expiration, abandonment, or external fulfillment refund to market. Shrine has no coin bonus: actual shrine delivery, ordinary witnessed aid effects, and a service receipt are its consequences. Partial work remains transferred even if the quest closes. If NPCs meet the target first, the request becomes superseded; the bonus is returned, with no punitive debt.

## Knowledge and persistence

Only a player within reach and sight of the board learns its current notice. Known snapshots include the last observed turn, not live remote stock or a remote closure oracle. Withdraw from a remembered accepted ID anywhere, with the same player-facing outcome even if the request has closed unseen. Remote withdrawal is a personal decision, not information about unseen events. Local observation may subsequently reveal the actual board disposition. Retain at most two current records, two snapshots and 32 remembered terminal receipts; lifetime completed/paid counters persist per site. All scheduling, identities, physical escrow, records, observations and receipts are hash-bound. Commands use historical 5-byte journal records: tags 44/45/46 with full u32 ID; previous rules reject them structurally. Existing tags and older slots unchanged. Save: checkpoint.c46, no automatic migration.

## Play

Launch --c46 or --c46 --pixel. ASCII quests, questlog [count], acceptquest [ID], declinequest [ID], abandonquest [ID or shrine/market]; if multiple remembered commitments are ambiguous specify ID. questdebug is developer-only. Normal inspection is read-only. Pixel: F11 known quests, F12 accept the local offer, 7 decline, 8 withdraw shrine, 9 withdraw market, 0 last two receipts; H deliver, I sell. F6-F10 history/caches/save-load and prior controls are retained. Local boards show active offers automatically. EN/ID labels distinguish quote, escrow, paid and unpaid.

## Acceptance

Targeted domain tests for one-time transfers and bonus, refusal and insufficient funds, cancellation/refund, external closure, deadline/horizon, bounded receipts, revision admission, tamper rejection and stale knowledge. Six seeds x 2000 passive turns compare C4.5 public simulation fields, material/coin conservation, positions and replay; four seeds x 1000 mixed turns compare deterministic twins. Six real canonical movement traces for shrine/market completion, decline and withdrawal. Frozen passive and branch digests, bilingual CLI persistence and Bevy input tests. Windows gate evidence separate. Benchmark is descriptive, not a proof of production performance.

Scope limits: only two material requests; no generic quest DSL, arbitrary narrative tasks, faction-wide service reputation, full lore import, or human playtest. Local implementation decisions: two boards, shortfalls/quantity, 240/96 turn policy, one-coin market bonus, receipt cap. No dependencies, authored content packs or lockfiles changed; no GitHub push.
