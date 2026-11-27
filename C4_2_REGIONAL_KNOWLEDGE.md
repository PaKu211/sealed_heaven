# C4.2 — Regional knowledge and causal economic history

## Scope and lineage

Standalone Rust/Bevy game, not a SillyTavern extension. Parent C4.1 commit 4e04756311736f55d318f85c9a84d5ae3e647957. Revision 29 deliberately changes frontier decision rules; revisions 1–28 and their golden checkpoints remain immutable. The original v1 registry, Pineford overlay, generator v1, maps and all actor identities remain unchanged. The milestone advances the GDD's situated information, physical logistics and readable history principles, not a claim of complete world simulation.

## Situated knowledge contract

Four stable regional observation sites: Qinghe shrine, Willowford market, Pineford depot and Pineford grove. At initialization and at the pre-/post-NPC commit barriers, a living actor at Manhattan reach <=1 with line of sight observes actual stock. A receipt contains the original witness, observation tick and within-turn phase, immediate sender (if reported), acquisition tick and hops. Newer observations win; firsthand beats relayed knowledge at the same observation stamp. Observation at a site does not retroactively identify unwitnessed historical events.

NPCs exchange public regional facts at adjacent, mutually nonhostile contact when the sender's authored capability permits news sharing. The player must explicitly use the existing successful talk action; there is no passive remote gossip acquisition. A conversation queues at most one report per recipient/site and one regional event per recipient. Arrival is two turns later: an already-heard conversation can be remembered after the speaker leaves, but dead recipients, stale reports and retired events do not receive new knowledge. No four-hop forwarding; origin is never rewritten. Direct newer evidence cancels inferior pending reports. Contact indexing uses occupied neighbor cells with stable sender/recipient ID order, verified against the ordered-pair reference.

Freshness for decision-use/forwarding: 120 turns from original stock observation, not the most recent retelling. Event reports expire after 640 turns from occurrence. Retained stale receipts remain inspectable but do not authorize trade planning. A recent estimate is not guaranteed current stock. No truth inflation, fabricated rumors, or reputation score changes are added by this public-news channel.

## Behavioral consequences

Li Wen (37) has no initial remote Willowford inventory. Without fresh demand evidence he surveys Willowford physically. With fresh demand and depot surplus information, he travels to the depot; local reserve is checked at loading and quantity is limited by remembered market demand, not by an omniscient destination read. An already loaded shipment is delivered even if demand changed since observation. With stale/unknown remote market stock he must resurvey (unless a newer report arrives). Han Yu (38) uses dated depot and grove observations to decide when to survey, gather and replenish. Local source stocks remain commit-time authority. Existing higher-priority threats, treatment, travel constraints and fatigue still apply. This is intentionally scoped to the new Pineford roles; legacy NPC decision systems may still use older aggregate/global signals. No change to physical resource or coin conservation.

## Chronicle and bounds

The new regional economic chronicle records actual nonzero frontier harvest/stock/load/delivery, Willowford market stock/load/trade, Bao Jun delivery to Qinghe, shrine contributions and changes in shortage bands at observation barriers. A shortage event's amount is the stock sampled at that barrier; other quantities are committed transfer amounts. Multiple within-turn stock fluctuations are not a complete transaction log. Firsthand event knowledge is given to present living observers at the site who can see the acting character when applicable. Only witnessed/relayed events can appear in ordinary player news. Institutional disputes and violence remain in their existing dedicated histories; the regional chronicle does not invent new versions of those facts.

A 128-event ring retains the most recent events with monotonically increasing identities (no ID reuse). Eviction purges associated operational receipts and pending reports. At current population, at most 13 x 128 event receipts, 13 x 4 stock receipts and 13 x 5 pending entries. Historical receipts held by dead actors are retained for provenance until retirement. No external clock, network, stochastic gossip or unbounded log. This is a bounded operational chronicle, not a full procedural prehistory or permanent multi-century archive.

## Persistence and play

Rules revision 29; checkpoint.c42 is separate from checkpoint.c41, checkpoint.c40 and all older slots. A new scenario digest domain binds revision and existing authored/generated geography. Canonical news payload includes shortage bands, monotonic ID, event fields, knowledge and pending queues in stable order. Restore reconstructs all of them from seed and journal, then checks the state digest. No automatic migration or direct editing of a historical save.

    sealed_agency.exe --c42 --seed 7
    sealed_agency.exe --c42 --pixel --seed 7
    sealed_agency.exe --c42 --debug-world --seed 7

ASCII: talk <actor ID>, news, newsnext (cycle one entry), help, survey. The news views do not consume turns; talk is a normal canonical action. After a successful conversation, allow two turns for the queued report. Developer mode alone enables chronicle (last 16 canonical events). Pixel: E to talk to a nearby selected NPC, T selects target, F3 cycles the player's known news, F2 surveys terrain, Tab EN/ID. New text is in the existing locale packs. No new intent wire tags or runtime dependencies.

## Acceptance

- Seven targeted domain tests: counterfactual hidden stock changes, contact and three-hop provenance, superseded observations, death/hostility/expiration, no retrospective witness, bounded eviction, pending-state hashing, spatial-index/reference parity.
- Six seeds x 2,000 turns: every committed live position valid/unique and <=1 tile displacement, actual freight crosses the eastern river, both freight routes' counters match chronicle amounts, conserved herb/coin totals, all knowledge receipts causal, distant reports present, idle player no remote facts, replay at four checkpoints.
- Actual player contact in two seeds, avoiding local Willowford market observation: no receipt on talk or first wait, receipt on second wait, replay of in-flight reports exact.
- Reviewed six-seed golden CSV: six map hashes, twelve checkpoint hashes, six terminal state hashes plus event and receipt counts; frozen before spatial-index optimization and required exact thereafter.
- Rehashed tampering and foreign revision rejection, historical admission matrix, full debug/release core and client checks, localized real executable and native Bevy tests. Separate acceptance evidence is required before release.

Limits: Windows-only automated validation; no independent human audit, human playtest, hosted CI, Linux build, large-population LOD, procedural prehistory, false rumors or generalized epistemic NPC planning. The new geography and provisional names are inherited unchanged from C4.1.
