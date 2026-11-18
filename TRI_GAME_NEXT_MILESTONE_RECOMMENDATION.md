# Next milestone recommendation

Do not start C0.6 from this branch. Do not merge `research/tri-game-integration-spikes` into `main`. The experiments are not independently accepted; they need a separate review.

## What should be proposed, in order

1. **Observable daily phase for the existing 20 NPCs.**  
   Keep the kernel scheduler. Derive the in-game minute from the existing tick at 2 ticks per minute. Drive presentation only: Sleep, Eat, Work, Socialize, and Travel between places the current scene can show. Success is a player watching the 20 NPCs change activity with the clock, a 10,000-tick replay that still matches, and unchanged golden vectors. The headless roster already shows 180 phase changes per day and a clean restore. It is not yet on screen.

2. **Shrine supply as a desk over the C0.3 ledger.**  
   Only after the daily-phase review, and only if the owner wants hauling to be visible. Reuse actor-owned spirit herbs and the shrine stock. Demand, one open task per hauler, and the canonical checkpoint remain the source of truth. The spike already conserved 12 herbs, delivered 5, rejected a bad command without mutation, and replayed equal bytes. Multi-hauler contention is not yet proven.

3. **Leave navigation alone until the map grows.**  
   Hierarchical search lost to plain BFS on the small map and won on 64×64 and 128×128. Revisit it when a milestone actually introduces a map of that size, with the production tie-break and save format in the test.

## What should not be in that milestone

- Energy-queue combat, Z-level geology, a city-size exponent, a second tick, or a new resource ledger.
- Any rewrite of C0.1–C0.5 contracts, `demo.c03`, or player-agency input rules.
- A performance claim against the C0.4 figure of about 1.44 ticks/second. That sample was a debug foreground session. Kernel compute for 20 NPCs is about 0.21 ms per unpaced tick; the client budget is 500 ms.

## Review gate before any of this is accepted

- Independent reader, not this research session.
- Root `tools/c05_verify.ps1` still exit 0.
- The 20-actor checkpoint hash remains `32d87f149099b17d8eef0494afc3e42ce11d38743d8818b788a3ae326d74bedf` for the same 110-tick unpaced run.
- A native pass only if presentation changes. This research did not recapture frames.

## Reproduce the evidence

```powershell
git checkout research/tri-game-integration-spikes
powershell -NoProfile -ExecutionPolicy Bypass -File tools/c05_verify.ps1
cd research/tri_game
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings
cargo build --release --offline
.\target\release\tri_game.exe experiments
```

Expected test count is 6 passed. Player aggregates other than 20 NPCs exit 3.
