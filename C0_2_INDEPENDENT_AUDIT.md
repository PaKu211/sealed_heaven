# Sealed Heaven C0.2 Independent Audit (G2)

17 November 2026

**Verdict: PASS**

The submitted archive matches the declared hash. On this Windows host, the locked format, compile, lint, and test commands passed, and the published checkpoint, digest, and RNG oracles matched the R1.1 acceptance document rather than values derived from the encoder. Auditor-authored checks for corrupt input, illegal state, overflow, rollback, replay, disabled synthetic tasks, and `step` misuse also passed. No blocking correctness defect was found.

This is not cross-platform certification. Final program acceptance can still record that limitation separately. C0.3 was not started, and the submitted archive was not modified.

## Evidence boundary

The audit executed `Sealed_Heaven_C0_2_Source.zip` after extracting it to a temporary directory. The archive itself was left unchanged. Additional adversarial tests were compiled only in that temporary copy.

A file named as a C0.1 independent audit report was not in the archive, the C0.2 workspace, or `D:\projek\sealed`. The approved C0.1 source archive was present and was used as the baseline. `G0_REVIEW_REPORT.md` is a G0 documentation review, not a C0.1 code audit. `C0_1_IMPLEMENTATION_REPORT.md` states that independent C0.1 sign-off was still pending and records a Linux run. This audit re-executed the preserved C0.1 tests on Windows.

## Artifact integrity

| Check | Result |
| --- | --- |
| Archive SHA-256 | `2804d33a785acb7331a0d82fd1c080bec82d0add348578abb4b1a6807db60a40` |
| Submitted hash | Same value |
| Members | 29 files under `sealed-heaven-c0.2/` |
| Toolchain | Rust `1.100.0`, `rustfmt`, `clippy` |
| Direct dependencies | `rand_chacha` `=0.10.0` with default features disabled; `sha2` `=0.11.1` |
| Package version | `0.1.0` |
| Normative docs in the archive | `C0_2_IMPLEMENTATION_SPEC.md`, `C0_2_ACCEPTANCE_TESTS.md`, and `C0_2_ARCHITECTURE_REVIEW.md`, each version `0.1-review-r1.1` |

Implementation files: `src/lib.rs`, `src/encoding.rs`, `src/domain.rs`. Test files: `src/tests/mod.rs`, `src/tests/golden.rs`, `src/tests/c02.rs`, `src/tests/c02_golden.rs`.

Against `Sealed_Heaven_C0_1_Source (1).zip` (`9c3b7907845c8cf0ff2e757c9a5d4125a4264ab6977b7513b1300f16aaa51250`):

| File | Comparison |
| --- | --- |
| `Cargo.lock` | Byte match |
| `rust-toolchain.toml` | Byte match |
| `src/tests/golden.rs` | Byte match |
| `C0_1_IMPLEMENTATION_REPORT.md` | Byte match |
| `C0_1_VERIFICATION_LOG.txt` | Byte match |
| `Cargo.toml` | Description text changed from C0.1 to C0.2. Version and dependency lines are unchanged |
| `src/encoding.rs` | C0.1 event and checkpoint encoding retained. New payload arms panic, which matches the C0.1 observation rule. A scheduler digest helper was added |
| `src/lib.rs`, `src/tests/mod.rs` | C0.2 API, phases, and test-module declarations |
| `src/domain.rs`, `src/tests/c02.rs`, `src/tests/c02_golden.rs` | New |

No unexpected difference was found in the preserved C0.1 golden literals, lockfile, or toolchain pin.

## Source audit

`domain.rs` writes `SH:C02:STATE` revision 1, little-endian records, and `body || SHA-256(body)`. Restore applies truncation, tag, revision, checksum, bounded parse, fixture identity, content hash, and then the numbered semantic rules. List counts use checked multiplication against the unread body before `Vec::with_capacity`. `u64::MAX` therefore returns `Corrupt` without an input-sized allocation.

`step` on fixture 2 returns `UseDrive` before the boundary check and before mutation. `drive` stages a clone and publishes it only after admission, the tick increment, the mode phase, the command phase, and the work phase succeed. Mint identifiers advance one monotonic cursor. Task completion moves an existing cargo owner and releases the linked reservation; it does not allocate a cargo or reservation record. Disabled actors reject mints and still complete an already installed synthetic task.

The C0.1 `Event` accessors and `encoding::event_bytes` path are unchanged for `Claimed`, `Rejected`, and `Probed`. Fixture 1 probe continuation still uses `SH:C01:RNG:v1`.

## Acceptance execution

Commands were run in the extracted archive with the pinned toolchain:

```text
cargo fmt --all --check
cargo check --locked --offline
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --locked --offline
cargo test --release --locked --offline
```

`--offline` was required because the lockfile forbids a network update. Debug and release each ran 25 unit tests and 2 doctests. Zero failed. Zero ignored.

The 15 labeled section 11 blobs were hashed with Python `hashlib` from the acceptance document. All 15 digests matched the published SHA-256 labels. The hex embedded in `src/tests/c02_golden.rs` matched those document blocks exactly. The approved event-3 digest was recomputed from H2 and the published event bytes as `1a1cc22bf1041c19845bceb790e6761dad0edaf41cb3eca1f90b0ce06f1875b1`. Fixture 2 H0 and content identity also matched the specification. The C0.1 suite, using the unchanged `golden.rs`, covered the four RNG vectors.

## Adversarial checks

Six additional tests were executed only in the temporary copy. All six passed:

- An unknown payload tag returns `Corrupt`. A 14-byte revision-2 header returns `UnsupportedRevision { found: 2 }`.
- `next_actor_id = 4` returns `InvalidState`. A `u64::MAX` reservation count returns `Corrupt`.
- A conflicting command pair leaves the domain bytes and tick unchanged, and a later empty `step` advances.
- Reversed fixture 2 mode batches produce equal domain bytes.
- A mint at `next_record_id = u64::MAX` returns `CapacityExceeded` without mutation, and a following empty `drive` advances. Five restore cycles reproduce the same bytes.
- With actor 1 disabled, both installed tasks still complete once, cargo moves to locations 2 and 3, and a later drive does not move them again.
- `step(9, &[ClaimSlot])` on fixture 2 returns `UseDrive`, leaves tick 0 and the domain bytes unchanged, and a later `drive` advances.

## Findings

### G2-I01 — INFORMATIONAL

The required C0.1 independent audit report is not in the submission. The C0.1 implementation report says that sign-off was pending and cites Linux. This audit compared the preserved C0.1 files with the approved source archive and re-ran the C0.1 tests on Windows. No C0.1 golden drift was found. No remediation is required for the C0.2 code gate.

### G2-I02 — INFORMATIONAL

Both the submitted log and this audit ran on `x86_64-pc-windows-msvc`. No Linux execution was available, so the Windows result was not compared with a Linux result. R2 remains outside C0.2. No code change is indicated.

### G2-I03 — INFORMATIONAL

`Cargo.toml` still says version `0.1.0` while its description says C0.2. The specification allows the version to become `0.2.0` and also requires the lockfile to stay authoritative. Keeping `0.1.0` preserves the approved lockfile hash `176e4bb5d145739c101c10296ce9f97d4a24917381be057e74134e0ba1779d75`. This is a naming choice, not a behavior defect.

## Verdict

**PASS.** The mandatory C0.1 and C0.2 acceptance criteria that could be executed on this host were independently verified. No blocking correctness defect remains in the submitted implementation.
