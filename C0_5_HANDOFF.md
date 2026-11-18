# C0.5 continuation

Stage: core/client implementation present; verification/audits in progress. Not accepted.
Base C0.4 `f35bfd3664d4baf2feaf78755b1a18d29cccf795` verified HEAD and remote;
initial tree clean. Working branch `feat/c05-player-agency` created from base.

Plan/oracles: `C0_5_IMPLEMENTATION_SPEC.md`, `C0_5_ACCEPTANCE_TESTS.md`.
Existing public command API has NPC task/stock commands but no manual movement or
direct gathering. Add explicit `sealed_sim::player5` aggregate ownership in same
core; leave accepted NPC semantics and checkpoints unchanged. Separate C05 native
binary/adapter; do not replace C04. No dependency/toolchain/root-lock change.

Domain author completed `src/player5.rs`, codec/tests, lib export: root93+2 in
debug/release pass. Tool author completed example/PowerShell benchmark: 5 tests
pass; genuine NPC20/100/1000 and aggregate20 measurements executed once while
development was concurrent (temp `c05-performance-baseline-01`); final run pending.
Owner explicitly approved one-way occupancy limitation after architecture review.

Active independent audits (no source authoring):
- Core: `ses_e127478e6ffeka2NRqKleaNyWy`.
- Client: `ses_e1271d88effea7XHC9JVb7V4NL`.
Parent full verifier `tools/c05_verify.ps1` completed: root93 tests+5 benchmark
tests+2 doctests in debug/release; app33 tests per profile (all old26 retained),
fmt/check/strict Clippy/packers/debug+release binaries pass. See verification log.
Native driver prepared `tools/c05_native.ps1`, syntax checked, not yet executed.
Preparing first verified implementation checkpoint; independent review not yet
closed. Next native driver with fresh approved temp evidence directory.

Next: complete integration, tests, scaling measurements, native gameplay, separate
critical audit and remediation. Commit/push only verified increments to this branch.
No C0.5 PASS, main merge or C0.6 authorization. Preserve all unrelated work.
Tools: Git `C:/Program Files/Git/cmd/git.exe`, cargo
`C:/Users/PC_4/.cargo/bin/cargo.exe`, Python `C:/Python314/python.exe`.
