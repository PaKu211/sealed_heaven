# Sealed Heaven — GDD implementation pass

Status: **implemented in source, pending compilation and runtime verification.**

| GDD stage | Implemented in this pass | Acceptance still open |
| --- | --- | --- |
| A. Proven local simulation | Retained the frozen C0.x sources and legacy entry points. Defined the living world's command boundary below. | Native regression tests and genuine older save samples. |
| B. A village worth inhabiting | Resolved the three required audit defects in the current rules. Added EN/ID presentation, actual dusk-to-dawn routines, and local recognition of shrine contributions during a shortage. Authored regression scenarios for a blocked passage, recipient death, hidden targets, and crossing dusk through save/load. | Native multi-day playtest, local occupation breadth, route travel and return. |
| C. RPG vertical slice | Implemented the supplement's first cultivation consequence. At the shrine, a carried herb can instead fund one turn of breath practice. Three practices improve guard from blocking one point to blocking two. Both uses reach the same journal, ledger, and save boundary. | Trading, character backgrounds, neighboring sites, pixel presentation, and the GDD's 30–60 minute acceptance session. |
| D–F. Regional sandbox, sect society, immense world | Reserved for after the local and vertical-slice gates. | Regional travel and identity, faction logistics, deeper cultivation, world generation and measured simulation scaling. |

## Audit decisions

- **C10-01** — Care selection in revision 2 requires available medicine, a living healer who will accept the patient, and a route to the clinic. The patient can choose another activity if the path is temporarily blocked and reevaluate on the next turn.
- **C10-02** — A report's designated recipient must be alive and reachable at the known or perceived meeting site. A trusted living contact can substitute in stable actor-ID order. Actual receipt still requires a conversation. Pending delivery to someone who dies grants no knowledge.
- **C10-03** — The regular terminal shows the living player's LOS and range. It shows directly observable facts such as carried herbs, the staffed clinic, and local stock. Guessed hidden target IDs and unknown IDs give the same hint without advancing time. Full positions and goals are available through `--debug-world`.
- **C10-04** — The living world is an independent simulation domain. An admitted command addresses one exact turn boundary and supplies one player intent. Wrong boundaries and unsupported rules are rejected before mutation. An admitted action can fail locally while autonomous activity still advances. A repeated call for an outdated boundary cannot reapply the intent. NPCs propose from one committed view and execute in stable actor-ID order. These semantics cover one player and autonomous NPCs. The older kernel's multi-producer identity, batch deduplication and transactional event contract require a separate parity design before migration.
- **C10-05** — Restore still replays the full journal. The path search uses a dense grid for first-step bookkeeping, preserving its original visit order. A model comparison covered 589,824 bounded navigation inputs. The restore example measures 4,096, 16,384 and 65,536 turns and accepts additional recorded Qinghe saves as arguments. Initial target for a release build is p95 ≤ 2 seconds at the horizon on the i7-12700F development host, with a separate measurement on the 4 GB laptop. A snapshot design remains conditional on timing and replay equivalence.
- **C10-06** — `lang id` and `lang en` switch terminal presentation without affecting checkpoint bytes. Input tokens and authored proper names are shared. The developer view carries the same localizations.

## Checkpoint compatibility

New campaigns write revision 2 to `checkpoint.c10`. Revision 1 runs using its original calendar, goals and delivery rules, and saves back as revision 1. Starting a new campaign enables revision 2. The reader selects the rules by header before checking the scenario fingerprint and replay digest. A test includes an independently encoded initial revision 1 vector, plus a replay/resume check. Earlier C0.x saves still belong to their own clients.

## Native checks for the next pass

Run the root and headless application tests on the pinned toolchain, then smoke the graphical compatibility entry points and validate both original Qinghe combat timings through player-facing output. Check the day/night commute, supply after a witnessed gift, breath practice versus clinic use, and restore measurements.

```sh
cargo test
cargo test --manifest-path apps/sealed_playable/Cargo.toml --no-default-features
cargo run --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --bin sealed_agency
cargo run --release --example living_restore
```
