# Sealed Heaven C0.1 Implementation Report

17 November 2026

**Verification result: C01-T01 through C01-T06 passed on the pinned Linux target.** Both debug and release runs completed 14 unit tests and one public-API doctest with zero failures or skips. This report records a separate verification pass by the implementation agent. Independent C0.1 gate sign-off is pending.

## Scope and build

`sealed_sim` is one Rust library package. Its public entry point takes a fixture configuration, then accepts immutable command batches at explicit logical boundaries. Each successful step returns ordered consequential events. The library exposes the logical state, canonical event and checkpoint bytes, and SHA-256 digests for replay comparisons.

A step validates the entire batch, stages admission and state changes, revalidates claims as they commit, then publishes the step only on success. The two fixed actors contend for one synthetic slot. RNG probes use four isolated actor/stream ordinals and ChaCha12 key derivation. Positive producer IDs through `u32::MAX` pass fixture producer preflight. Zero yields `InvalidProducer` in the defined diagnostic order.

The authorized G0 archive was used as the implementation baseline. Archive SHA-256 is `54398621beb611f4eccee3ba26df9ebf61e0b0ff33dd66b12e99460b984e9587`. The acceptance document SHA-256 is `7f178b174c0004dd24de4652843e824eb6c3a969ddd3adf3ea7834994b058c48`. The golden literals were transcribed from that document and compared back to it independently of the implementation output. No architectural contradiction or required ADR change was found. [Approved G0 package](https://chatgpt.com/library?file_id=libfile_99210eab78588191abaa08af403fa9d3&account_id=personal)

| Item | Verified value |
| --- | --- |
| Fixture | ID 1, canonical encoding revision 1, seeds 100 and 101 |
| Fixture content SHA-256 | `042797b9b5032aaf66af7473dd8ff50a349c865a5b4f147405446f78dad96c59` |
| Toolchain | Rust 1.100.0, rustc commit `0ef1bc386059596dada8fb65b50425aab2ea637a`, Cargo 1.100.0 |
| Target | `x86_64-unknown-linux-gnu`, Linux 6.18.57. The test process had no display configuration or GPU device nodes |
| Direct dependencies | `rand_chacha = "=0.10.0"` with default features disabled, and `sha2 = "=0.11.1"`. The resolved versions compiled and passed the golden vectors |
| Cargo.lock SHA-256 | `176e4bb5d145739c101c10296ce9f97d4a24917381be057e74134e0ba1779d75` |

## Acceptance results

| Group | Evidence | Result |
| --- | --- | --- |
| C01-T01 | Initial identity/flags/digest, empty steps, paused observations, future work and due tick | PASS |
| C01-T02 | Both input permutations, disabled actor and probe, repeat claim, numeric producer IDs including 256 and `u32::MAX`, then sequence precedence | PASS |
| C01-T03 | Duplicate collapse, conflict precedence, future and late inputs, invalid fields, batch rollback, corrected retry, tick/cursor/ordinal overflow | PASS |
| C01-T04 | Four complete checkpoint vectors, ordered event bytes and chained digests, four RNG keys/word pairs, stream isolation, numeric queue encoding and mutable field coverage | PASS |
| C01-T05 | Three runs using equivalent submissions and different grouping, including reversed input and one rejected attempt. All checkpoints and concatenated events match. Seed 101 produces the published word | PASS |
| C01-T06 | Formatting, locked offline fetch and check, Clippy with warnings denied, headless debug/release runs, public-API doctest, current/all-target dependency trees, and Cargo metadata audit | PASS |

The host dependency graph contains only the fixture's RNG and SHA-256 support and their dependencies. The all-target inspection also accounts for platform CPU feature detection and derive tooling. Cargo metadata identified one production library target and 19 locked dependency packages across all targets. Source and graph review found no Bevy, rendering, windowing, networking, runtime model, save/load, or system entropy dependency used by gameplay.

## Expected and observed golden values

| Output | Expected = observed SHA-256 |
| --- | --- |
| Initial checkpoint | `1d59cf7c1c215a314981cbbf210f4feada221adc4cbca620ac5d89dc9920224b` |
| Tick 1, both claims pending | `80f5c7a13e262cf5991a10d5298d47f1c186e3b3e564fd0ecc899ca34f29884e` |
| Tick 2, actor 1 owns the slot | `e02c6e47e0ebfc387c32d679edb13cf988285ccc51f603ace8e698556814b16c` |
| Probe-only checkpoint | `83a113fff7d020df2714e0f4ca48bd892e19b1ac92c2fad5ab6b361d4d13daf8` |
| Final contention event digest | `54fabd2a30f91a021e8593391702c18e09e7ff407f0158ea5b532f86ac39acba` |

At tick 2, event 1 claims the slot for actor 1 and event 2 rejects actor 2 as occupied. The seed-101 probe's first word is `0x71ac287c`. The exact observed event bytes, all RNG words, and command output are in [C0_1_VERIFICATION_LOG.txt](C0_1_VERIFICATION_LOG.txt).

## Source and reproduction

- `src/lib.rs` implements command admission, commit, fixture state, and the probe path. Its crate-level doctest shows the public call sequence.
- `src/encoding.rs` implements the revision-1 byte and digest contracts.
- `src/tests/mod.rs` and `src/tests/golden.rs` contain the executable test groups and frozen reference vectors.
- `Cargo.toml`, `Cargo.lock`, and `rust-toolchain.toml` pin the package build. The source package contains the lockfile as there was no Git checkout in the workspace.

From the extracted package root, Cargo uses the pinned toolchain. Run `cargo fetch --locked` once if dependencies are absent, then run:

```sh
cargo fmt --all --check
cargo check -p sealed_sim --locked --offline
cargo clippy -p sealed_sim --all-targets --locked --offline -- -D warnings
cargo test -p sealed_sim --locked --offline -- --test-threads=1 --nocapture
cargo test -p sealed_sim --release --locked --offline -- --test-threads=1
cargo tree -p sealed_sim --locked --offline -e normal,build,dev
cargo tree -p sealed_sim --locked --offline -e normal,build,dev --target all
```

## Limitations and open findings

No unresolved C0.1 acceptance failures or skipped cases. Runtime evidence covers one host architecture and toolchain. The atomic step stages a copy of the fixture state and its pending queue. Large queue performance was outside the acceptance workload and was not measured. The replay driver is in-package test code. The canonical checkpoint is an in-memory observation. C0.2 persistence and LOD are outside this delivery and require separate authorization.
