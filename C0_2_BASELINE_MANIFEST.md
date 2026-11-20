# C0.2 Baseline Manifest

Version 1.0 · 17 November 2026  
Status: frozen. C0.3 implementation is not authorized.

This manifest identifies the approved C0.2 source. It does not change that source. Reproduction uses the archive below and the commands in §5. Do not edit files inside the archive to match a later tree.

## 1. Approved artifact

| Item | Value |
| --- | --- |
| Archive | `Sealed_Heaven_C0_2_Source.zip` |
| SHA-256 | `2804d33a785acb7331a0d82fd1c080bec82d0add348578abb4b1a6807db60a40` |
| Root inside the archive | `sealed-heaven-c0.2/` |
| Members | 29 |
| Independent audit | `C0_2_INDEPENDENT_AUDIT.md`, verdict **PASS** |
| Normative C0.2 documents | `docs/C0_2_IMPLEMENTATION_SPEC.md`, `docs/C0_2_ACCEPTANCE_TESTS.md`, `docs/C0_2_ARCHITECTURE_REVIEW.md`, each version `0.1-review-r1.1` |

The hash above was recomputed on 17 November 2026 and matched the approved value. The C0.1 source baseline used by that audit remains `Sealed_Heaven_C0_1_Source (1).zip`, SHA-256 `9c3b7907845c8cf0ff2e757c9a5d4125a4264ab6977b7513b1300f16aaa51250`.

## 2. Toolchain and dependencies

| Item | Frozen value |
| --- | --- |
| Rust | 1.100.0, commit `0ef1bc386059596dada8fb65b50425aab2ea637a` |
| Pin file | `rust-toolchain.toml` (`rustfmt`, `clippy`) |
| Package | `sealed_sim` 0.1.0, edition 2024 |
| Direct dependencies | `rand_chacha` `=0.10.0`, default features disabled; `sha2` `=0.11.1` |
| `Cargo.lock` SHA-256 | `176e4bb5d145739c101c10296ce9f97d4a24917381be057e74134e0ba1779d75` |

`Cargo.lock` and `src/tests/golden.rs` in the C0.2 archive are byte-identical to the approved C0.1 archive. `Cargo.toml` differs only in the package description. No Bevy, serde, or additional direct crate is present.

## 3. Source and tests

| Role | Path in the archive |
| --- | --- |
| Public API and tick phases | `src/lib.rs` |
| C0.1 observation and event bytes | `src/encoding.rs` |
| `SH:C02:STATE` revision 1 | `src/domain.rs` |
| C0.1 tests and golden literals | `src/tests/mod.rs`, `src/tests/golden.rs` |
| C0.2 tests and transcribed vectors | `src/tests/c02.rs`, `src/tests/c02_golden.rs` |

Acceptance ownership:

| Group | Owner document | Result on the audited Windows host |
| --- | --- | --- |
| C01-T01 through C01-T06 | `docs/C0_1_ACCEPTANCE_TESTS.md` | Pass |
| C02-T01 through C02-T04 | `docs/C0_2_ACCEPTANCE_TESTS.md` | Pass |

The independent run used Rust 1.100.0 on `x86_64-pc-windows-msvc`. Debug and release each executed 25 unit tests and 2 doctests, with zero failures and zero ignored tests. That host does not certify R2.

## 4. Published golden identities

These values are frozen. C0.3 must not replace them. The full checkpoint hex lives in `docs/C0_2_ACCEPTANCE_TESTS.md` §11 and in `src/tests/c02_golden.rs`. The C0.1 checkpoint, event, and RNG literals live in `src/tests/golden.rs`.

