# C2.3 implementation report

Player agency is a revision-14 campaign on branch `feat/c23-player-agency`. The frozen C2.2 baseline remains `4f7bdd69d36856daa55d789997ca0351a5235952`. `--c22` still starts revision 13. The C0.1 kernel is unchanged from `7694ce2`.

The player is actor 21. Knowledge uses the existing event ledger, provenance, pending reports, and institutional disputes. New intents are `Examine`, `Ask`, and `Testify`. They advance canonical time. `know` only reads the player's ledger.

What the player can do:

- Witness an audit only when standing in reach of the site and able to see it. Distant sight does not grant the finding.
- Examine that same site later. The receipt is a personal check: reported by the player, one hop, origin the institution that recorded the finding.
- Ask a living NPC in reach. Their account is queued and arrives after the existing report delay. The player initiated the meeting, so the usual trust gate is skipped. The receipt stays their account, one hop.
- Tell a known finding to an NPC in reach. Firsthand or personally examined evidence is offered at one hop and can open or update a dispute. An account the player only heard is offered at two hops and is remembered without opening a dispute. Unknown, distant, duplicate, or weaker repeats are refused.

Institutions still decide through C2.2 disputes. A later delivered finding can restore cooperation. It does not rewrite the earlier failure. Withholding testimony leaves the second institution untouched. The world continues either way.

Checkpoint revision is 14. Player knowledge, pending testimony, disputes, and `next_event` are inside the state digest. A forged knowledge entry changes the digest and restore fails. Older revisions are not upgraded.
