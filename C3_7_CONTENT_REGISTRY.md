# C3.7 — Typed content registry foundation

## Scope and contract
This is the standalone Rust/Bevy project, Windows x64 only. C3.6 baseline a3de8e2e40c1c2a3adcdc6f1dbc7abdb199e2cd9. The GDD section 9.5 distinguishes authored definitions from runtime state and requires typed, validated references and an explicit content migration pipeline. Reconciliation section 6 requires schema, stable IDs, dependency resolution and version migration before new data-driven gameplay. Those requirements motivate this first registry release, not unrestricted mod execution.

The content/v1/core.json and content/v1/sealed_heaven.json packs contain 43 definitions: 5 items, 2 tiles, 1 recipe, 10 actors, 3 maps, 9 scenario variants, 1 region, 3 backgrounds, 2 institutions, 3 techniques, and 4 initial physical lots. They reproduce existing authored C3.5.1/C3.6 content; no new lore, areas, skills or economic mechanics are implied. The source lore export must be audited and mapped separately before activation.

## Data flow
- Strict serde structs with deny_unknown_fields compile flat authored JSON using the host build script. Unknown schema, fields, variants, duplicate stable IDs or numeric IDs, missing/wrong-version dependencies, cycles, invalid bounds and cross-scenario references are rejected.
- Flood fill checks that each scenario's entrance, sites, actor spawns/homes/routes, relevant regional anchors and relic lots are reachable; contacts/recruits/report targets resolve within the scenario.
- Generated static Scenario values replace the handwritten Qinghe/Riverlands scenario tables. Campaign ranges 1–26 are selected from the pack. The region/economy, starting backgrounds, remedy recipe, faction treasury, rivalry, sect rank initialization and merit, martial profiles/tuition, and initial lots consume generated values.
- Definition identity uses namespaced IDs, explicit stable actor/lot wire IDs and typed bindings. Runtime lookup is a sorted static slice. There is no JSON parser, filesystem access or data-dependent hashing inside the simulation turn.
- Compile/test-only serde 1.0.229 and serde_json 1.0.154 are pinned in both lockfiles; normal sealed_sim dependencies remain rand_chacha and sha2. Bevy remains a presentation adapter.
- Canonical serialization preserves array order (historical scenario digest order is significant) and normalizes object key order/whitespace. SHA-256 domain SH:CONTENT:REGISTRY:v1 plus length-prefixed typed pack encodings. The historical v1 fingerprint is 5d33f089bf1086ed4e9465f4ef3be8254b8090c6a35fcbdf9a2e9f9bb0451b80, pinned in build.rs. Per-pack digests and resolved definition bindings appear in the manifest.

## Version and checkpoint policy
All original checkpoint revisions 1–26 keep their exact historical scenario/state hashes and wire format. Their content identity is resolved from the immutable v1 mapping, not retroactively inserted in old save bytes. Old saves remain in their original slots. Do not change the fingerprint pin to bypass compatibility. No new gameplay revision is necessary for this extraction.

For future content:
1. Copy both packs to a draft directory; leave content/v1 unchanged.
2. Edit typed data, increment each changed pack version by exactly one, and update declared dependencies.
3. Run the same compiler in the authoring tool. Stable IDs, kinds, typed bindings and wire IDs may not be removed or rebound without a separate migration implementation.
4. Review the upgrade plan. A changed fingerprint requires a new rules revision and explicit content identity in future checkpoint metadata. Preserve the historical v1 registry and binding. Design state migration explicitly if desired; never replay a legacy journal against new definitions silently.
5. Review engine semantics, scenario tests and compatibility before activating a new pack. Structural validation is not a proof that a modified world is balanced or all new mechanics are supported.

There is no runtime pack replacement, arbitrary scripting, automatic save conversion, universal item/recipe engine, or worldgen expansion yet. Item kinds, terrain, action handlers and the nine historical scenario shapes still use the current typed engine semantics. The registry is an authoring foundation for later mechanics, not a generic interpreter.

## Commands (PowerShell, repo root)
    cargo run --locked --offline --example content_tool -- validate content/v1/core.json content/v1/sealed_heaven.json
    cargo run --locked --offline --example content_tool -- plan-upgrade D:\draft\core.json D:\draft\sealed_heaven.json
    .\sealed_agency.exe --content-info

The authoring tool emits JSON, exits 2 on rejection, and never changes a pack or a save. The actual game binary exposes a read-only manifest. Locale packs from C3.6 remain independent.

## Regression and provenance
The original C3.6 checkout was retained. A detached fixture-capture worktree at that commit appended only the recorded capture_golden.rs test driver. It emitted 294 SHA-256 hashes for 98 revision/seed cases: revisions 1–2 seed 0, revisions 3–26 seeds 0/1/2/7, checkpoints at ticks 0/40/1000. Fixture tests/fixtures/living/c36_registry_golden.csv has SHA-256 aace8d9da06b80c6f168f15eda0a96d0a365bc45b927a2b54f380e906d60ae0f. The C3.7 test reproduces every hash and restores every checkpoint. The pre-existing binary fixtures remain unchanged.

Further acceptance: locked/offline Rust 1.100.0 Windows core/headless/desktop debug and release, strict Clippy, executable replay, native GUI smoke, pinned-pack mutation rejection, exact source/patch verification and distribution extraction. Actual gate results are kept outside the source tree. No independent audit, full human playtest, Linux or hosted CI is claimed.