| Identity | Value |
| --- | --- |
| C0.1 content SHA-256 | `042797b9b5032aaf66af7473dd8ff50a349c865a5b4f147405446f78dad96c59` |
| C0.1 H0 | `87cb000d61574a1611078311af86095b5b9c51b780f01d65447da0f549233874` |
| C0.1 H2 | `54fabd2a30f91a021e8593391702c18e09e7ff407f0158ea5b532f86ac39acba` |
| Approved event 3 digest | `1a1cc22bf1041c19845bceb790e6761dad0edaf41cb3eca1f90b0ce06f1875b1` |
| Fixture 2 content SHA-256 | `043d3507909cf4718d94488279deb1943dfa69530f9e86b109e381c3cbd2dd92` |
| Fixture 2 H0 | `413408f703da057f8b39023e580c8c65649376824d1a77be0a4d24aaf048dd0b` |
| T01 domain save, 312 bytes | `446a3b22d12c800c95c40811038531422afe419fb8b9923bba2df6381c3ee6f7` |
| T01 domain final, 287 bytes | `53939438b57d0dcc685929484ac15f54eea1a5f82fff44b8862943de2fadf3ae` |
| Witness tick 1, 566 bytes | `35b5104ca23dc7723baffbe76948a044fdda583eb8374604149133986e2f4ac2` |

The superseded witness digest beginning `12d94a56` is not part of the baseline.

## 5. Reproduction

Extract the archive. From `sealed-heaven-c0.2/`, with the pinned toolchain:

```text
cargo fmt --all --check
cargo check --locked --offline
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --locked --offline
cargo test --release --locked --offline
```

`--offline` is valid after the lockfile dependencies are already in the local cargo registry. Do not regenerate section 11 or `golden.rs` if a result disagrees. Fix the code under a new authorization, or return the specification for review.

## 6. Frozen behavior that C0.3 must preserve

- Fixture 1 `step` and `drive` keep C0.1 command order, event bytes, digest chain `SH:C01:EVENT:v1`, and ChaCha12 probes under `SH:C01:RNG:v1`.
- `checkpoint_bytes()` remains the C0.1 observation. It is not a C0.2 save. `SH:C01:STATE` restores as `UnsupportedFormat`.
- Fixture 2 uses `drive`. `step` returns `UseDrive` and does not mutate.
- Fixture 2 phases are mode, then command, then work. Synthetic tasks due at the staged tick complete even when the actor is disabled. That rule is the C0.2 LOD fixture, not the production job rule.
- Record IDs are monotonic and are not reused. Domain saves are `SH:C02:STATE` revision 1 plus a SHA-256 trailer.
- A failed `step` or `drive` leaves the previous domain bytes unchanged.

## 7. Known limitations carried forward

- R2 cross-platform bitwise replay is not certified.
- `write_domain_checkpoint` is an ordinary file write. A crash can leave a torn file. Restore rejects it. The simulation is unchanged when the write fails.
- There is no migrator from `SH:C01:STATE` or from any revision other than `SH:C02:STATE` revision 1.
- `ticks_per_game_day` is not a revision 1 field.
- Production activation without an explicit `SetMode`, travel approximation, aggregate cohorts, needs, jobs, navigation, and production reservation expiry remain outside this baseline.
- The C0.1 implementation report records a Linux run and says independent C0.1 sign-off was pending. The C0.2 audit compared the preserved C0.1 files with the approved C0.1 archive and re-ran those tests on Windows.

## 8. Deferred requirements

| Requirement | Owning gate | Why it is not in C0.2 |
| --- | --- | --- |
| Production reservation lifecycle, expiry, cancellation, and fairness | C0.3, C03-T01 | Core §6, Core §7, ADR-013, M0 §6 |
| Disabled actor cannot continue an invalid production job | C0.3, Core invariant 4 | C0.2 completes the synthetic task instead |
| Needs, jobs, ledger transactions, initial navigation | C0.3 | README, Technical Architecture §2 and §10 |
| Applicable contention, transaction, and invalidation scenarios | M0-T06, M0-T07, M0-T08 | M0 matrix; C03-T01 says to run them before promoting C0.3 |
| First real schema migration | M0-T11 | C0.2 revision 1 has no successor |
| 30-day scenario and day length | M0, ADR-024 | Not required by the C0.2 absolute-tick fixture |
| Presentation and host cadence | V0, I0 | Not part of `sealed_sim` |
