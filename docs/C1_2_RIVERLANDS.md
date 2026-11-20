# Riverlands development pass

Status: **implemented in source, pending compilation and native playtest.**

| GDD stage | Implemented in this pass | Acceptance still open |
| --- | --- | --- |
| B. Village life | Walk out of Qinghe and return. The village and its inhabitants continue on the same clock. A player can harvest, trade, or contribute to shrine supplies. | Multi-day playtest on the native build. |
| C. Vertical slice | Two reachable sites, three starting backgrounds, a finite coin pool, price changes based on stock, breath training, localized help and inspection, and a pixel window. The ASCII and graphical clients submit the same actions. | Desktop and terminal runtime checks, balancing, and the 30–60 minute unfamiliar-player session. |
| D. Regional sandbox | Two settlements share a herb economy. Seed chooses one of two authored road layouts and initial market stock separately. Willowford's grower stocks its market. Local orders take 30 turns to reach the market and a named courier carries physical herbs to Qinghe. | More settlements, procedural geography and prehistory, full factions, regional events, and measured off-screen simulation policies. |
| E. Cultivation society | A named mentor admits the player to Reed Hall as an apprentice after a breath lesson and a one-herb offering. Members can convert two carried herbs into a portable remedy when the mentor is on duty. | Additional ranks, schools, lineage, sect politics and succession. |
| F. Large world | The finite, authored regional slice remains the current implementation. | Biomes, ruins, long histories, content packs, and performance evidence. |

## Playing

The normal client starts in Qinghe. Optionally issue `seed 1` at turn zero for the lower road. Choose `background herbalist` to gather up to two herbs per action, or `background peddler` for more starting coins. Taking another first action selects Wanderer. Head east past the outrider and cross the mountain pass to Willowford. The market buys and sells one herb at a time during daylight. Gathered or purchased herbs can go to the shrine, breath practice, or Reed Hall.

```sh
cargo run --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --bin sealed_agency
cargo run --manifest-path apps/sealed_playable/Cargo.toml --bin sealed_agency -- --pixel
```

Use `help`, `inspect <id>`, and `lang id` in the terminal. The pixel window displays controls, local services and nearby targets. It shares the normal save path `checkpoint.c10`.

## State and compatibility

- New campaigns use checkpoint revision 3. The seed is in the header and participates in the complete state digest. The seed, typed scenario, action journal, and digest reconstruct cargo, outstanding orders, scenery, coin balances, school membership and remedies. Revisions 1 and 2 still open and replay with their original campaigns. Reset starts a fresh regional campaign.
- Coin endowments, purchases and sales transfer coins between the player and market. All herb movements are transfers except explicit regrowth, clinic consumption, breath practice and brewing. The tests count offerings, remedies, and consumed herbs when checking conservation.
- Qinghe's shrine target triggers orders rather than imposing a warehouse capacity. Delayed orders can see demand that has since changed. Freights already picked up remain in the courier's inventory and can still be delivered. Willowford keeps two herbs in reserve for courier decisions. Goods that named actors carry persist at death.
- The graphics layer uses the same player visibility as the terminal. It draws only from simulation state and sends keyboard intents to the same command adapter.

## Release verification

Native verification still needs the root and headless application test suites, a desktop build and pixel smoke, both routes with a mid-transit save, a return trip with a purchased herb, and a visit to Reed Hall during and outside duty hours. Restore still replays the full input journal. Re-measure restore on the wider map before drawing conclusions about scale.
