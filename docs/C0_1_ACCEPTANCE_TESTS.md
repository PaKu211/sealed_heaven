# C0.1 Acceptance Tests

Version 0.2-draft · 17 November 2026

Run under the [C0.1 specification](C0_1_IMPLEMENTATION_SPEC.md). Every row below is a required executable test group or reproducible build check at C0.1. The documentation supplies independent expected values. Execution and project approval are pending.

## Acceptance matrix

| ID | Inputs and procedure | Required assertions |
| --- | --- | --- |
| C01-T01 | Initialize seed 100 and enabled actors 1/2. Read the checkpoint repeatedly without stepping. At boundary 0 admit the two tick-2 claims specified below. Pause by invoking no steps. Then step at boundary 1 with an empty batch. In a separate fresh run invoke two empty steps. | Initial read is unchanged. Tick 1 holds both future commands, vacant slot, cursor 1, and H0. Only tick 2 drains the claims. Two empty steps advance through ticks 1 and 2 with no events or digest changes. Fixture IDs, seed, initial state and both flags are observable. |
| C01-T02 | Run the two-claim batch in each input order. Run again with actor 1 disabled and actor 2 enabled. Submit a later claim from the current owner. Run two same-actor contending claims in separate cases to distinguish producer and sequence ordering: producer 1 / sequence 9 vs producer 2 / sequence 0, then producer 1 / sequences 0 and 1. Repeat a producer case with IDs 1 and 256 to distinguish numeric order from little-endian byte order. Also place a probe from a disabled actor in a fresh run. | The enabled base case emits event 1 `Claimed` for actor 1 and event 2 `Rejected(OccupiedSlot)` for actor 2, both at tick 2. With actor 1 disabled the event order is `Rejected(DisabledActor)`, then `Claimed` for actor 2. The current owner's repeat claim is `Rejected(OccupiedSlot)`. Lower producer wins the first same-actor case and lower sequence wins the second. A disabled probe emits only `Rejected(DisabledActor)` and retains its ordinal. |
| C01-T03 | Submit exact duplicates in one batch and resubmit identical pending work before its target. Submit the same key with a claim and a probe in one batch, then conflict with a previously admitted pending command. Include valid same-batch work alongside each failure. Cover wrong boundary, target at or before boundary, unknown actor, producer zero, invalid stream/count where representable, multiple invalid keys, and replay of an already consumed command. Use a test-only state at numeric limits to hit tick, cursor and enabled-probe ordinal overflow. | Exact duplicates yield one due event. Every failed batch returns the specified diagnostic and lowest relevant key, when applicable. Check full checkpoint bytes before/after, no emitted events and no advance. Permuting the batch leaves the diagnostic unchanged. A retry at the same boundary succeeds with corrected input. Overflow returns `CapacityExceeded` with full rollback even if the candidate step had earlier valid work. |
| C01-T04 | Encode all checkpoint and event vectors below. Reconstruct the four RNG vectors. Run probe sequence A with one draw then B, and repeat with two draws from A before B. Also run two successive A probes. Permute actor/queue insertion where a test fixture builder permits it, and vary one mutable checkpoint group at a time using valid states or an internal encoding fixture. | Match exact bytes and hashes. A and B stand for streams 1 and 2 for actor 1. A's first word is identical in either run, B's words and next ordinal match across runs, and A's ordinal advances once per decision. The second A probe uses the ordinal-1 vector. Numeric key order is used even if byte order sorts differently. Check that changes to tick, seed, an actor flag, owner, pending record, ordinal, cursor and prefix digest affect the expected bytes and hash. Fixed tags, widths, counts, discriminants, revision and content identity match the literal vectors. |
| C01-T05 | Repeat the contention replay three times on the same locked build/platform. Include the tick-1 prefix, compare individual calls and driver groupings over the same explicit boundaries, and retry one boundary after a failed admission. In a separate run take one probe from seed 101. | Compare byte-for-byte at each checkpoint and compare the concatenated event stream. Match the independent tick/slot and event oracle, including the unchanged prefix after the rejected attempt. Seed 101 produces the published distinct word with the same tick/cursor rules. Driver batching never changes the submission boundary. |
| C01-T06 | Review package targets and the locked normal/build/dev dependency graph. Run formatting and headless tests with dependencies cached, Cargo locked and offline, in an environment with no window/GPU. Inspect source/dependencies for the milestone exclusions. | Exactly one production library package named `sealed_sim`, with tests. No Bevy or other forbidden dependencies or systems, and no system entropy for simulation draws. Record selected toolchain, target, platform, fixture revision, dependency-lock digest, exact commands, test totals and expected/observed hash and event excerpts. Report results and authorized skips. Skips of required assertions block exit. |

Candidate commands for C01-T06 are `cargo fmt --all --check`, `cargo test -p sealed_sim --locked --offline`, and `cargo tree -p sealed_sim --locked -e normal,build,dev`. C0.1 exit requires review of the executed evidence for all six groups.

## Golden fixture

The vectors below were computed from the byte specification before production code existed. Fields were packed with Python standard tooling and SHA-256 was independently checked with OpenSSL. Separate Python and JavaScript ChaCha block calculations from the published algorithm agreed on all four complete blocks. The production crate must compare its outputs to these literal values. A newly generated implementation snapshot is insufficient evidence. Hex blocks may wrap. Concatenate their characters without whitespace.

### Slot contention and event chain

Use fixture 1, seed 100, both actors enabled. At boundary 0 submit `ClaimSlot(target=2, actor=2, producer=1, sequence=0)` and `ClaimSlot(target=2, actor=1, producer=1, sequence=0)`. At boundary 1 submit an empty batch. Expect tick 1 vacant and both commands pending. At tick 2 actor 1 owns the slot, pending is empty, and the next event ID is 3. All RNG ordinals stay zero. Event 1 claims for actor 1 and event 2 rejects actor 2 as occupied.

