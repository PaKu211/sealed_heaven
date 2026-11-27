# C5.0 - Autonomous social networks

Baseline C4.9 018a6407dcc09872c4d050e8b31f26822836a69e. Rules revision 37; isolated checkpoint.c50. No dependency or authored content change, automatic save migration, or remote push.

## Autonomous interaction
After ordinary NPC commits and reciprocity transfers, eligible named NPCs may exchange a brief greeting. Both must be alive, healthy, unwounded, below or at their rest threshold, without active sect duty, in an allowed routine activity, adjacent and mutually nonhostile. Actual positional line of sight is required. No added travel or interruption of existing goals. Candidate generation uses indexed four-neighbor occupancy, ordered oldest pair greeting first then stable IDs, revalidated at commit. At most one greeting per participant per tick. Any greeting, aid or damaging conflict imposes a 24-tick actor interval before an optional greeting; the same pair waits 96 ticks between greetings. These are provisional tuning decisions (12 and 48 in-game minutes).

A greeting increases existing bilateral familiarity only; it cannot fabricate regard, gratitude, money, cargo, healing or hostility. Familiarity can enable existing trusted-news and reciprocity gates, subject to their other conditions. Actual existing NPC-NPC aid transfers/care and damaging combat update the pair memory with direction and retain their original consequences. No second trust change. No zero-damage, waiver or player action recorded as third-party aid or conflict. This phase does not replace NPC goals or implement full conversations, families, political alliances, reconciliation or deliberate social travel.

## State and knowledge
Sparse canonical unordered pair graph: first/last actual tick, greeting count, last greeting tick, directed aid/conflict counts. Actor last activity is persisted; event history capped at 128, independent witnessed history capped at 48. Only a living player seeing both participants at the act receives the historical observation. No private relation scores or debt, no remote updates, no inferred current locations. Raw graph is developer-only in the client. Bounds depend on persistent named actor count and simulation horizon. Canonical digest binds graph and both logs; ordinary intent replay reconstructs the state. Older revisions keep their own rules and checkpoint bytes.

## Controls
--c50 [--seed N] [--pixel | --debug-world].
ASCII: sociallog [count] (1-48, default 12), socialdebug only in debug-world.
Pixel: Ctrl+F3 most recent witnessed interactions, F3 existing regional news. Existing contacts/aid/favor controls remain. EN/ID.
Inspection and locale changes are read-only.

## Verification plan
Natural passive greetings across multiple seeds; deterministic twin/replay stress, eligibility/cooldown/fairness/knowledge, real resource aid and damage hooks, old goldens, new bilingual frozen checkpoints, native projection, localization, Windows release and artifact reproducibility. Independent audit and human playtest remain separate. A plan is not acceptance evidence.
