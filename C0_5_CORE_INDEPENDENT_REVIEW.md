# C0.5 independent core source review

Date: 18 November 2026. **Source-only verdict: FAIL — one medium-severity
restore/history-validation finding, with three reproduced variants.** This is not
final C0.5 acceptance. No kernel, application, dependency, or test source was edited
by this reviewer. The probe is a separate temporary crate using public APIs.

## Scope and contract interpretation

Read `C0_5_IMPLEMENTATION_SPEC.md`, `C0_5_ACCEPTANCE_TESTS.md`,
`src/player5.rs`, `src/player5/codec.rs`, `src/player5/tests.rs`, and the
`src/lib.rs` module export; inspected relevant frozen C03 checkpoint encoding.
Focus: restore semantics, checksum-valid mutations, registry/retry behavior,
finite resource ownership, and whole-aggregate failure atomicity.

The approved one-way occupancy rule is **not a finding**. NPCs may arrive over
the player; the player must still be prevented from *entering* an occupied cell.
The separate finite source/player/designated-shrine herb ledger is also within
the approved scope. Same-crate composition with the frozen NPC simulation is
acceptable. No 100/1000-player capability is assumed or required.

The finding is against the saved registry's claimed gameplay outcomes and the
advertised rejection of semantically inconsistent candidates (spec lines 16–20,
53–61; C05-T06), reinforced by `codec.rs:155`'s claim to validate the complete
finite C05 history. It is **not** a demand for cryptographic save authentication
or proof of every historical C03 dynamic field. If the intended restore contract
is only structural/current-state invariants, this finding needs an explicit
contract decision: that weaker contract does not establish outcome-history
validity, despite the existing validator/comment suggesting it does.

## C05-CORE-R1 — NPC-dependent registry outcomes are trusted on restore

**Severity: medium; checkpoint semantic-integrity gap.** No failure was found
when obtaining worlds only through ordinary admitted player commands. Exploiting
these reproductions requires editing a checkpoint and recomputing its unkeyed
hashes. SHA256 is not authentication, and accepting an alternative *valid* history
would not itself be a defect. These examples instead encode outcomes that the
rules reject, using unchanged authentic NPC checkpoints at ticks 1 or 4.

Relevant code in `src/player5/codec.rs`:

- Lines 208–214 replay successful movement using static `walkable` only; no live
  NPC occupancy is checked.
- Line 215 accepts every `(Talk(1), Requested)` without range validation.
- Lines 263–269 allow an `Occupied` rejection for any statically walkable
  destination and an `OutOfRange` rejection for every `Talk(1)`.
- The reconstructed event digest authenticates none of those predicates: the
  outcomes supplied by the file are inputs to reconstruction.

### Three independent public-API reproductions

All start with `World::settlement()`. Every action uses a correctly stamped
`Command` submitted through `World::drive`. No private field access is used.

| Variant | Actual public execution | Mutation, with hashes recomputed | Actual restore result |
| --- | --- | --- | --- |
| A: remote request | At tick 1, `Talk(1)` returns `Rejected(OutOfRange)`. Player `(0,4)`, NPC1 `(3,3)`, distance 4. | Change record outcome to `Requested` and `requested` to true. | `Ok(world)`, `requested() == true`. |
| B: invented occupancy | At tick 1, `Move(East)` returns `Moved`; `(1,4)` has no object or enabled NPC. | Change outcome to `Rejected(Occupied)` and player x back to 0. | `Ok(world)`. |
| C: player enters NPC cell | Submit East, East, North, East. At tick 4 the last action returns `Rejected(Occupied)` from `(2,3)` toward `(3,3)`. NPC occupancy is confirmed on the committed snapshot. | Change last outcome to `Moved` and player x to 3. | `Ok(world)` with player at `(3,3)`. |

Variant C is not an NPC arriving after the player: the genuine NPC-first drive
rejects this exact player entry. Variants A/B require only one known simulation
step; C requires four. None depends on an inaccessible or hypothetical long
history. Each forged checkpoint retains the exact original embedded C03 bytes,
valid command identity/high-water, quantities, and all unrelated fields.

Variant A restores quest authorization without the required nearby conversation.
Variant C restores a player movement that violates the approved entry rule.
Variant B demonstrates the broader false-outcome acceptance. These are grouped
as one root cause, not three independent severity counts.

### Reproduction artifact and command

Probe crate:

`C:\Users\PC_4\AppData\Local\Temp\opencode\c05-core-independent-probe`

Its `Cargo.toml` references this repository as a path dev-dependency and uses
`sha2 = "=0.11.1"`. `src/lib.rs` contains eight executable public-API tests.
Run from that directory in PowerShell:

```powershell
& 'C:\Users\PC_4\.cargo\bin\cargo.exe' test --offline --release -- --nocapture
```

