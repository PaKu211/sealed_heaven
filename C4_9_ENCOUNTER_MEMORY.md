# C4.9 - Encounter memory and social continuity

Baseline C4.8 011433d. New deterministic rules revision 36, isolated checkpoint.c49.
No new dependencies, automatic save migration, new command intents, or content changes.

## Mechanics
- Successful direct Talk records first/last confirmed tick, observed tile, visit count and last interaction kind for the actual NPC. A failed/refused/unreachable talk does not record a contact.
- Ordinary conversation raises bilateral familiarity at most once per actor per 12 ticks (6 in-game minutes). Other effects of talk, including regional news and favor discussions, still execute. Real clinic care and other existing aid actions keep their original rules.
- A real successful clinic treatment is a recorded visit. It does not consume the ordinary greeting cooldown. Empty/away clinic rejection does not produce an encounter.
- Storage is one record per directly encountered persistent actor, ordered by ID canonically. There is no remote tracking, fabricated meeting, current location disclosure, or unseen death update. Read-only contacts are sorted by last confirmed tick descending then actor ID.
- State digest includes the record. Checkpoint replay reconstructs it from ordinary existing intents. C4.8 and earlier retain their exact rules and checkpoint formats.
- Cooldown is a provisional balancing choice, not a full interpersonal conversation simulator. Waiting still advances the world; legitimate aid is not gated by the greeting interval.

## Controls
--c49 [--seed N] [--pixel | --debug-world]
ASCII: contacts [count] (default 12, 1-48).
Pixel: Ctrl+F2, most recent two contacts. F2 still surveys terrain.
EN/ID projection, no simulation mutation on inspection or language switch.

## Acceptance
Run new domain and CLI tests, frozen C4.8 endpoint regressions, locale parity, core and client build/check and format. Desktop projection/native window and full regression must be verified before claiming Windows acceptance. The manual playtest and independent external review remain separate. This document is a design and test plan, not evidence that any gate passed.
