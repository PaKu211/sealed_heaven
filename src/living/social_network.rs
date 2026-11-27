//! C5.0: local, optional social interactions among autonomous named NPCs.
//! Existing aid and combat transactions remain the authority for trust and
//! hostility. Greetings establish familiarity only, never goods or regard.
use super::{ActorId, DIRECTIONS, Goal, Npc, PLAYER, Position, World, navigation};
use std::collections::{BTreeMap, VecDeque};

pub const REVISION: u16 = 37;
pub const VERSION: u16 = 1;
pub const ACTOR_INTERVAL: u64 = 24;
pub const PAIR_INTERVAL: u64 = 96;
pub const HISTORY_CAP: usize = 128;
pub const OBSERVED_CAP: usize = 48;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Kind {
    Greeting,
    Aid,
    Conflict,
}

/// A committed public act, not a private relation score or balance.
/// Greeting participants are in ascending ID order; aid/conflict are directed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SocialEvent {
    pub at: u64,
    pub actor: ActorId,
    pub other: ActorId,
    pub kind: Kind,
    pub place: Position,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PairMemory {
    pub first_at: u64,
    pub last_at: u64,
    pub greetings: u32,
    pub aid_a_to_b: u32,
    pub aid_b_to_a: u32,
    pub conflicts_a_to_b: u32,
    pub conflicts_b_to_a: u32,
    pub last_greeting: Option<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SocialNetwork {
    pairs: BTreeMap<(ActorId, ActorId), PairMemory>,
    last_activity: BTreeMap<ActorId, u64>,
    history: VecDeque<SocialEvent>,
    observed: VecDeque<SocialEvent>,
}

impl SocialNetwork {
    fn pair(a: ActorId, b: ActorId) -> (ActorId, ActorId) {
        (a.min(b), a.max(b))
    }
    pub fn memory(&self, a: ActorId, b: ActorId) -> Option<&PairMemory> {
        self.pairs.get(&Self::pair(a, b))
    }
    pub fn pairs(&self) -> impl ExactSizeIterator<Item = (&(ActorId, ActorId), &PairMemory)> {
        self.pairs.iter()
    }
    pub fn history(&self) -> &VecDeque<SocialEvent> {
        &self.history
    }
    pub fn observed(&self) -> &VecDeque<SocialEvent> {
        &self.observed
    }
    fn ready(&self, id: ActorId, at: u64) -> bool {
        self.last_activity
            .get(&id)
            .is_none_or(|&last| at.saturating_sub(last) >= ACTOR_INTERVAL)
    }
    fn pair_ready(&self, a: ActorId, b: ActorId, at: u64) -> bool {
        self.memory(a, b)
            .and_then(|m| m.last_greeting)
            .is_none_or(|last| at.saturating_sub(last) >= PAIR_INTERVAL)
    }
    fn record(&mut self, event: SocialEvent, seen: bool) {
        debug_assert!(event.actor != event.other && event.actor != PLAYER && event.other != PLAYER);
        let pair = Self::pair(event.actor, event.other);
        let m = self.pairs.entry(pair).or_insert(PairMemory {
            first_at: event.at,
            ..PairMemory::default()
        });
        m.last_at = event.at;
        match event.kind {
            Kind::Greeting => {
                m.greetings += 1;
                m.last_greeting = Some(event.at);
            }
            Kind::Aid if event.actor == pair.0 => m.aid_a_to_b += 1,
            Kind::Aid => m.aid_b_to_a += 1,
            Kind::Conflict if event.actor == pair.0 => m.conflicts_a_to_b += 1,
            Kind::Conflict => m.conflicts_b_to_a += 1,
        }
        self.last_activity.insert(event.actor, event.at);
        self.last_activity.insert(event.other, event.at);
        if self.history.len() == HISTORY_CAP {
            self.history.pop_front();
        }
        self.history.push_back(event);
        if seen {
            if self.observed.len() == OBSERVED_CAP {
                self.observed.pop_front();
            }
            self.observed.push_back(event);
        }
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        fn events(out: &mut Vec<u8>, events: &VecDeque<SocialEvent>) {
            out.extend((events.len() as u32).to_le_bytes());
            for e in events {
                out.extend(e.at.to_le_bytes());
                out.extend(e.actor.get().to_le_bytes());
                out.extend(e.other.get().to_le_bytes());
                out.push(e.kind as u8);
                out.extend(e.place.x.to_le_bytes());
                out.extend(e.place.y.to_le_bytes());
            }
        }
        let mut out = b"SH:C50:SOCIAL_NETWORK:v1\0".to_vec();
        out.extend(VERSION.to_le_bytes());
        out.extend((self.pairs.len() as u32).to_le_bytes());
        for (&(a, b), m) in &self.pairs {
            out.extend(a.get().to_le_bytes());
            out.extend(b.get().to_le_bytes());
            out.extend(m.first_at.to_le_bytes());
            out.extend(m.last_at.to_le_bytes());
            for n in [
                m.greetings,
                m.aid_a_to_b,
                m.aid_b_to_a,
                m.conflicts_a_to_b,
                m.conflicts_b_to_a,
            ] {
                out.extend(n.to_le_bytes());
            }
            out.push(u8::from(m.last_greeting.is_some()));
            if let Some(at) = m.last_greeting {
                out.extend(at.to_le_bytes());
            }
        }
        out.extend((self.last_activity.len() as u32).to_le_bytes());
        for (&a, &at) in &self.last_activity {
            out.extend(a.get().to_le_bytes());
            out.extend(at.to_le_bytes());
        }
        events(&mut out, &self.history);
        events(&mut out, &self.observed);
        out
    }
}

impl World {
    /// Full simulation graph: developer or engine use, not player knowledge.
    pub fn social_network(&self) -> Option<&SocialNetwork> {
        self.social_network.as_ref()
    }
    /// The only player-facing projection. Private acts cannot evict this log.
    pub fn witnessed_social(&self) -> impl DoubleEndedIterator<Item = &SocialEvent> {
        self.social_network()
            .into_iter()
            .flat_map(|n| n.observed().iter())
    }
    fn record_npc_social(
        &mut self,
        a: ActorId,
        b: ActorId,
        from: Position,
        to: Position,
        kind: Kind,
    ) {
        if self.social_network.is_none() || a == PLAYER || b == PLAYER || a == b {
            return;
        }
        let seen = self.player.alive() && self.player_sees(from) && self.player_sees(to);
        let event = SocialEvent {
            at: self.tick,
            actor: a,
            other: b,
            kind,
            place: to,
        };
        self.social_network.as_mut().unwrap().record(event, seen);
    }
    pub(super) fn remember_npc_aid(
        &mut self,
        a: ActorId,
        b: ActorId,
        from: Position,
        to: Position,
    ) {
        self.record_npc_social(a, b, from, to, Kind::Aid);
    }
    pub(super) fn remember_npc_conflict(
        &mut self,
        a: ActorId,
        b: ActorId,
        from: Position,
        to: Position,
    ) {
        self.record_npc_social(a, b, from, to, Kind::Conflict);
    }
    fn greeting_available(&self, n: &Npc) -> bool {
        n.alive()
            && n.hp == n.definition.hp
            && !self.injuries().unwrap().people[&n.id()].injured()
            && n.fatigue <= n.definition.rest_threshold
            && !self.sect_duty_active(n.id())
            && matches!(
                n.activity.goal,
                Goal::Idle
                    | Goal::Rest
                    | Goal::CampRest
                    | Goal::Patrol
                    | Goal::TendClinic
                    | Goal::TeachAtHall
                    | Goal::StockGarden
                    | Goal::StockMarket
                    | Goal::CollectHerbs
                    | Goal::DeliverHerbs
                    | Goal::GatherFrontier
                    | Goal::StockFrontierDepot
                    | Goal::CollectFrontierFreight
                    | Goal::DeliverFrontierFreight
            )
    }
    fn greeting_allowed(&self, a: ActorId, b: ActorId) -> bool {
        let n = self.social_network().unwrap();
        let x = &self.npcs[&a];
        let y = &self.npcs[&b];
        a < b
            && self.greeting_available(x)
            && self.greeting_available(y)
            && navigation::distance(x.position, y.position) == 1
            && self.can_see(x.position, y.position)
            && !self.social.relationship(a, b).refuses_aid()
            && !self.social.relationship(b, a).refuses_aid()
            && n.ready(a, self.tick)
            && n.ready(b, self.tick)
            && n.pair_ready(a, b, self.tick)
    }
    /// Snapshot candidates from the occupied neighboring cells. Oldest pair
    /// first, then stable IDs. Revalidate on commit; one optional greeting per
    /// participant, no new movement, goal preemption or resource transfer.
    pub(super) fn advance_social_network(&mut self) {
        if self.social_network.is_none() {
            return;
        }
        let local: BTreeMap<_, _> = self
            .npcs()
            .filter(|n| n.alive())
            .map(|n| (n.position, n.id()))
            .collect();
        let mut candidates = Vec::new();
        for (&pos, &a) in &local {
            for d in DIRECTIONS {
                let Some(b) = navigation::moved(pos, d)
                    .and_then(|p| local.get(&p))
                    .copied()
                else {
                    continue;
                };
                if self.greeting_allowed(a, b) {
                    let last = self
                        .social_network()
                        .unwrap()
                        .memory(a, b)
                        .and_then(|m| m.last_greeting);
                    candidates.push((last, a, b));
                }
            }
        }
        candidates.sort_unstable();
        for (_, a, b) in candidates {
            if !self.greeting_allowed(a, b) {
                continue;
            }
            let (from, to) = (self.npcs[&a].position, self.npcs[&b].position);
            self.social.meet(a, b);
            self.record_npc_social(a, b, from, to, Kind::Greeting);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Intent, Outcome};

    fn event(at: u64, a: ActorId, b: ActorId, kind: Kind) -> SocialEvent {
        SocialEvent {
            at,
            actor: a,
            other: b,
            kind,
            place: Position { x: 1, y: 2 },
        }
    }

    #[test]
    fn separate_bounded_observed_history_and_directional_pair_memory() {
        let a = ActorId::new(2);
        let b = ActorId::new(3);
        let c = ActorId::new(4);
        let mut n = SocialNetwork::default();
        n.record(event(1, a, b, Kind::Greeting), true);
        n.record(event(2, b, a, Kind::Aid), true);
        n.record(event(3, a, b, Kind::Conflict), true);
        assert!(!n.ready(a, 26));
        assert!(n.ready(a, 27));
        assert!(!n.pair_ready(a, b, 96));
        assert!(n.pair_ready(a, b, 97));
        let seen = n.observed().clone();
        for t in 4..300 {
            n.record(event(t, b, c, Kind::Aid), false);
        }
        assert_eq!(n.history().len(), HISTORY_CAP);
        assert_eq!(n.observed(), &seen);
        let m = n.memory(b, a).unwrap();
        assert_eq!(
            (
                m.first_at,
                m.last_at,
                m.greetings,
                m.aid_b_to_a,
                m.conflicts_a_to_b
            ),
            (1, 3, 1, 1, 1)
        );
        assert_eq!(m.aid_a_to_b, 0);
        for t in 300..400 {
            n.record(event(t, b, c, Kind::Aid), true);
        }
        assert_eq!(n.observed().len(), OBSERVED_CAP);
        assert_eq!(n.pairs().len(), 2);
    }

    #[test]
    fn local_greetings_choose_oldest_pair_and_never_create_trust_or_goods() {
        let mut w = World::social_network_campaign(0);
        let ids = w
            .npcs()
            .filter(|n| w.greeting_available(n))
            .take(3)
            .map(Npc::id)
            .collect::<Vec<_>>();
        let [a, b, c] = [ids[0], ids[1], ids[2]];
        let (center, west, east, viewer) = (1..w.scenario.height - 1)
            .flat_map(|y| (1..w.scenario.width - 1).map(move |x| Position { x, y }))
            .find_map(|p| {
                let west = Position { x: p.x - 1, y: p.y };
                let east = Position { x: p.x + 1, y: p.y };
                let viewer = Position { x: p.x, y: p.y - 1 };
                ([p, west, east, viewer].into_iter().all(|q| w.walkable(q))
                    && w.can_see(viewer, p)
                    && w.can_see(viewer, west)
                    && w.can_see(viewer, east))
                .then_some((p, west, east, viewer))
            })
            .unwrap();
        for n in w.npcs.values_mut() {
            n.activity.goal = Goal::SeekCare;
        }
        for (id, pos) in [(a, center), (b, west), (c, east)] {
            let n = w.npcs.get_mut(&id).unwrap();
            n.position = pos;
            n.activity.goal = Goal::Idle;
            n.fatigue = 0;
        }
        for x in [a, b, c] {
            for y in [a, b, c] {
                w.social.relations.remove(&(x, y));
            }
        }
        w.player.position = viewer;
        let supply = w.supply();
        let people = w.npcs.clone();
        let agency = w.character_agency().unwrap().clone();
        let region = *w.region().unwrap();
        w.tick = 1;
        w.advance_social_network();
        let n = w.social_network().unwrap();
        assert_eq!(n.history().len(), 1);
        assert_eq!((n.history()[0].actor, n.history()[0].other), (a, b));
        assert_eq!(n.observed().len(), 1);
        assert_eq!(w.social.relationship(a, b).familiarity, 1);
        assert_eq!(w.social.relationship(a, b).regard, 0);
        assert_eq!(w.social.relationship(b, a).regard, 0);
        assert_eq!(w.social.relationship(a, c).familiarity, 0);
        assert!(w.social.relationship(a, b).trusts_news());
        w.tick = 24;
        w.advance_social_network();
        assert_eq!(w.social_network().unwrap().history().len(), 1);
        w.tick = 25;
        w.advance_social_network();
        assert_eq!(w.social_network().unwrap().history().len(), 2);
        assert_eq!(
            w.social_network().unwrap().memory(a, c).unwrap().greetings,
            1
        );
        assert_eq!(w.social.relationship(c, a).familiarity, 1);
        assert_eq!(w.social.relationship(c, a).regard, 0);
        // Remote history must not retroactively appear.
        let remote = (0..w.scenario.height)
            .flat_map(|y| (0..w.scenario.width).map(move |x| Position { x, y }))
            .find(|&p| w.walkable(p) && navigation::distance(p, center) > 12)
            .unwrap();
        w.player.position = remote;
        w.tick = 97;
        w.advance_social_network();
        assert_eq!(
            w.social_network().unwrap().memory(a, b).unwrap().greetings,
            2
        );
        assert_eq!(w.witnessed_social().count(), 2);
        w.player.position = viewer;
        assert_eq!(w.witnessed_social().count(), 2);
        assert_eq!(w.supply(), supply);
        assert_eq!(w.npcs, people);
        assert_eq!(w.character_agency().unwrap(), &agency);
        assert_eq!(w.region().unwrap(), &region);
        // Hostility, medical need and active combat all prevent optional greetings.
        w.social.relations.get_mut(&(b, a)).unwrap().regard = -1;
        w.tick = 193;
        w.npcs.get_mut(&c).unwrap().activity.goal = Goal::Retaliate;
        w.advance_social_network();
        assert_eq!(w.social_network().unwrap().history().len(), 3);
        w.npcs.get_mut(&c).unwrap().activity.goal = Goal::Idle;
        w.npcs.get_mut(&c).unwrap().hp -= 1;
        w.advance_social_network();
        assert_eq!(w.social_network().unwrap().history().len(), 3);
        let before = w.social.incidents().len();
        let hp = w.npcs[&b].hp;
        w.damage(a, b, 3, false);
        assert_eq!(w.social.incidents().len(), before + 1);
        assert!(w.npcs[&b].hp < hp);
        assert_eq!(
            w.social_network()
                .unwrap()
                .memory(a, b)
                .unwrap()
                .conflicts_a_to_b,
            1
        );
        assert_eq!(w.witnessed_social().last().unwrap().kind, Kind::Conflict);
        w.damage(a, b, 0, false);
        assert_eq!(
            w.social_network()
                .unwrap()
                .memory(a, b)
                .unwrap()
                .conflicts_a_to_b,
            1
        );
    }

    #[test]
    fn autonomous_social_phase_is_deterministic_and_replayable() {
        let mut total = 0;
        for seed in [0, 1, 7, 42] {
            let mut w = World::social_network_campaign(seed);
            let mut twin = w.clone();
            for _ in 0..1000 {
                assert_eq!(w.step(w.tick(), Intent::Wait), Ok(Outcome::Waited));
                twin.step(twin.tick(), Intent::Wait).unwrap();
            }
            assert_eq!(w, twin);
            assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            assert!(w.known_contacts().is_empty());
            let n = w.social_network().unwrap();
            total += n.pairs().map(|(_, m)| m.greetings).sum::<u32>();
            assert!(
                n.pairs()
                    .all(|(&(a, b), _)| a < b && a != PLAYER && b != PLAYER)
            );
        }
        assert!(
            total > 0,
            "natural autonomous greetings must actually occur"
        );
    }
}