| Value | Hex |
| --- | --- |
| Fixture/content bytes | `53483a4330313a464958545552453a76310a` |
| Content SHA-256 | `042797b9b5032aaf66af7473dd8ff50a349c865a5b4f147405446f78dad96c59` |
| H0 | `87cb000d61574a1611078311af86095b5b9c51b780f01d65447da0f549233874` |
| Event 1 bytes | `010000000000000002000000000000000100000001000000000000000000000001` |
| H1 | `00b7b41aa859829eae7227842d324449527d90367569b7e0d045586364b1d0b4` |
| Event 2 bytes | `02000000000000000200000000000000020000000100000000000000000000000202` |
| H2 | `54fabd2a30f91a021e8593391702c18e09e7ff407f0158ea5b532f86ac39acba` |

### Canonical checkpoint encodings

**Tick 0, before input** · 193 bytes · SHA-256 `1d59cf7c1c215a314981cbbf210f4feada221adc4cbca620ac5d89dc9920224b`

```text
53483a4330313a5354415445010001000000042797b9b5032aaf66af7473dd8f
f50a349c865a5b4f147405446f78dad96c590000000000000000640000000000
0000020000000000000001000000010200000001000000000000000000040000
0000000000010100000000000000000000000102000000000000000000000002
0100000000000000000000000202000000000000000000000001000000000000
0087cb000d61574a1611078311af86095b5b9c51b780f01d65447da0f5492338
74
```

**Tick 1, both commands pending** · 243 bytes · SHA-256 `80f5c7a13e262cf5991a10d5298d47f1c186e3b3e564fd0ecc899ca34f29884e`

```text
53483a4330313a5354415445010001000000042797b9b5032aaf66af7473dd8f
f50a349c865a5b4f147405446f78dad96c590100000000000000640000000000
0000020000000000000001000000010200000001000200000000000000020000
0000000000010000000100000000000000000000000102000000000000000200
0000010000000000000000000000010400000000000000010100000000000000
0000000001020000000000000000000000020100000000000000000000000202
0000000000000000000000010000000000000087cb000d61574a1611078311af
86095b5b9c51b780f01d65447da0f549233874
```

**Tick 2, actor 1 holds the slot** · 197 bytes · SHA-256 `e02c6e47e0ebfc387c32d679edb13cf988285ccc51f603ace8e698556814b16c`

```text
53483a4330313a5354415445010001000000042797b9b5032aaf66af7473dd8f
f50a349c865a5b4f147405446f78dad96c590200000000000000640000000000
0000020000000000000001000000010200000001010100000000000000000000
0004000000000000000101000000000000000000000001020000000000000000
0000000201000000000000000000000002020000000000000000000000030000
000000000054fabd2a30f91a021e8593391702c18e09e7ff407f0158ea5b532f
86ac39acba
```

**Tick 1, probe-only replay below** · 193 bytes · SHA-256 `83a113fff7d020df2714e0f4ca48bd892e19b1ac92c2fad5ab6b361d4d13daf8`

```text
53483a4330313a5354415445010001000000042797b9b5032aaf66af7473dd8f
f50a349c865a5b4f147405446f78dad96c590100000000000000640000000000
0000020000000000000001000000010200000001000000000000000000040000
0000000000010100000001000000000000000102000000000000000000000002
0100000000000000000000000202000000000000000000000002000000000000
003b0ee130dab2cc1d830bda3c2e544bf63b6beb02a9ab4b3b09a73650f32f62
a9
```

### Seed derivation and ChaCha12 words

All rows use actor 1. Words are displayed as hexadecimal integer values. Their event encoding uses little-endian bytes.

| Seed / stream / ordinal | Seed derivation input bytes | Derived 32-byte key | First two words |
| --- | --- | --- | --- |
| 100 / 1 / 0 | `53483a4330313a524e473a7631640000000000000001010000000000000000000000` | `2699fe5fb9f79d3aefd18d4a75249047d7187fe20d15f083f3806d73e9d67a9b` | `0x11915b32`, `0x9dcfed71` |
| 100 / 2 / 0 | `53483a4330313a524e473a7631640000000000000002010000000000000000000000` | `a9a8185f80e1ff1e04df6c1f5ace0d98f5f070b9c13fa65d0d97e88ab5ea0217` | `0xa498660b`, `0xddb1465b` |
| 100 / 1 / 1 | `53483a4330313a524e473a7631640000000000000001010000000100000000000000` | `5c38ce9bda76cade6f76677921dec92c94c92a75d34b8fed61873d39999fdc7e` | `0xdf3be1d5`, `0xb7742291` |
| 101 / 1 / 0 | `53483a4330313a524e473a7631650000000000000001010000000000000000000000` | `22f6e8a3164d12836a1882410932ffb200d52ada31a89fb106806d897a269e82` | `0x71ac287c`, `0xca155226` |

### Probe-only checkpoint and event

With seed 100 and both actors enabled, at boundary 0 submit only a tick-1 probe by actor 1, producer 1, sequence 0, stream 1, count 2. Expect tick 1, a vacant slot, no pending commands, ordinal 1 for stream 1/actor 1 only, and next event ID 2. The probe-only checkpoint appears above.

- Event bytes `0100000000000000010000000000000001000000010000000000000000000000030102325b911171edcf9d`.
- Digest after the probe `3b0ee130dab2cc1d830bda3c2e544bf63b6beb02a9ab4b3b09a73650f32f62a9`.
