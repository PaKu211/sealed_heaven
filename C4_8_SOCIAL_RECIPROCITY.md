# C4.8 - Social obligations and reciprocity

Baseline: C4.7 db40b4b45300ca57d0bef8922a2a788d911f0f1b, accepted Windows distribution. New rules revision 35, reciprocity v1, isolated checkpoint.c48. No new dependencies, content pack edits, automatic save migration or remote pushes.

## Semantics

A sparse canonical unordered pair of living actor identities has a signed net gratitude balance, maximum magnitude 3. Positive from the creditor's perspective means the other owes goodwill. An actual committed aid action increments helper-oriented balance by one, cancelling one existing opposing unit first. This is not money, interest, property, compulsory debt, or global reputation. Overflowing the gratitude cap does not invalidate a real gift. No reward or penalty for refusing or ignoring a claim. Obligations survive parting; death is not remotely reported through the ledger. One-to-one, nontransferable, no third-party liability.

Sources: successful companion provisioning (accepted or spontaneous), successful C4.7 care funding, actual free clinic HP or persistent-wound care for the player, actual companion field care, and existing ordinary free NPC clinic HP treatment. Paid/bartered NPC wound care, institutional contracts, trade and mere dialogue do not create a second unpaid obligation. Existing C4.7 relation changes remain intact. Ordinary care is not gated on debt and can cancel goodwill. Repayment and waiver never grant additional trust. A newly initiated trusted-neighbor gift uses the existing directed aid regard effect; returns of gratitude do not.

## Player actions

requestaid ACTOR: face-to-face; requester must have a positive credit, mutual nonhostility and donor familiarity. Donor must be alive, full HP, unwounded, below rest threshold, not performing urgent danger/medical/livelihood action or active sect duty. Return one existing surplus personal coin (first choice) or packed herb, retaining one of either material. Never seize public cargo or mint goods. May be refused. repayaid ACTOR: voluntarily transfer one real player coin against a negative balance, same contact and nonhostility/familiarity, no extra relationship bonus. waiveaid ACTOR: release the whole positive balance face-to-face, no money or reward. No deadlines or coercive obligations. These are three new full-u32 actor-ID wire intents 51-53. Older revisions reject them atomically before time advances.

## Autonomous local assistance

After ordinary NPC commits and before personal request reconciliation, detect living NPCs with wounds, no clinic fee, and no personal barter stock. Reconcile provenance. A safe nearby donor who owes gratitude to the patient, or has familiarity and regard at least 2 toward the patient, may voluntarily give one personal coin or one personal livelihood herb, retaining one unit. Mutual nonhostility required. Donor and recipient must be in reach with line of sight. Snapshot candidate order: repay an existing obligation first, severe injury next, donor ID then patient ID. Commit-time revalidation and a used-participant set bound this phase to one transfer per participating actor. This is an extra local interaction phase, not a replacement goal, movement, or cure. NPCs still have to seek available treatment or barter normally. No transfer of institutional stock. No direct healing or economic generation.

## Knowledge and persistence

Full recent history: 96 aid events. Separately bounded 48 observed entries, so private world activity cannot displace witnessed entries. Only actual participants know their bilateral account and change. Witnesses see a nearby physical act if both parties are visible; their observation omits the third-party balance and delta. No inferred remote updates, omniscient disclosure or query-induced simulation advancement. Own ledger persists even when old receipts roll off. Stable ordered pairs and event IDs, canonical bytes bind both histories and balances. Digest and replay check all new state. Existing content identity and historical fixtures remain unchanged.

## Controls

--c48 [--seed N] [--pixel | --debug-world]. CLI: bonds [count], aidlog [count], requestaid ACTOR, repayaid ACTOR, waiveaid ACTOR. bonddebug developer-only. Pixel: Ctrl+F1 own goodwill (selected target if known), Ctrl+Shift+F1 latest witnessed aid; comma request, slash repay, semicolon waive. Explicit adjacent target or unique nearby person required for new actions. Existing C4.7 F1/Shift+F1, Q, Backspace, Delete, period, O, and other controls remain. Tab EN/ID. Pixel view is compact; ASCII provides longer lists. Save checkpoint.c48.

## Evidence and scope

Seed 0 accepted C4.7 journey reused: first pack at tick 32, reserve refusal at 33; second pack at 33, reciprocal herb at 34, release at 35; part at 34 retains both goodwill units. Actual player receives clinic treatment and repays a coin at tick 35 on a separate seed-0 route. Seven frozen endpoints plus 18 passive hashes. Controlled domain fixtures test coin/field care, NPC-NPC coin or herb aid, competing donors, reserve/hostility/refusal, paid clinic treatment, private versus observed memory, bounds and replay. Six passive seeds x 2000 turns compare C4.7 world state and conservation; four mixed seeds x 1000 deterministic twin turns. Autonomous NPC-NPC emergency aid is fixture-verified, not established as a naturally encountered passive event. Design choices: cap 3, reserve 1, trust threshold 2, history/observed caps 96/48, local extra phase. This is not arbitrary dynamic diplomacy, a lending market, a full interpersonal relationship simulation, or complete authored lore integration. Windows-only automated acceptance; not human playtest or independent audit.
