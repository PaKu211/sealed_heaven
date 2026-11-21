# C2.2 Test and Audit Report

## Environment

- Implementation commit: `7d92ad3f2173875ed630ca13e967d63db2da9e5f`
- Host: Linux cloud agent
- rustc: 1.100.0
- Baseline: Git commit `7694ce2` (`feat/c21-institutional-accountability`)
- Note: the Windows-tested C2.1 ZIP was not supplied in-tree; the matching Git tip was used

## Final Linux gates

```bash
cargo fmt --all -- --check
cargo test --lib living::
cargo test --manifest-path apps/sealed_playable/Cargo.toml --no-default-features
cargo clippy --lib -- -D warnings
cargo clippy --manifest-path apps/sealed_playable/Cargo.toml --no-default-features -- -D warnings
```

| Command | Result |
| --- | --- |
| `cargo test --lib living::` | **130 passed** (C2.2.1) |
| playable `--no-default-features` `--lib` | **48 passed** |
| playable adapters | **6 passed** |
| playable agency (`tests` c05) | **9 passed** |
| playable clock (c06) | **3 passed** |
| playable supply (c07) | **9 passed** |
| playable conflict (c08) | **2 passed** |
| playable memory (c09) | **2 passed** |
| fmt / clippy `-D warnings` | clean |

Total playable `--no-default-features` automated tests counted above: **48 + 6 + 9 + 3 + 9 + 2 + 2 = 79**.

## Critical audit (finalization)

| Area | Finding | Action |
| --- | --- | --- |
| Propagation | Face-to-face + delay; no global omniscience | Covered by tests |
| Provenance | Firsthand / Reported{origin,hops} preserved | Covered by tests |
| Duplicate reports | Single knowledge + single dispute per event/institution | Covered by tests |
| Hop-2 allegations | Previously opened Open disputes | **Fixed:** hop-2 remembered only; no dispute/bid effect |
| EVENT_CAP | Dead prune branch never removed entries | **Fixed:** refuse new events when full |
| Suspension / bids | Suspended providers filtered; deltas applied | Covered by tests |
| Checkpoint rev 13 | Fingerprints v13; rev 12/11 isolated | Covered by tests |
| `--c21` | Rev 12, no `Information`, no suspension path | Living + ASCII isolation tests |
| ASCII privacy | `infolog` / frame filter `witnessed` only; absent player history unmarked | Covered by tests |
| C0.1 kernel | No diff under `src/lib.rs`, `domain`, `encoding`, `fixture3`, `player5`, `tests/` | Verified vs `7694ce2` |

## Still not claimed

- Windows host re-run
- Human playtest
