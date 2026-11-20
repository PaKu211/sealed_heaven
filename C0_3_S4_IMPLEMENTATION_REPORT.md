# C0.3 S4 implementation and acceptance evidence

2026-11-18. Branch `feat/c03-s4-integration`, based on verified/pushed S3
`ca85bfdadf6b451bd4d41d2347cc7e450ceab495`. S2 was verified/pushed at
`509d9c014b6d2aa294e79310dfcee1f840d070a2`.

## Outcome

The existing generic fixture-3 kernel now runs the approved 20-actor integration.
S4 adds the exact contention fixture and mixed-goal/lifecycle regression coverage;
it does not introduce a second scheduler or a separate production implementation.
80 unit tests and 2 doctests pass in debug and release. Formatting, locked/offline
check, strict Clippy, and the standalone contract-reference check all pass.

This is implementation verification, **not independent final acceptance**.
No merge to main, new milestone, Bevy integration, or dependency change is claimed.

## Acceptance coverage

| Contract | Executable evidence | Result |
| --- | --- | --- |
| C03-T01a, separate one-unit contention | `c03_s1::c03_t01a_two_claimants_match_independent_blob` | Verified; published blob unchanged |
| C03-T01b/c/d invalidation | S1 expiry/cancel/disable tests; remediation H06 | Verified |
| C03-T01e identity/replay | S1 completion test; H03/H05/H09/H10; S4 lifecycle | Verified |
| C03-T01f transactions/rollback | S1 later-leg/credit/reservation tests; S3 M0-T07 | Verified |
| C03-T01g saved completion provenance | H10 reload tests; S3 transfer reloads; S4 every split | Verified |
| C03-T01h hostile saves/overflow | S1 and remediation codec matrix; navigation overflow | Verified |
| C03-T01i grouped/reordered boundaries | Remediation reordered test; S4 lifecycle grouping | Verified |
| M0-T06 twenty actors, supply 5 | S4 exact event contract, four input permutations, scheduled mode | Verified; published blob `528bc334...` unchanged |
| M0-T06 atomic failure | Actor-20 over-reserve after earlier staged allocations | Verified; all bytes restored, next valid call matches pristine run |
| M0-T07 four locations and recipe | S3 ten-tick script, initial oracle, event stream oracle | Verified; tick 10 expected `InvalidLeg`, tick remains 9 |
| M0-T08 cause priority/exactly once | H06; S3 unreachable; S4 empty Rest/Return and simultaneous causes | Verified; reason 1 wins, no completion legs |
| Five goals together | S4 twenty living actors, four of each goal | Verified; active/scheduled event equivalence, all complete |
| Future command and in-flight holds | S4 lifecycle, save at each boundary | Verified; exact suffix outputs and checkpoint bytes |
| Route/grid/residual travel | S3 movement tests | Concrete acceptance verified; timing interpretation below |
| Frozen C0.1/C0.2 compatibility | Entire original suites, goldens, file comparison to `bf91789` | Verified unchanged |

## Oracle provenance

`docs/tools/c03_vectors.py` is a standalone Python struct/hashlib packer using
fixed acceptance-table inputs, not simulator output. It reproduces the published
M0-T06 body/blob and M0-T07 initial body/blob hashes. Its checked-in reference is
`docs/C0_3_VECTOR_REFERENCE.txt`. It also derives Eat and M0-T07 tick-9 streams.
Rust tests compare those independently packed streams/hashes to runtime events.
This separation is oracle evidence, not a claim of independent reviewer sign-off.

## Change review

- Fixtures 1/2 source, frozen golden modules, Cargo pins/lock and toolchain file
  are byte-identical to inherited S1 `bf91789`.
- Fixture-3 checkpoint/event formats and scenario block encoding are unchanged.
- Approved graph remains sites 1/2/9, edges 1/2; OC-EDGE is not introduced.
- Creation-tick skip and one transition per tick remain; failed boundaries publish
  nothing. New unreachable handling shares the existing release/provenance path.
- Only intended source/tests/reports/reference files are committed. `target/`
  remains ignored and preserved. No credentials, build artifacts, or archive added.

## Remaining gate

Independent implementation review and owner acceptance are still required.
Spec §8's broad residual-work sentence and acceptance §14's concrete post-cancel
save have different possible same-boundary timing readings. Implementation follows
the concrete test: cancellation preserves edge cost 0, then the next drive spends
it, including for a disabled actor. See the S3 report. Request explicit reviewer/
owner confirmation before declaring the C0.3 milestone accepted; no normative
document was silently amended to resolve this ambiguity.