Observed in **both debug and release**: 5 passed, 3 failed, exit 1. The three
failing tests assert that the above semantically false checkpoints must be
rejected. Their diagnostics are:

```text
forged remote request accepted=true requested=Ok(true)
forged false occupied accepted=true
forged entry into occupied cell accepted=true
```

For reproduction without the temporary crate, the exact mutation recipe is:

1. Save the event digest immediately before the last action; generate the
   genuine checkpoint after the action.
2. Rev1 offsets: player x at `[58..62]`, requested byte 105, digest
   `[123..155]`, registry starts at 159, each record 31 bytes. Record key is
   24 bytes, intent 5 bytes, outcome 2 bytes. Change the final outcome to
   `[2,0]` (Requested), `[5,3]` (Rejected Occupied), or `[1,0]` (Moved), as above.
3. Recompute the last event digest as SHA256 of the concatenation:
   `b"SH:C05:EVENT:v1"`, preceding digest[32], event ID:u64LE,
   event tick:u64LE, record key[24], outcome[2]. Event ID/tick are 1/1 in A/B
   and 4/4 in C. Write it at `[123..155]`.
4. Recompute the trailing SHA256 over every preceding byte.
5. `World::restore` accepts the resulting bytes.

### Remediation/exit requirements (not implemented by reviewer)

- Resolve the advertised history-validation boundary explicitly. To preserve the
  current strong claim, validate NPC-dependent predicates against the canonical
  NPC state at the relevant action boundaries, not supplied outcome tags.
- Preserve bounded restore. An unbounded replay loop over a hostile u64 tick is
  not an acceptable fix; do not silently add a restore limit that makes ordinary
  generated checkpoints unloadable.
- Add the three checksum-valid regression cases, plus valid nearby requests,
  legal occupancy rejection, and approved NPC-arrival overlap roundtrips.
- Rerun debug/release tests and have this independent reviewer rerun the probe.
  Any deliberately weaker state-only contract needs explicit approval and honest
  documentation; do not relabel it as complete history validation.

## Positive coverage and observations

Independently executed:

| Coverage | Result |
| --- | --- |
| Root `cargo test --offline` | 93 unit tests + 2 doctests passed. |
| Root `cargo test --offline --release` | 93 unit tests + 2 doctests passed. |
| Focused `cargo test --offline player5:: -- --nocapture` | All 11 new tests passed. |
| External probe, debug and release | 5 positive controls passed; 3 restore regressions failed as above. |

External positive controls tested:

- Exact retries after restore, multiple identical copies, conflicting historical
  identity mixed with a fresh command, and differently ordered equivalent batches.
  Conflict preserves the entire previous checkpoint; valid suffixes match.
- Public sequence `u64::MAX` rejection, accepting `MAX-1` then restoring the
  exhausted high-water, retry safety at exhaustion, and `MAX_BATCH+1` rollback.
- A 150-action deterministic stream, checkpoint/restore and event equivalence at
  every boundary, unchanged autonomous NPC checkpoints, and herb sum 3 throughout.
- An independently written public-API full gameplay loop, completed at tick 24;
  save/load and duplicate attempts at every action, final ledger `(0,2,1)`, and
  repeated restored delivery identity with no extra transfer or event.
- A restored C03 event cursor set to `u64::MAX` with valid inner/outer checksums.
  The next NPC step fails at boundary 1 and preserves **every aggregate checkpoint
  byte**, including the pending fresh player's action. Cursor exhaustion is not
  itself classified as an invalid-history defect.

Source inspection found clear debit/credit staging with checked arithmetic,
typed/fixed herb owners, exact registry persistence, and clone-before-NPC-step
publication. No additional resource duplication, retry side effect, sequence
reuse, or partial candidate publication was reproduced. Full registry capacity
and player event/tick overflow are covered by existing internal tests; this
review did not independently generate 4096 real admitted commands.

`git diff -- src/lib.rs` showed only `pub mod player5;` in the old root source.
No separate historical reachability claim for all restored NPC dynamic fields is
made or treated as another finding here.

## Limits and acceptance status

No application, native UI, locale, file-write atomicity, frame partitioning,
rendering, scaling, CPU/memory, or delivery audit was performed by this reviewer.
Root suite success does not cover all C0.4 app tests. Formatting/Clippy/packers
were not rerun here. No claim is made for C05-T09/T10 or final C0.5 PASS.
No remediation was authored, so reviewer independence is retained.

Reviewed file SHA256 values (before remediation):

```text
src/player5.rs
64BDA4367A415C38DBD97353719F51F93CD2ADB47A384B4AA8F8C62A1B4046EA
src/player5/codec.rs
D2F03472D81B23D1ACD3DC2F34B6D0B290172F127BEE136C951C8AE6F9FB2EAB
src/player5/tests.rs
E1BA68E8ADA5CA5EB8E4011A11ADA97CD5B99DA0DE03471322E3B8E7A6513DEA
```
