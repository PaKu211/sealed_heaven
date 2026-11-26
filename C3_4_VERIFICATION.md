# C3.4 — Autonomous Character Agency (revision 24)

## Baseline and identity

- Branch: `feat/c34-autonomous-character-agency`
- Verified base: C3.3 commit `a3d13a1f522a29f24e4dd309e6bebfbcab2324fb`
- Implementation commit: `9918d20`
- Checkpoint revision: `24`
- Domains: `SH:C10:RULES:v24`, `SH:C10:STATE:v24`
- Default save: `checkpoint.c34`; `--c33` retains revision 23 and `checkpoint.c33`.
- Platform/toolchain used for this gate: Windows x86-64, Rust 1.100.0.

The pre-existing local modification to `C0_5_VERIFICATION_LOG.txt` was preserved and excluded from the C3.4 commit.

## Implemented

- Extended the existing deterministic utility planner rather than adding GOAP or a second scheduler.
- Added persisted per-character ambition, wallet, livelihood provenance, suspended pursuit, decision reason, and a bounded 256-entry agency history with monotonic IDs.
- Ambition and suspended pursuits alter valid-goal priority; bodily danger, injury, fatigue, institutional work, martial training, cultivation, and routine duties still compete in one planner.
- Added a physical NPC livelihood/care loop:
  1. an injured character without care funds travels to the existing garden;
  2. one real garden herb enters NPC custody;
  3. the herb is sold to the existing market for conserved coin, or bartered as the treatment dose when clinic stock is empty;
  4. a present acceptable medic receives the fee (self-care pays a clinic till);
  5. one physical medicine and one wound are consumed/removed;
  6. persisted martial or other valid pursuits can resume.
- Reconciles livelihood provenance to physical NPC herb custody at every committed boundary.
- Limits a medic to one consequential care action per turn with a deterministic turn-local reservation.
- Preserves inherited spirit-ore ward protection in revision 24.
- Added witness-only, read-only `agency <visible npc>` and `agencylog` ASCII surfaces with English/Indonesian reason and transaction text. Exact wallets, ambitions, suspended goals, and remote decisions are not disclosed.

## Verification

Final debug gates:

```text
cargo test --locked --offline
  320 passed; 1 ignored; 2 doctests passed

cargo test --manifest-path apps/sealed_playable/Cargo.toml --no-default-features --locked --offline
  62 playable library + 31 integration = 93 passed
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

- injury -> gather -> sell/barter -> paid care -> recovery -> resumed partial training;
- empty clinic stock bootstrapped by one physical barter herb;
- absent medic/medicine and stale provenance cause no partial charge or underflow;
- one medic/two patients resolves one treatment per turn, leaving the loser uncharged;
- injured medic self-care while the player occupies the clinic tile;
- inherited ward behavior in revisions 23 and 24;
- ambition and suspended-goal priority effects;
- bounded history and monotonic IDs across eviction;
- save/restore during unfinished care work;
- witness-only and bilingual ASCII output;
- four seeds through 1,000 turns, with deterministic paired runs and restore checks at turns 200, 500, and 1,000.

Scripted public gameplay used `SEALED_ASCII_COMMANDS` to attack Yao Lian, wait through work and recovery, inspect `agencylog`, and read C3.4 help. The public log reported the witnessed one-coin wound-care payment without exposing remote work.

## Legacy evidence

Authentic fixtures were generated directly from detached baseline `a3d13a1`, then checked into `tests/fixtures/living/` and restored byte-for-byte by the revision-24 code:

| Fixture | Bytes | SHA-256 |
| --- | ---: | --- |
| `revision21.bin` | 438 | `1885c0a8038816740f789b07513bb22b62497f9747ac0049b8549648056f1d53` |
| `revision22.bin` | 438 | `d780f9356a494330b98fdd416e9fc7bd4a6b8fed2dec9185ab8d177a1abf2e2d` |
| `revision23.bin` | 438 | `3d1adef4407e490f88e36185fa71ea0e0ae21af6beea8c537ec47af899f6192e` |
| `revision23_wounded.bin` | 878 | `20fb3497d4e8f808794d5a547c4e84224ce6c097f9bf5f48df1262ca0e0117e7` |

Revision 21–23 fixtures also continue for 40 turns with byte-identical checkpoints. The active revision-23 wound/medic trace prevents revision-24 rest, payment, and self-care semantics from leaking backward.

## Performance

Release, seed 7, passive maximum-horizon probe (65,536 turns):

| Revision | Step total | Average/turn | Encode | Restore | Checkpoint | Agency history |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 23 | 461 ms | 7.045 us | 619 us | 466 ms | 327,798 bytes | 0 |
| 24 | 482 ms | 7.366 us | 751 us | 468 ms | 327,798 bytes | 256 |

These are single Windows measurements, not a cross-platform benchmark. Peak RSS was not measured. Restore remains linear in journal length.

## Audit and remaining risks

- Independent read-only gameplay and persistence reviews found unreachable zero-stock barter, stale herb provenance, lost revision-24 wards, non-behavioral ambitions, unlimited same-turn medic service, and private-state disclosure. All were repaired and regression-tested before the final gate.
- C3.1 is real but narrow: mentorship remains role-authored, and one autonomous Hall deputy is the principal learner.
- C3.2 retains a legacy school-Qi/form path alongside meridian Qi; unification is deferred because changing it would alter old campaign semantics.
- Older martial, meridian, and injury histories remain horizon-bounded rather than explicitly capped. C3.4 agency history is capped.
- Human playtest: **not run**. Automated Windows/headless/default-desktop compile and scripted gameplay: **pass**.

## Gameplay result

The player can now witness NPCs change course for injury and finances, observe physical work and paid/bartered treatment, and inspect witnessed causal reasons without omniscient access. NPC recovery and progression do not require player assistance. Different clinic stock, market liquidity, fatigue, threats, medicine contention, and ambitions produce different deterministic choices through shared systems.
