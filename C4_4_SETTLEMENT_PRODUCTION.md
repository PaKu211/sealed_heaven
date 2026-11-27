# C4.4 — Settlement production & site scheduling

Baseline C4.3 0183dd160971fc24ad3957b46343e010ff8eee97. This is the first exact L2 site-process foundation in the Master GDD §7.4. It is deliberately not an implementation of anonymous population cohorts (ADR-010 remains deferred), a complete settlement economy, or a claim of 200+ live NPCs.

## Contract

Revision 31 inherits all revision-30 named actors, canonical clock, geometry, resource quantities, jobs, conflicts, L0/L1 and information rules. No new resource stock, actor, automatic teleportation, fast-forward, or new random stream. Existing periodic natural-growth and order processes are replaced in the new revision with a deterministic due calendar. Old revisions keep the original functions.

Five stable job identities in ascending phase order: WestGrowth, GardenGrowth, OrderArrival, OrderDispatch, GroveGrowth. Three growth jobs recur at the existing generator-derived cadence; OrderDispatch repeats every 48 turns; OrderArrival runs once 30 turns after each dispatch. At tick start after incoming information and before the player's action, take due entries in (due tick, job ordinal) order. The order preserves the legacy phase semantics: west growth, garden growth, arrival, dispatch, grove growth. The shortage calculation reads shrine stock and physical cargo carried by living suppliers/couriers only. A request is information/demand, not a physical herb reservation or shipment. Arrival sets the existing order balance; actual load and delivery still require the NPC at the correct site and existing commit-time checks. If a future ruleset permits overlapping requests, replacement must cancel the prior pending arrival (the immutable current content has travel < dispatch interval).

A growth check at capacity adds zero; otherwise add one to the existing source inventory and increment its existing regrown counter. No duplicate resource owner. Each job can have at most one pending execution; its future due time advances only when it executes or an explicit replacement cancels it. Past due entries are internal invariant errors, not silently fast-forwarded. All player-facing transitions remain on the same logical tick as C4.3. Five bounded per-job records retain execution count, cumulative amount, last execution tick and last amount. Growth quantities count actual herbs produced; order quantities count requested herbs, not physical deliveries. Counts reconcile to the original authoritative inventory counters and pending order. The agenda is bounded to four recurring entries plus at most one pending arrival. No unbounded archive.

## Player presentation and save

Run sealed_agency.exe --c44 [--seed N] [--pixel | --debug-world]. Use production in ASCII or F4 in Pixel when within one Manhattan tile and with line of sight to the Qinghe herb patch, Willowford garden, or Pineford grove. Shows current stock/capacity, interval and turns until the next growth check, which is not a promise of new stock. A dead player or a remote player receives no site report. This is a read-only view, not persistent remote knowledge. Existing news/chronicle remain unchanged. Developer-only productionlog reports all job identities and counts. EN/ID presentation uses seven added locale keys and external catalog support.

The new checkpoint.c44 uses revision 31 and binds calendar and records in deterministic semantic order to the state digest. Restore reconstructs from the seed and journal and verifies that digest. Old slots and content pack identity are preserved; no automatic migration or checkpoint relabeling.

## Acceptance

Exact projected World equality and state digest against C4.3 for 12,000 passive and 4,000 mixed-action turns plus actual journey to all three production sites. Verify per-tick job count, due date, physical regrown counts, dispatch/arrival accounting and pending request. Check full-capacity no-op, growth at due boundary, duplicate schedule rejection, stale command rejection, tamper detection, save/load one turn before order arrival, and calendar-only progression to 65,536 turns. Freeze 6 initial checkpoint digests, 6 tick-1000 checkpoint digests, 6 state digests. Verify bilingual real CLI and Bevy F4 input, isolated save slots, unchanged historical goldens, offline pinned Windows debug/release/Clippy builds, native GUI and distribution smoke. Record actual results separately.

## Limits

No generalized recipes/workshop input consumption, anonymous workers, population aggregates, seasons, new lore, prehistory, longer horizon, or L3/L4 strategic simulation. Three sources and one inter-settlement request flow. This is a small due-work architecture proof rather than a performance guarantee; benchmark the current workload separately. Windows automated review only, no hosted CI or independent human playtest. New revision and milestone names are local engineering decisions under the user's authorization to develop the GDD iteratively.
