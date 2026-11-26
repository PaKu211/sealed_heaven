# C3.5 — Sect Hierarchy, Duties & Advancement (revision 25)

## Baseline and identity

- Branch: `feat/c35-sect-hierarchy-advancement`
- Verified base: C3.4 commit `e32e136`
- Implementation commit: `4f55d5f`
- Checkpoint revision: `25`
- Domains: `SH:C10:RULES:v25`, `SH:C10:STATE:v25`
- Default save: `checkpoint.c35`; `--c34` retains revision 24 and `checkpoint.c34`.
- Platform/toolchain used for this gate: Windows x86-64, Rust 1.100.0.

The pre-existing local modification to `C0_5_VERIFICATION_LOG.txt` was preserved and excluded from C3.5 commits.

## Implemented

- Deepened the existing Reed Hall `School`, `Politics`, `Martial`, `Meridians`, and `CharacterAgency` systems rather than creating parallel sect state.
- Extended `HallRank` to persisted `Apprentice`, `Disciple`, `SeniorDisciple`, and `Elder` tiers with persisted per-member merit, duty state (`SectDuty`), physical allocation state, and a bounded 256-entry hierarchy event history with monotonic IDs (`SectHierarchy`).
- Made rank affect real gameplay access and physical resources:
  - `Disciple`+ unlocks Reed Hall Qi sanctuary (`drawqi` at Reed Hall), `study` for Reed Strike, and waives Reed Hall martial tuition on `drill`.
  - `SeniorDisciple`+ unlocks one physical herb allocation from real `Region::hall_offerings` via `allocate` (`Intent::ClaimHallAllocation`), checked against Hall hours, mentor presence, contestation, and available stock.
- Added repeat Hall delivery duties (`duty`) for enrolled members using real `hall_offerings` -> Qinghe shrine `supply.shrine` transfers.
- Enabled autonomous NPC sect duties and advancement without player intervention:
  - `Luo Xiu` (`HallDeputy`) autonomously travels to Reed Hall when the leader is on duty, accepts real `hall_offerings`, delivers them to Qinghe shrine, reports back to the leader, earns merit, advances to `Disciple` / `SeniorDisciple`, and unlocks Hall Qi drawing.
  - `Qiao Lin` (`HallSteward`) starts at `Disciple` and earns merit when physically stocking Reed Hall (`SupplyHall`).
  - Succession (`update_succession` + `sync_hierarchy_office`) promotes the succeeding Hall deputy to `HallRank::Elder` and records `HierarchyEventKind::OfficeAssumed`; an Elder who mastered Reed Palm can also teach drills.
- Preserved physical custody & legacy invariants:
  - Reconciles external NPC herb custody changes against active `SectDuty::Carrying` at every committed boundary.
  - Preserves spirit-ore ward inheritance and `livelihood_herbs` reconciliation in revision 25.
  - Stonebank defection (`pledge`) remains blocked while carrying Hall duty and forfeits Hall hierarchy standing on completion.
- Added witness-only, read-only `sect`, `sectlog`, and `allocate` ASCII surfaces with English and Indonesian text. Unseen remote NPC duties remain private until witnessed.

## Verification

Final debug gates:

```text
cargo test --locked --offline
  329 passed; 0 failed; 1 ignored; 2 doctests passed

cargo test --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --locked --offline
  63 playable library + 31 integration = 94 passed
```

The same root and playable suites passed under `--release`.

Static/toolchain gates passed:

```text
cargo fmt --all -- --check
cargo check --all-targets --locked --offline
cargo clippy --all-targets --locked --offline -- -D warnings
cargo check --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --all-targets --locked --offline
cargo clippy --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --all-targets --locked --offline -- -D warnings
cargo check --manifest-path apps/sealed_playable/Cargo.toml --all-targets --locked --offline
cargo clippy --manifest-path apps/sealed_playable/Cargo.toml --all-targets --locked --offline -- -D warnings
git diff --check
```

Focused acceptance includes:

- full player advancement path (`join` -> `duty` -> `Disciple` -> free `drill` & Hall `drawqi` -> repeat `duty` -> `SeniorDisciple` -> `allocate` physical herb) with strict herb conservation;
- autonomous NPC duty execution and rank advancement for `Luo Xiu` and `Qiao Lin` across multiple seeds without player assistance;
- Hall offering contention between player `ClaimHallAllocation` and NPC `SectDuty` without minting herbs;
- mentor death blocking headless reporting and succession promoting `Luo Xiu` to `Elder` (and enabling drills if Reed Palm is mastered);
- Stonebank oath rejection while carrying repeat duty and rank forfeiture upon defection;
- revision-25 ward inheritance and committed-boundary custody reconciliation;
- bounded 256-entry hierarchy history with monotonic IDs and digest sensitivity;
- bilingual (`en`/`id`), witness-only, read-only `sect` and `sectlog` commands and `--c34` route isolation;
- four seeds (`0, 1, 2, 7`) through 1,000 turns with paired determinism, material conservation, and checkpoint restore checks at turns 200, 500, and 1,000.

Scripted CLI verification (`SEALED_ASCII_COMMANDS`) on `sealed_agency.exe` observed `Luo Xiu` autonomously completing a Hall duty at the shrine and advancing to `Reed Hall disciple` (`#8 t102 Luo Xiu promoted to Reed Hall disciple`) while `--c34` kept revision-24 rules and rejected `allocate`.

## Legacy evidence

Authentic revision-24 fixtures were generated directly from C3.4 commit `e32e136`, checked into `tests/fixtures/living/`, and verified byte-for-byte with 40-turn continuation replay:

| Fixture | Bytes | SHA-256 |
| --- | ---: | --- |
| `revision24.bin` | 438 | `60da49df938fb3c8b309d8420fe459afcdafbe3fb7d354165b5b92b63ee69a7c` |
| `revision24_wounded.bin` | 978 | `a642a3437ebab5de2e94863e4b4c1d726fe9cd092cfef7ac1fd8a381988a84d5` |

Revision 24 also rejects revision-25 `Intent::ClaimHallAllocation` before mutating state (`StepError::SchoolCampaignRequired`).

## Performance

Release, seed 7, passive maximum-horizon probe (65,536 turns):

| Revision | Step total | Average/turn | Encode | Restore | Checkpoint | Hierarchy history |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 24 | 497 ms | 7.599 us | 776 us | 493 ms | 327,798 bytes | 0 |
| 25 | 488 ms | 7.461 us | 737 us | 498 ms | 327,798 bytes | 13 |

Single Windows measurements; peak RSS was not measured. Restore remains linear in journal length.

## Audit and remaining risks

- Self-audit during implementation identified and fixed:
  - missing rank enforcement for Reed Hall `drawqi` in `src/living/meridian.rs`;
  - successor `Elder` unable to teach `ReedPalm` drills in `src/living/martial.rs`;
  - misleading `Rejection::RankRequired` copy when a `Disciple` attempts `allocate` in `apps/sealed_playable/src/living_ascii.rs`;
  - contested Hall state unnecessarily blocking NPC duty reporting inside `hall_leader_on_duty_for`.
- `SectHierarchy` focuses on Reed Hall's existing charter and members (`Mentor`, `HallDeputy`, `HallSteward`, and enrolled player); Stonebank remains represented by its single envoy (`Role::RivalEnvoy`) until a broader lodge roster is warranted.
- Human playtest: **not run**. Automated Windows debug/release suites, headless/default desktop compile, strict Clippy, and scripted CLI gameplay: **pass**.
