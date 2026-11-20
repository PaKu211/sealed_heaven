# Game Design Document — Sealed Heaven

Version 0.2-draft · 17 November 2026

## 1. Game identity

Genre: pausable real-time systemic sandbox RPG with Eastern Fantasy content, discrete canonical simulation and 2D top-down symbolic pixel art. Core player promise: live as a person in a world that continues independently of you. No forced main quest.

## 2. Core gameplay loops

### Moment-to-moment

Observe surroundings -> choose an action -> pay time/resource/relationship cost -> world state changes -> receive readable feedback -> choose again.

### Short session

Find opportunity (work, training, trade or exploration) -> carry out a few actions -> earn skill/resource/reputation -> encounter an NPC or systemic obstacle -> observe consequence.

### Long arc (future)

Individual reputation and contacts -> optional companion/squad -> optional sect/organization -> optional settlement and regional influence. No mandatory tier unlock.

## 3. Time and combat

The M1 interaction prototype is pausable real-time. Canonical time is an integer tick. A scenario supplies ticks per game day. The host maps real elapsed time to step calls, and rendering interpolates read-only snapshots. Pause takes effect between steps and makes zero new step calls. The client experiment compares exact 10/20/30 steps per wall-clock second. An approved scenario conversion remains fixed across that experiment.

Commands enter a closed immutable batch for a defined boundary. Preflight is read-only. The simulation orders due commands, rechecks eligibility and commits whole outcomes serially. The actor-ID ordering in [C0.1](C0_1_IMPLEMENTATION_SPEC.md) applies to the fixture. Gameplay fairness and combat action priorities need their own rules and tests before adoption. M1 combat actions follow intent, eligibility/reservation, windup, commit, then recovery. Specify interrupt and cancel behavior for the combat fixture. Player, squad and autonomous actor actions use the same canonical commit discipline.

M1 combat boundary: one hostile encounter, basic movement/range/cover or obstacles, one melee attack, one defense/retreat option, and one generic technique costing energy. No massive tactical battle simulator or projectile physics engine required.

## 4. Player and character

Starting background changes initial skill access, residence, occupation, relationships, debt/obligations and equipment. M1 may ship with one curated background, while data design supports future variants. Skills progress through relevant action plus costs/conditions, not automatic universal XP alone.

Generic mechanics: attributes, needs, injuries, inventory, resource pools, progression tracks, abilities, conditions, modifiers, relationships, faction membership and long-running tasks. Ruleset-specific presentation: Qi, martial techniques, cultivator rank, sect duties and spirit herbs.

Death in the M1 smoke fixture ends the current attempt. Explicit reload of the pre-encounter checkpoint resumes that saved timeline, with the actor identity and world history up to the save. Dynasty continuation and ironman remain deferred.

## 5. NPCs and emergent life

NPCs possess persistent IDs, home/site, needs, work role, inventory/ownership, minimal personality, a small set of relationships and a serializable current task. Decision architecture: scheduled goal selection -> utility scores -> eligible job -> reservation -> navigation -> action -> event. Provide reasons for every autonomous choice in developer inspector.

M0 minimal goals: eat, rest, work, haul, return home, optional socialize. Avoid expensive planning every tick and GOAP framework in C0. Important named actors persist even offscreen.

## 6. World and simulation

M0: one region, two sites, two factions, 20 persistent NPCs, limited terrain/road graph, a few sources and stores. One local resource dispute demonstrates cause and consequence. World generation is deterministic and staged, but full procedural planet and prehistory remain later goals. Economy begins as physical input/storage/transport/recipe/output transactions.

The simulation owns active and scheduled named actor modes. Camera visibility can change projections only. Named actors retain their canonical records regardless of mode and visibility. Aggregate cohorts remain deferred. See the [LOD contract](SIMULATION_LOD_SPEC.md).

## 7. First playable vertical slice (M1)

1. Player starts near a frontier sect and receives contextual information, not a forced grand quest.
2. Move, talk to an NPC, and inspect the resource cue.
3. Gather an herb, accept a paid job, make the delivery and receive payment.
4. Make one trade and spend time and a generic pool or resource on an elementary technique.
5. Observe one NPC relationship change from the trade, job or another completed interaction.
6. Observe one local dispute and its later consequence. Joining the dispute is optional.
7. Save, advance, and reload. Check that world and NPC histories through the saved tick match. Rest is optional in the test.

### M1 content budget

One local map + nearby site, roughly 20 relevant NPCs, a few occupations, 6–10 resource/item kinds, 2 factions, 1 conflict, 1 generic progression track, 2–3 techniques maximum. All values are design caps for initial validation, not release promises.

## 8. Interaction, UI and controls

Top-down tile world, symbolic readable sprites. World view, compact health/energy status, context menu/inspector, message history. Character, inventory, relationships, map and task screens are keyboard navigable; mouse is optional convenience. Game UI reveals world facts the player can reasonably know, while developer UI may inspect hidden goals, RNG/state hashes, reservations and LOD.

## 9. Art and sound

Visual reference is the simplicity/readability of Of Tales and Deeds and Caves of Qud, not the rejected ornate AI-generated mockup. Handcrafted tiles and sprites, deterministic palette/variant changes, composed portraits and modest effects. Audio is deferred beyond minimal action/UI cues in V0.

## 10. Win/loss and player meaning

The game is an open-ended life simulation with meaningful local objectives, feedback and understandable danger. The M1 acceptance fixture uses the reload policy in §4.

## 11. Out of scope and unresolved questions

Large-scale politics, migration, automated dynasties, regional trade economics, detailed cultivation tribulations and complete world history are later milestones. Open at the owned gates: exact action timing and controls at M1, screen layout at V0, and content text style and session pacing at M1. Pause and boundary stamping follow [Technical Architecture](TECHNICAL_ARCHITECTURE.md#4-simulation-host).

## 12. Validation

M1-T01 is a human playtest from one authored starting checkpoint. Complete the required steps in §7 without developer assistance, identify one success or failure reason and explain which simulated resource or relationship change caused a later consequence. The tester records optional actions separately.

M1-T02 is a separate controlled death/reload check. Trigger a known lethal encounter after the pre-encounter save, verify the attempt ends, then explicitly reload and compare saved actor identity, world checkpoint and history prefix.

M1-T03 is the provisional host-cadence experiment under ADR-007/008. For exact 10/20/30 steps per wall second, repeat the same admitted input log twice per configuration with the same scenario conversion and compare checkpoints and events. Measure input-to-commit latency and CPU on recorded hardware, and have a tester complete movement, pause/resume and one encounter without stuck or skipped input. Select cadence after reviewing these measurements and qualitative results. A turn-based interaction prototype is optional if the cadence results leave the interaction choice open.
