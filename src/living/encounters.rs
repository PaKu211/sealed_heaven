//! Direct encounter memory. A conversation can exchange information at any time,
//! but repeatedly greeting the same person is not a new social encounter.
use super::{ActorId, Position, World};
use std::collections::BTreeMap;

pub const REVISION: u16 = 36;
pub const VERSION: u16 = 1;
pub const TALK_INTERVAL: u64 = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Kind {
    Talk,
    Care,
}

/// Last confirmed meeting, not the actor's current position or condition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Encounter {
    pub actor: ActorId,
    pub first_at: u64,
    pub last_at: u64,
    pub last_place: Position,
    pub last_kind: Kind,
    pub visits: u32,
    pub distinct_talks: u32,
    pub last_distinct_talk: Option<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Encounters {
    contacts: BTreeMap<ActorId, Encounter>,
}

impl Encounters {
    pub fn contact(&self, actor: ActorId) -> Option<&Encounter> {
        self.contacts.get(&actor)
    }

    pub fn contacts(&self) -> impl ExactSizeIterator<Item = &Encounter> {
        self.contacts.values()
    }

    fn record(&mut self, actor: ActorId, at: u64, place: Position, kind: Kind) -> bool {
        let e = self.contacts.entry(actor).or_insert(Encounter {
            actor,
            first_at: at,
            last_at: at,
            last_place: place,
            last_kind: kind,
            visits: 0,
            distinct_talks: 0,
            last_distinct_talk: None,
        });
        e.last_at = at;
        e.last_place = place;
        e.last_kind = kind;
        // At most one direct player encounter per bounded simulation tick.
        e.visits += 1;
        let distinct = kind == Kind::Talk
            && e.last_distinct_talk
                .is_none_or(|last| at.saturating_sub(last) >= TALK_INTERVAL);
        if distinct {
            e.last_distinct_talk = Some(at);
            e.distinct_talks += 1;
        }
        distinct
    }

    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = b"SH:C49:ENCOUNTERS:v1\0".to_vec();
        out.extend(VERSION.to_le_bytes());
        out.extend((self.contacts.len() as u32).to_le_bytes());
        for e in self.contacts.values() {
            out.extend(e.actor.get().to_le_bytes());
            out.extend(e.first_at.to_le_bytes());
            out.extend(e.last_at.to_le_bytes());
            out.extend(e.last_place.x.to_le_bytes());
            out.extend(e.last_place.y.to_le_bytes());
            out.push(e.last_kind as u8);
            out.extend(e.visits.to_le_bytes());
            out.extend(e.distinct_talks.to_le_bytes());
            out.push(u8::from(e.last_distinct_talk.is_some()));
            if let Some(at) = e.last_distinct_talk {
                out.extend(at.to_le_bytes());
            }
        }
        out
    }
}

impl World {
    pub fn encounters(&self) -> Option<&Encounters> {
        self.encounters.as_ref()
    }

    pub fn known_contacts(&self) -> Vec<Encounter> {
        let mut contacts = self
            .encounters()
            .map_or_else(Vec::new, |e| e.contacts().copied().collect());
        contacts.sort_by_key(|e| (std::cmp::Reverse(e.last_at), e.actor));
        contacts
    }

