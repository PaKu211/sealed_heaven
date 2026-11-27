//! Exact named-actor scheduling. A routine certificate replaces the full
//! utility search only when its result is provably the same. It never skips
//! movement, work, care, reports, or any other consequential commit.
use super::{ActorId, Npc, World, character_agency::Decision};
use std::collections::BTreeMap;

pub const REVISION: u16 = 30;
pub const WAKE_INTERVAL: u64 = 4;
pub const LOCAL_RADIUS: u32 = 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Mode {
    Active,
    Scheduled,
    Dead,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Entry {
    pub mode: Mode,
    pub next_wake: u64,
    pub full_evaluations: u64,
    pub routine_evaluations: u64,
    pub transitions: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scheduler {
    entries: BTreeMap<ActorId, Entry>,
}
impl Scheduler {
    pub(super) fn new(world: &World) -> Self {
        Self {
            entries: world
                .npcs()
                .map(|n| {
                    (
                        n.id(),
                        Entry {
                            mode: Mode::Active,
                            next_wake: 1,
                            full_evaluations: 0,
                            routine_evaluations: 0,
                            transitions: 0,
                        },
                    )
                })
                .collect(),
        }
    }
    pub fn entries(&self) -> &BTreeMap<ActorId, Entry> {
        &self.entries
    }
    pub fn entry(&self, id: ActorId) -> Option<Entry> {
        self.entries.get(&id).copied()
    }
    pub fn evaluation_totals(&self) -> (u64, u64) {
        self.entries.values().fold((0, 0), |(a, b), e| {
            (a + e.full_evaluations, b + e.routine_evaluations)
        })
    }
    fn set_mode(e: &mut Entry, mode: Mode) {
        if mode != e.mode {
            e.transitions += 1;
            e.mode = mode;
        }
    }
    pub(super) fn update(&mut self, id: ActorId, tick: u64, eligible: bool, used_routine: bool) {
        let e = self.entries.get_mut(&id).expect("stable actor identity");
        if eligible {
            Self::set_mode(e, Mode::Scheduled);
            if used_routine {
                e.routine_evaluations += 1;
            } else {
                e.full_evaluations += 1;
                e.next_wake = tick + WAKE_INTERVAL;
            }
        } else {
            Self::set_mode(e, Mode::Active);
            e.full_evaluations += 1;
            e.next_wake = tick + 1;
        }
    }
    pub(super) fn dead(&mut self, id: ActorId) {
        let e = self.entries.get_mut(&id).expect("stable actor identity");
        Self::set_mode(e, Mode::Dead);
        e.next_wake = 0;
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = b"SH:C43:LOD:v1\0".to_vec();
        out.extend((self.entries.len() as u32).to_le_bytes());
        for (&id, e) in &self.entries {
            out.extend(id.get().to_le_bytes());
            out.push(e.mode as u8);
            for n in [
                e.next_wake,
                e.full_evaluations,
                e.routine_evaluations,
                e.transitions,
            ] {
                out.extend(n.to_le_bytes());
            }
        }
        out
    }
}
impl World {
    pub fn scheduler(&self) -> Option<&Scheduler> {
        self.scheduler.as_ref()
    }
    pub(super) fn scheduled_plan(&self, npc: &Npc) -> (Decision, bool, bool) {
        let Some(entry) = self.scheduler().and_then(|s| s.entry(npc.id())) else {
            return (self.choose_decision(npc), false, false);
        };
        let certificate = self.routine_certificate(npc);
        let eligible = certificate.is_some();
        if let Some(decision) = certificate
            && entry.mode == Mode::Scheduled
            && self.tick < entry.next_wake
            && decision.goal == npc.activity.goal
        {
            return (decision, true, true);
        }
        (self.choose_decision(npc), eligible, false)
    }
    pub(super) fn retire_scheduled_dead(&mut self) {
        if let Some(scheduler) = self.scheduler.as_mut() {
            for npc in self.npcs.values().filter(|n| !n.alive()) {
                scheduler.dead(npc.id());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{
        Direction, Goal, Intent, Outcome, Position, Rules,
        expansion::CARRIER,
        navigation::{distance, next_step},
    };
    fn same_gameplay(optimized: &World, reference: &World) {
        let mut projected = optimized.clone();
        projected.scheduler = None;
        projected.rules = Rules::RegionalKnowledge;
        assert_eq!(
            &projected,
            reference,
            "gameplay divergence at tick {}",
            reference.tick()
        );
        assert_eq!(projected.state_digest(), reference.state_digest());
    }
    fn both(a: &mut World, b: &mut World, intent: Intent) -> Outcome {
        let result = a.step(a.tick(), intent);
        assert_eq!(result, b.step(b.tick(), intent));
        same_gameplay(a, b);
        result.unwrap()
    }

    #[test]
    fn exact_reference_replay_preserves_every_gameplay_field_and_deterministic_schedule() {
        for seed in [0, 1, 2, 7, 42, u32::MAX] {
            let mut a = World::exact_lod_campaign(seed);
            let mut b = World::regional_knowledge_campaign(seed);
            same_gameplay(&a, &b);
            for _ in 0..2000 {
                both(&mut a, &mut b, Intent::Wait);
                let s = a.scheduler().unwrap();
                assert_eq!(s.entries().len(), 12);
                for n in a.npcs() {
                    let e = s.entry(n.id()).unwrap();
                    assert_eq!(e.mode == Mode::Dead, !n.alive());
                    if n.alive() {
                        assert!(e.next_wake > a.tick() && e.next_wake <= a.tick() + WAKE_INTERVAL);
                    } else {
                        assert_eq!(e.next_wake, 0);
                    }
                }
                if matches!(a.tick(), 100 | 500 | 1000 | 2000) {
                    assert_eq!(World::restore(&a.checkpoint()).unwrap(), a);
                }
            }
            let (full, routine) = a.scheduler().unwrap().evaluation_totals();
            println!(
                "seed={seed} full={full} routine={routine} transitions={} shipped={} events={}",
                a.scheduler()
                    .unwrap()
                    .entries()
                    .values()
                    .map(|e| e.transitions)
                    .sum::<u64>(),
                a.frontier().unwrap().shipped_to_willowford,
                a.regional_news().unwrap().next_event()
            );
            assert_eq!(full + routine, 24000);
            assert!(
                routine > 1000,
                "scheduler must actually reduce full decisions"
            );
            assert!(a.frontier().unwrap().shipped_to_willowford > 0);
        }
    }

    fn travel(a: &mut World, b: &mut World, target: Position, reach: u32) {
        for _ in 0..500 {
            let from = a.player().position;
            if distance(from, target) <= reach {
                return;
            }
            let next = next_step(a.scenario, from, target, reach, |p| {
                a.walkable(p) && !a.occupied(p)
            });
            let intent = next.map_or(Intent::Wait, |p| {
                let dir = if p.x > from.x {
                    Direction::East
                } else if p.x < from.x {
                    Direction::West
                } else if p.y > from.y {
                    Direction::South
                } else {
                    Direction::North
                };
                Intent::Move(dir)
            });
            both(a, b, intent);
        }
        panic!("unreachable {target:?}");
    }

    #[test]
    fn local_scheduled_local_reentry_and_player_actions_match_full_reference() {
        let mut a = World::exact_lod_campaign(7);
        let mut b = World::regional_knowledge_campaign(7);
        both(
            &mut a,
            &mut b,
            Intent::ChooseBackground(crate::living::Background::Peddler),
        );
        assert_eq!(
            a.scheduler().unwrap().entry(CARRIER).unwrap().mode,
            Mode::Scheduled
        );
        let initial = a.npc(CARRIER).unwrap().definition;
        travel(&mut a, &mut b, crate::living::expansion::DEPOT, 1);
        assert!(
            a.scheduler()
                .unwrap()
                .entries()
                .values()
                .any(|e| e.transitions >= 2)
        );
        for intent in [
            Intent::BuyHerb,
            Intent::SellHerb,
            Intent::Talk(CARRIER),
            Intent::Wait,
            Intent::Wait,
        ] {
            both(&mut a, &mut b, intent);
        }
        travel(&mut a, &mut b, crate::living::expansion::GROVE, 1);
        both(&mut a, &mut b, Intent::Gather);
        let saved = a.checkpoint();
        assert_eq!(World::restore(&saved).unwrap(), a);
        travel(&mut a, &mut b, Position { x: 3, y: 5 }, 1);
        both(&mut a, &mut b, Intent::Deliver);
        for _ in 0..200 {
            both(&mut a, &mut b, Intent::Wait);
        }
        assert_eq!(a.npc(CARRIER).unwrap().definition, initial);
        assert_eq!(
            a.scheduler().unwrap().entry(CARRIER).unwrap().mode,
            Mode::Scheduled
        );
        assert!(a.scheduler().unwrap().entry(CARRIER).unwrap().transitions >= 3);
    }

    #[test]
    fn certificate_checks_all_fatigue_boundaries_and_interruptions() {
        let mut w = World::exact_lod_campaign(7);
        let far = w.npcs[&CARRIER].position;
        assert!(distance(w.player.position, far) > LOCAL_RADIUS);
        for fatigue in 0..=40 {
            for activity in [
                Goal::Idle,
                Goal::Rest,
                Goal::CampRest,
                Goal::SurveyWillowford,
            ] {
                let n = w.npcs.get_mut(&CARRIER).unwrap();
                n.fatigue = fatigue;
                n.activity.goal = activity;
                let n = &w.npcs[&CARRIER];
                assert_eq!(w.routine_certificate(n), Some(w.choose_decision(n)));
            }
        }
        w.npcs.get_mut(&CARRIER).unwrap().hp = 1;
        assert!(w.routine_certificate(&w.npcs[&CARRIER]).is_none());
        w.npcs.get_mut(&CARRIER).unwrap().hp = w.npcs[&CARRIER].definition.hp;
        w.player.position = far;
        assert!(w.routine_certificate(&w.npcs[&CARRIER]).is_none());
        w.player.position = w.scenario.entrance;
        w.npcs.get_mut(&CARRIER).unwrap().hp = 0;
        w.retire_scheduled_dead();
        let entry = w.scheduler().unwrap().entry(CARRIER).unwrap();
        assert_eq!(entry.mode, Mode::Dead);
        assert_eq!(entry.next_wake, 0);
        let hash = w.state_digest();
        w.retire_scheduled_dead();
        assert_eq!(hash, w.state_digest());
    }

    #[test]
    fn scheduler_work_is_bounded_and_hashes_future_wake_state() {
        let mut w = World::exact_lod_campaign(7);
        w.step(0, Intent::Wait).unwrap();
        let mut changed = w.clone();
        changed
            .scheduler
            .as_mut()
            .unwrap()
            .entries
            .get_mut(&CARRIER)
            .unwrap()
            .next_wake += 1;
        assert_ne!(changed.state_digest(), w.state_digest());
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
        assert_eq!(
            World::restore(&changed.checkpoint()),
            Err(crate::living::LoadError::StateMismatch)
        );
    }

    #[test]
    fn mixed_player_commands_preserve_reference_including_interruptions_and_rejections() {
        let actions = [
            Intent::Move(Direction::North),
            Intent::Move(Direction::East),
            Intent::Move(Direction::South),
            Intent::Move(Direction::West),
            Intent::Wait,
            Intent::Guard,
            Intent::Gather,
            Intent::Deliver,
            Intent::BuyHerb,
            Intent::SellHerb,
            Intent::Talk(ActorId::new(1)),
            Intent::Attack(ActorId::new(1)),
            Intent::Talk(CARRIER),
            Intent::Attack(CARRIER),
        ];
        for seed in [0u32, 1, 7, 42] {
            let mut a = World::exact_lod_campaign(seed);
            let mut b = World::regional_knowledge_campaign(seed);
            let mut rng = u64::from(seed) + 1;
            let mut rejected = 0;
            for _ in 0..1000 {
                rng = rng
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let intent = actions[((rng >> 32) as usize) % actions.len()];
                rejected +=
                    usize::from(matches!(both(&mut a, &mut b, intent), Outcome::Rejected(_)));
            }
            assert!(rejected > 0);
            assert_eq!(World::restore(&a.checkpoint()).unwrap(), a);
        }
    }
}
