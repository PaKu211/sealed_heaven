# AI Development Agent Workflow — G0 to M1

Version 0.2-draft · 17 November 2026

These instructions govern development agents for the standalone game.

## 1. Agent authorization boundary

The developer/owner controls scope and architecture approvals. An agent may inspect, plan, edit, run tests and report within a clearly authorized milestone. It must not expand into deferred systems, copy third-party code, delete unrelated files, publish releases or change architecture decisions without authorization. Existing user changes are preserved.

## 2. Mandatory task lifecycle

1. Read the [navigation and gate status](../README.md) and every document relevant to the authorized milestone. For implementation, use the current milestone specification and acceptance matrix.
2. Confirm current milestone and exact allowed changes; list non-goals.
3. Verify versions/APIs with pinned source or official docs before adding dependencies.
4. Propose bounded file/module changes and tests, identifying any ADR conflict.
5. Implement the smallest vertical technical increment with readable Rust and domain boundaries.
6. Add deterministic unit, property and scenario tests that verify independent invariants.
7. Run format/lint/tests/headless scenario as applicable; capture commands, results and environment.
8. Inspect diff for unrelated mutations, platform assumptions and accidentally changed acceptance criteria.
9. Submit a verification report with PASS/FAIL evidence and unresolved risks.
10. Update an ADR only when a decision genuinely changes and it has been reviewed.

## 3. Roles / review separation

Planning agent: translates milestone into bounded work packages. Implementation agent: builds code and tests. Verification agent or human reviewer: reads diff, runs independent fixtures and checks invariants. Do not treat the implementation agent's self-reported PASS as independent audit. If only one agent is available, use a distinct review pass and disclose limitation.

## 4. Required task prompt structure

Each coding-agent task must include: milestone ID, purpose, allowed files, out-of-scope list, canonical references, acceptance tests, dependency restrictions, expected evidence, rollback plan for failed experiment, and stop conditions.

## 5. Non-negotiable safeguards

- No invented crate APIs, function names, compile results, or benchmarks.
- No replacing failing tests with weak assertions to force a PASS.
- No direct gameplay mutation from Bevy Transform, animation or reactive UI callbacks.
- No Bevy Entity IDs in persistent saves.
- No global nondeterministic RNG for gameplay.
- No hidden introduction of scripting, LLM runtime, networking or multiplayer.
- No deep inheritance, giant generic engine or unexplained crate proliferation.
- No copying assets/proprietary algorithms without source/provenance/license review.
- Every schema/data change has migration/validation impact assessed.

## 6. Verification report template

Task/Milestone:  
Implemented scope:  
Files changed:  
Pinned build/toolchain:  
Tests and commands executed:  
Deterministic scenario (seed/input/content hash):  
Expected vs observed invariants:  
Performance evidence and hardware (if relevant):  
Known failures / skipped tests:  
ADRs touched / contradictions found:  
Gate verdict: PENDING / PASS / CONDITIONAL / FAIL  
Evidence status: NOT RUN / EXECUTED / PARTIAL

## 7. Gate progression

G0 requires independent R4 review and a recorded project-owner decision. The R1–R3 remediation author supplies documentation evidence and leaves the R4 verdict pending. C0.1 implementation starts only after separate project-owner authorization.

C0.1 requires executed C01-T01 through C01-T06. C0.2 requires C02-T01/T02, and C0.3 requires C03-T01 with its linked transaction checks. V0 records visual and mock recovery evidence. I0 records actual bridge and host evidence. M0 requires its headless matrix and the named I0 evidence. M1 requires the human playtest and timing evidence in the GDD. Each gate reports the test results and any authorized skips. A plan or successful build alone cannot close an implementation gate.