    pub(super) fn remember_encounter(
        &mut self,
        actor: ActorId,
        place: Position,
        kind: Kind,
    ) -> bool {
        // Prior revisions retain their exact conversation rules.
        self.encounters
            .as_mut()
            .is_none_or(|e| e.record(actor, self.tick, place, kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Intent, Outcome, PLAYER, Role};

    #[test]
    fn exact_interval_and_care_do_not_extend_talk_cooldown() {
        let a = ActorId::new(2);
        let b = ActorId::new(3);
        let p = Position { x: 1, y: 2 };
        let q = Position { x: 2, y: 3 };
        let mut e = Encounters::default();
        assert!(e.record(a, 1, p, Kind::Talk));
        assert!(!e.record(a, 2, p, Kind::Talk));
        assert!(e.record(b, 3, p, Kind::Talk));
        assert!(!e.record(a, 12, q, Kind::Care));
        assert!(e.record(a, 13, q, Kind::Talk));
        assert!(!e.record(a, 14, q, Kind::Talk));
        let a = e.contact(a).unwrap();
        assert_eq!(
            (a.first_at, a.last_at, a.visits, a.distinct_talks),
            (1, 14, 5, 2)
        );
        assert_eq!(a.last_place, q);
        assert_eq!(a.last_distinct_talk, Some(13));
    }

    #[test]
    fn direct_talk_credits_familiarity_once_and_remembers_only_actual_contact() {
        let mut w = World::encounter_campaign(0);
        let a = w
            .npcs()
            .find(|n| n.alive() && n.id() != PLAYER && n.definition.role != Role::Medic)
            .unwrap()
            .id();
        let pos = w.npc(a).unwrap().position;
        w.player.position = pos;
        let before = w.social.relationship(a, PLAYER).familiarity;
        w.tick = 1;
        assert_eq!(w.talk(a), Ok(Outcome::Spoke(a)));
        let first = w.social.relationship(a, PLAYER).familiarity;
        assert_eq!(first, (before + 1).min(8));
        w.tick = 2;
        assert_eq!(w.talk(a), Ok(Outcome::Spoke(a)));
        assert_eq!(w.social.relationship(a, PLAYER).familiarity, first);
        let seen = w.known_contacts();
        w.npcs.get_mut(&a).unwrap().position = Position {
            x: pos.x + 20,
            y: pos.y,
        };
        w.tick = 3;
        assert!(w.talk(a).is_err());
        assert_eq!(w.known_contacts(), seen);
        assert_eq!(seen[0].last_place, pos);
        assert_eq!(seen[0].visits, 2);
    }

    #[test]
    fn actual_clinic_service_is_remembered_without_suppressing_aid() {
        let mut w = World::encounter_campaign(0);
        let medic = w
            .npcs()
            .find(|n| n.definition.role == Role::Medic)
            .unwrap()
            .id();
        let clinic = w.scenario.clinic;
        w.npcs.get_mut(&medic).unwrap().position = clinic;
        w.player.position = clinic;
        w.player.hp = 2;
        w.supply.shrine = 0;
        w.tick = 1;
        assert!(w.talk(medic).is_err());
        assert!(w.known_contacts().is_empty());
        w.supply.shrine = 2;
        for tick in [2, 3] {
            w.tick = tick;
            assert_eq!(
                w.talk(medic),
                Ok(Outcome::Treated {
                    by: medic,
                    healed: 2
                })
            );
        }
        assert_eq!(w.player.hp, super::super::PLAYER_HP);
        assert_eq!(w.supply.shrine, 0);
        let e = w.encounters().unwrap().contact(medic).unwrap();
        assert_eq!(
            (e.first_at, e.last_at, e.visits, e.distinct_talks),
            (2, 3, 2, 0)
        );
        assert_eq!(e.last_kind, Kind::Care);
        assert_eq!(w.reciprocity().unwrap().net(medic, PLAYER), 2);
    }

    #[test]
    fn passive_world_preserves_c48_physical_state_and_is_deterministic() {
        for seed in [0, 1, 7, 42] {
            let mut prior = World::reciprocal_bonds_campaign(seed);
            let mut next = World::encounter_campaign(seed);
            let mut twin = next.clone();
            for _ in 0..200 {
                let a = prior.step(prior.tick(), Intent::Wait).unwrap();
                let b = next.step(next.tick(), Intent::Wait).unwrap();
                let c = twin.step(twin.tick(), Intent::Wait).unwrap();
                assert_eq!(a, b);
                assert_eq!(b, c);
                assert_eq!(prior.player(), next.player());
                assert_eq!(
                    prior.npcs().collect::<Vec<_>>(),
                    next.npcs().collect::<Vec<_>>()
                );
                assert_eq!(prior.supply(), next.supply());
                assert_eq!(prior.social(), next.social());
                assert_eq!(prior.reciprocity(), next.reciprocity());
            }
            assert_eq!(next, twin);
            assert!(next.known_contacts().is_empty());
            assert_eq!(World::restore(&next.checkpoint()).unwrap(), next);
        }
    }

    #[test]
    fn replay_and_revision_isolation() {
        let mut w = World::encounter_campaign(3);
        assert_eq!(w.checkpoint_revision(), REVISION);
        let old = World::reciprocal_bonds_campaign(3);
        assert!(old.encounters().is_none());
        assert_eq!(World::restore(&old.checkpoint()).unwrap(), old);
        for _ in 0..40 {
            w.step(w.tick(), Intent::Wait).unwrap();
        }
        assert!(w.known_contacts().is_empty());
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
    }
}
