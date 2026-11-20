# Sealed Heaven C0.2 Implementation Report

17 November 2026

**Verification result: C01-T01 through C01-T06 and C02-T01 through C02-T04 passed on this Windows host.** Debug and release each completed 25 unit tests and 2 doctests, with zero failures and zero ignored tests. This is the implementation agent's verification. It is not cross-platform R2 certification, and it is not the independent C0.2 source audit.

## Baseline gate

The approved C0.1 archive is `D:\projek\sealed\Sealed_Heaven_C0_1_Source (1).zip`. Its SHA-256 is `9c3b7907845c8cf0ff2e757c9a5d4125a4264ab6977b7513b1300f16aaa51250`, matching the approved archive hash. Before any C0.2 edit, all nine archived files were byte-identical to the working tree.

The pinned C0.1 suite then passed on Rust 1.100.0: `cargo fmt --all --check`, `cargo check --locked`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`, and `cargo test --release --locked`. That run was 14 unit tests and 1 doctest in both debug and release.

## What landed

C0.2 adds the `SH:C02:STATE` revision 1 domain checkpoint, bounded restore, `drive`, fixture 2, and the mode, command, and work phases from `docs/C0_2_IMPLEMENTATION_SPEC.md` version 0.1-review-r1.1. `step` on fixture 1 still returns `Vec<Event>`. `step` on fixture 2 returns `UseDrive` and does not mutate. `checkpoint_bytes` remains the C0.1 observation and panics outside the states that observation can encode.

The package version stays `0.1.0`. The specification allows `0.2.0`, and leaving the version unchanged keeps `Cargo.lock` byte-identical to the approved archive. No dependency was added.

## Changed files

| Path | Change |
| --- | --- |
| `Cargo.toml` | Description now names the C0.2 fixture. Version and dependencies are unchanged |
| `src/lib.rs` | Domain types, `drive`, scheduler constructor, phased transition |
| `src/domain.rs` | New domain encoder, bounded decoder, and fixture validation |
| `src/encoding.rs` | C0.1 checkpoint match covers the new payloads by panicking; scheduler digest helper added |
| `src/tests/mod.rs` | Declares the C0.2 test modules |
| `src/tests/c02.rs` | C02-T01, C02-T02, and C02-T04 coverage |
| `src/tests/c02_golden.rs` | Section 11 hex transcribed from the acceptance document |
| `C0_2_IMPLEMENTATION_REPORT.md` | This report |
| `C0_2_VERIFICATION_LOG.txt` | Command log |

Unchanged from the approved archive: `Cargo.lock`, `rust-toolchain.toml`, and `src/tests/golden.rs`. The three normative C0.2 documents were not edited. No published golden literal was replaced.

## Acceptance results

| Group | Evidence | Result |
| --- | --- | --- |
| C01-T01 through C01-T06 | Existing 14 unit tests and the original doctest, debug and release | PASS |
| C02-T01a | Tick-2 domain blob, frozen C0.1 image, event 3, digest, and cursor | PASS |
| C02-T01b | Probe ordinal resume and both probe domain blobs | PASS |
| C02-T01c | Markers 10 and 11, IDs 1 through 9 retained, `InvalidDueTick` | PASS |
| C02-T01d | Truncation, format, revision, checksum, corrupt, fixture, and semantic failures | PASS |
| C02-T01e | Temp-file round trip, short read, missing path, failed write | PASS |
| C02-T01f | Fixture-specific illegal states and every published domain blob | PASS |
| C02-T01g | `u64::MAX` actor, pending, and cargo counts, plus an oversized cargo count | PASS |
| C02-T02a | Ticks 0 through 4, one transfer, resume through `drive`, `UseDrive` | PASS |
| C02-T02b | Same-tick mint and completions record scheduled mode | PASS |
| C02-T02c | Disabled mint rejects, cursor stays 10, no marker | PASS |
| C02-T02d | Grouped `drive` matches immediate `drive`; overdue and cursor overflow roll back | PASS |
| C02-T03 | Locked format, check path via the test build, Clippy, debug, release, dependency tree | PASS |
| C02-T04 | Doctest on `Simulation::drive` | PASS |

## Golden comparison

`src/tests/golden.rs` was compared byte for byte with the approved archive after implementation and still matches. Its C0.1 checkpoint, event, and RNG literals were not edited.

Every labeled blob in acceptance section 11 is compiled into `src/tests/c02_golden.rs` and compared with `domain_checkpoint_bytes`. The tests also check the approved event-3 digest `1a1cc22bf1041c19845bceb790e6761dad0edaf41cb3eca1f90b0ce06f1875b1`, the fixture 2 task and mint digests, and the witness tick-1 hash `35b5104ca23dc7723baffbe76948a044fdda583eb8374604149133986e2f4ac2`. The superseded witness hash `12d94a56` is not used.

## Build

| Item | Value |
| --- | --- |
| Toolchain | Rust 1.100.0, commit `0ef1bc386059596dada8fb65b50425aab2ea637a`, Cargo 1.100.0 |
| Host | `x86_64-pc-windows-msvc` |
| Direct dependencies | `rand_chacha` 0.10.0, default features off; `sha2` 0.11.1 |
| Cargo.lock SHA-256 | `176e4bb5d145739c101c10296ce9f97d4a24917381be057e74134e0ba1779d75` |
| Debug | 25 unit tests, 2 doctests, 0 failed, 0 ignored |
| Release | 25 unit tests, 2 doctests, 0 failed, 0 ignored |

`cargo tree -p sealed_sim --locked --offline -e normal,build,dev` shows only `rand_chacha`, `sha2`, and their existing crates. There is no Bevy, serde, or new direct crate.

## Limitations

- This host does not certify R2 or any other target.
- Domain writes use `std::fs::write`. A crash can leave a torn file. Restore rejects that file. The simulation is not changed by a failed write.
- There is no C0.1-to-C0.2 migrator and no `ticks_per_game_day` field.
- An already installed synthetic task still completes when its actor is disabled. Disabled mints, claims, and probes are rejected. Production disablement remains C0.3.
- `checkpoint_bytes` is not a save and panics on fixture 2 or on a scheduler payload.

## Verdict

**PASS** for C0.2 implementation verification on this host. Final C0.2 acceptance still requires the independent source and execution audit. C0.3 was not started.
