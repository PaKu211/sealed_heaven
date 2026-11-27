//! Bounded prehistory and physical roadside relief caches. No generated actor,
//! unowned loot spawn, or remote knowledge. The old deposits are financed by
//! opening settlement stock; each cache is resolved exactly once.
use super::{
    ActorId, Goal, Npc, Outcome, PLAYER, Position, Rejection, Role, World, navigation::in_reach,
    worldgen::sample,
};
use crate::encoding::sha256;

pub const REVISION: u16 = 32;
pub const HISTORY_VERSION: u16 = 1;
pub const CACHE_COUNT: usize = 2;
const DOMAIN: u64 = 0xe529_9a41_7013_862b;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Route {
    West,
    East,
}
impl Route {
    pub const ALL: [Self; CACHE_COUNT] = [Self::West, Self::East];
    pub const fn recipient(self) -> ActorId {
        match self {
            Self::West => ActorId::new(32),
            Self::East => super::expansion::CARRIER,
        }
    }
    fn duty(self) -> Goal {
        match self {
            Self::West => Goal::CollectFreight,
            Self::East => Goal::CollectFrontierFreight,
        }
    }
    fn role(self) -> Role {
        match self {
            Self::West => Role::Courier,
            Self::East => Role::FrontierCarrier,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Deposit {
    pub id: u32,
    pub route: Route,
    pub position: Position,
    /// Days before tick zero, not a fabricated running-calendar tick.
    pub days_ago: u8,
    pub amount: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Method {
    Taken,
    Entrusted,
    Collected,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Resolution {
    pub at: u64,
    pub recipient: ActorId,
    pub method: Method,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cache {
    pub deposit: Deposit,
    pub remaining: u32,
    pub discovered_at: Option<u64>,
    pub resolution: Option<Resolution>,
    /// True only for direct player participation or visual witness at commit.
    pub witnessed_resolution: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalCache {
    pub deposit: Deposit,
    pub remaining: u32,
    pub can_take: bool,
    pub can_entrust: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoadHistory {
    caches: [Cache; CACHE_COUNT],
}
impl RoadHistory {
    pub(super) fn new(world: &mut World, seed: u32) -> Self {
        let bridges = [
            world.generated_region().expect("west map").bridges(),
            world.expanded_region().expect("east map").bridges(),
        ];
        let caches = Route::ALL.map(|route| {
            let i = route as usize;
            let id = i as u32 + 1;
            let position = bridges[i][(sample(seed, DOMAIN, id, 0) % 2) as usize];
            assert!(world.walkable(position));
            let amount = 1 + (sample(seed, DOMAIN, id, 1) % 2) as u32;
            let days_ago = 2 + (sample(seed, DOMAIN, id, 2) % 11) as u8;
            let stock = match route {
                Route::West => &mut world.region.as_mut().expect("market").market_herbs,
                Route::East => &mut world.frontier.as_mut().expect("depot").depot_herbs,
            };
            *stock = stock.checked_sub(amount).expect("funded historic deposit");
            Cache {
                deposit: Deposit {
                    id,
                    route,
                    position,
                    days_ago,
                    amount,
                },
                remaining: amount,
                discovered_at: None,
                resolution: None,
                witnessed_resolution: false,
            }
        });
        assert_ne!(caches[0].deposit.position, caches[1].deposit.position);
        Self { caches }
    }
    pub fn caches(&self) -> &[Cache; CACHE_COUNT] {
        &self.caches
    }
    pub fn cache(&self, id: u32) -> Option<&Cache> {
        self.caches.iter().find(|c| c.deposit.id == id)
    }
    fn cache_mut(&mut self, id: u32) -> Option<&mut Cache> {
        self.caches.iter_mut().find(|c| c.deposit.id == id)
    }
    fn write_deposit(out: &mut Vec<u8>, d: Deposit) {
        out.extend(d.id.to_le_bytes());
        out.push(d.route as u8);
        out.extend(d.position.x.to_le_bytes());
        out.extend(d.position.y.to_le_bytes());
        out.push(d.days_ago);
        out.extend(d.amount.to_le_bytes());
    }
    pub fn initial_digest(&self) -> [u8; 32] {
        let mut out = b"SH:C45:DEPOSITS:v1\0".to_vec();
        out.extend(HISTORY_VERSION.to_le_bytes());
        for c in &self.caches {
            Self::write_deposit(&mut out, c.deposit);
        }
        sha256(&out)
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = b"SH:C45:HISTORY:v1\0".to_vec();
        out.extend(HISTORY_VERSION.to_le_bytes());
        for c in &self.caches {
            Self::write_deposit(&mut out, c.deposit);
            out.extend(c.remaining.to_le_bytes());
            out.push(u8::from(c.discovered_at.is_some()));
            if let Some(at) = c.discovered_at {
                out.extend(at.to_le_bytes());
            }
            out.push(u8::from(c.resolution.is_some()));
            if let Some(r) = c.resolution {
                out.extend(r.at.to_le_bytes());
                out.extend(r.recipient.get().to_le_bytes());
                out.push(r.method as u8);
            }
            out.push(u8::from(c.witnessed_resolution));
        }
        out
    }
    fn resolve(&mut self, id: u32, tick: u64, recipient: ActorId, method: Method, witnessed: bool) {
        let c = self.cache_mut(id).expect("validated cache");
        assert!(c.remaining > 0 && c.resolution.is_none());
        c.remaining = 0;
        c.witnessed_resolution = witnessed;
        if witnessed && c.discovered_at.is_none() {
            c.discovered_at = Some(tick);
        }
        c.resolution = Some(Resolution {
            at: tick,
            recipient,
            method,
        });
    }
}
impl World {
    /// Full immutable domain inspector, intended for tests and developer tools.
    pub fn road_history(&self) -> Option<&RoadHistory> {
        self.road_history.as_ref()
    }
    /// Player memory includes immutable deposit facts, not remote current stock.
    pub fn known_deposits(&self) -> Vec<Deposit> {
        self.road_history().map_or_else(Vec::new, |h| {
            h.caches
                .iter()
                .filter(|c| c.discovered_at.is_some())
                .map(|c| c.deposit)
                .collect()
        })
    }
    /// A remembered outcome is evidence, not a live remote-state lookup.
    pub fn known_cache_resolution(&self, id: u32) -> Option<Resolution> {
        let c = self.road_history()?.cache(id)?;
        c.resolution.filter(|_| c.witnessed_resolution)
    }
    /// Only currently visible markers expose a live encounter.
    pub fn local_caches(&self) -> Vec<LocalCache> {
        self.road_history().map_or_else(Vec::new, |h| {
            h.caches
                .iter()
                .filter(|c| self.player_sees(c.deposit.position))
                .map(|c| {
                    let can_take =
                        c.remaining > 0 && in_reach(self.player.position, c.deposit.position);
                    LocalCache {
                        deposit: c.deposit,
                        remaining: c.remaining,
                        can_take,
                        can_entrust: can_take && self.entrust_recipient(c).is_some(),
                    }
                })
                .collect()
        })
    }
    pub(super) fn observe_road_history(&mut self) {
        let Some(h) = self.road_history() else {
            return;
        };
        let visible = h
            .caches
            .map(|c| c.discovered_at.is_none() && self.player_sees(c.deposit.position));
        let h = self.road_history.as_mut().unwrap();
        for (c, seen) in h.caches.iter_mut().zip(visible) {
            if seen && c.discovered_at.is_none() {
                c.discovered_at = Some(self.tick);
            }
        }
    }
    fn cache_recipient(&self, cache: &Cache) -> Option<&Npc> {
        let n = self.npc(cache.deposit.route.recipient())?;
        (cache.remaining > 0
            && n.alive()
            && n.hp == n.definition.hp
            && n.herbs == 0
            && n.definition.role == cache.deposit.route.role()
            && in_reach(n.position, cache.deposit.position)
            && self.perceives(n.position, cache.deposit.position))
        .then_some(n)
    }
    fn entrust_recipient(&self, cache: &Cache) -> Option<&Npc> {
        self.cache_recipient(cache).filter(|n| {
            in_reach(self.player.position, n.position)
                && self.player_sees(n.position)
                && !self.social.relationship(n.id(), PLAYER).refuses_aid()
        })
    }

    /// One transfer to the living freight owner. A westbound reserve counts
    /// toward the outstanding request, never manufacturing another order.
    fn assign_cache_cargo(&mut self, cache: Cache, to: ActorId) {
        self.npcs.get_mut(&to).expect("validated carrier").herbs = cache.remaining;
        if cache.deposit.route == Route::West {
            let r = self.region.as_mut().unwrap();
            r.order_remaining = r.order_remaining.saturating_sub(cache.remaining);
        }
    }

    fn check_player_cache(&self, id: u32) -> Result<Cache, Rejection> {
        let c = self
            .road_history()
            .and_then(|h| h.cache(id))
            .ok_or(Rejection::NoEvidence)?;
        if !self.player.alive() {
            return Err(Rejection::Dead);
        }
        if !in_reach(self.player.position, c.deposit.position)
            || !self.player_sees(c.deposit.position)
        {
            return Err(Rejection::OutOfReach);
        }
        if c.remaining == 0 {
            return Err(Rejection::Empty);
        }
        Ok(*c)
    }
    pub(super) fn recover_cache(&mut self, id: u32, entrust: bool) -> Result<Outcome, Rejection> {
        let cache = self.check_player_cache(id)?;
        let herbs = cache.remaining;
        if entrust {
            let npc = self.entrust_recipient(&cache).ok_or(Rejection::Refused)?;
            let to = npc.id();
            self.assign_cache_cargo(cache, to);
            self.social.aided(PLAYER, to);
            self.road_history
                .as_mut()
                .unwrap()
                .resolve(id, self.tick, to, Method::Entrusted, true);
            Ok(Outcome::CacheEntrusted { id, to, herbs })
        } else {
            self.player.herbs = self
                .player
                .herbs
                .checked_add(herbs)
                .ok_or(Rejection::PackFull)?;
            self.road_history
                .as_mut()
                .unwrap()
                .resolve(id, self.tick, PLAYER, Method::Taken, true);
            Ok(Outcome::CacheRecovered { id, herbs })
        }
    }
    /// Local opportunity only. This is the ordinary livelihood candidate,
    /// not an extra action or a priority override of injury/rest/obligations.
    pub(super) fn cache_livelihood(&self, npc: &Npc) -> Option<Goal> {
        if !matches!(npc.definition.role, Role::Courier | Role::FrontierCarrier) || npc.herbs != 0 {
            return None;
        }
        self.road_history()?
            .caches
            .iter()
            .find(|c| self.cache_recipient(c).is_some_and(|n| n.id() == npc.id()))
            .map(|c| c.deposit.route.duty())
    }
    /// Invoked only during the normal freight-work commit. Revalidates stock,
    /// role, physical reach and health after earlier ID-ordered commits.
    pub(super) fn collect_road_cache(&mut self, id: ActorId, goal: Goal) -> bool {
        if !matches!(goal, Goal::CollectFreight | Goal::CollectFrontierFreight) {
            return false;
        }
        let Some(h) = self.road_history() else {
            return false;
        };
        let Some(cache) = h
            .caches
            .iter()
            .find(|c| {
                c.deposit.route.duty() == goal
                    && self.cache_recipient(c).is_some_and(|n| n.id() == id)
            })
            .copied()
        else {
            return false;
        };
        let witnessed =
            self.player_sees(cache.deposit.position) && self.player_sees(self.npcs[&id].position);
        self.assign_cache_cargo(cache, id);
        self.road_history.as_mut().unwrap().resolve(
            cache.deposit.id,
            self.tick,
            id,
            Method::Collected,
            witnessed,
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Direction, Intent, StepError};
    fn physical(w: &World) -> u32 {
        w.player.herbs
            + w.npcs.values().map(|n| n.herbs).sum::<u32>()
            + w.region().unwrap().market_herbs
            + w.frontier().unwrap().depot_herbs
            + w.road_history()
                .unwrap()
                .caches()
                .iter()
                .map(|c| c.remaining)
                .sum::<u32>()
    }
    fn place_player(w: &mut World, id: u32) {
        let p = w
            .road_history()
            .unwrap()
            .cache(id)
            .unwrap()
            .deposit
            .position;
        w.player.position = p;
        w.observe_road_history();
    }
    fn place_both(w: &mut World, id: u32) -> ActorId {
        let deposit = w.road_history().unwrap().cache(id).unwrap().deposit;
        let actor = deposit.route.recipient();
        w.npcs.get_mut(&actor).unwrap().position = deposit.position;
        w.player.position = super::super::DIRECTIONS
            .into_iter()
            .filter_map(|d| super::super::navigation::moved(deposit.position, d))
            .find(|&p| w.walkable(p) && !w.occupied(p))
            .expect("connected bridge");
        w.observe_road_history();
        actor
    }

    #[test]
    fn deposit_generation_is_funded_seed_bound_and_does_not_change_map_or_actors() {
        let mut variants = std::collections::BTreeSet::new();
        for seed in (0..128).chain([u32::MAX, 0x8000_0000]) {
            let w = World::road_history_campaign(seed);
            let old = World::settlement_production_campaign(seed);
            assert_eq!(w.expanded_region(), old.expanded_region());
            assert_eq!(w.generated_region(), old.generated_region());
            assert!(w.npcs().eq(old.npcs()));
            let h = w.road_history().unwrap();
            let [west, east] = h.caches();
            assert_eq!(west.deposit.route, Route::West);
            assert_eq!(east.deposit.route, Route::East);
            assert!(
                w.generated_region()
                    .unwrap()
                    .bridges()
                    .contains(&west.deposit.position)
            );
            assert!(
                w.expanded_region()
                    .unwrap()
                    .bridges()
                    .contains(&east.deposit.position)
            );
            assert_eq!(
                w.region().unwrap().market_herbs + west.remaining,
                old.region().unwrap().market_herbs
            );
            assert_eq!(
                w.frontier().unwrap().depot_herbs + east.remaining,
                old.frontier().unwrap().depot_herbs
            );
            for c in h.caches() {
                assert!((1..=2).contains(&c.remaining));
                assert!((2..=12).contains(&c.deposit.days_ago));
                assert_eq!(c.remaining, c.deposit.amount);
                assert!(c.resolution.is_none() && w.walkable(c.deposit.position));
                assert_eq!(
                    w.npc(c.deposit.route.recipient()).unwrap().definition.role,
                    c.deposit.route.role()
                );
            }
            variants.insert(h.initial_digest());
            assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
        }
        assert!(variants.len() > 16);
    }

    #[test]
    fn recovery_is_local_atomic_exactly_once_and_history_does_not_leak_live_remote_stock() {
        let mut w = World::road_history_campaign(7);
        let before = w.clone();
        assert_eq!(w.recover_cache(1, false), Err(Rejection::OutOfReach));
        assert_eq!(w.recover_cache(u32::MAX, false), Err(Rejection::NoEvidence));
        assert_eq!(w, before);
        assert!(w.known_deposits().is_empty() && w.local_caches().is_empty());
        place_player(&mut w, 1);
        let deposit = w.known_deposits()[0];
        assert_eq!(deposit.id, 1);
        assert!(w.local_caches()[0].can_take);
        let total = physical(&w);
        let inventory = w.player.herbs;
        let initial = w.road_history().unwrap().initial_digest();
        assert_eq!(
            w.recover_cache(1, false),
            Ok(Outcome::CacheRecovered {
                id: 1,
                herbs: deposit.amount
            })
        );
        assert_eq!(w.player.herbs, inventory + deposit.amount);
        assert_eq!(physical(&w), total);
        let resolved = *w.road_history().unwrap().cache(1).unwrap();
        assert_eq!(resolved.resolution.unwrap().method, Method::Taken);
        let after = w.clone();
        assert_eq!(w.recover_cache(1, false), Err(Rejection::Empty));
        assert_eq!(w.recover_cache(1, true), Err(Rejection::Empty));
        assert_eq!(w, after);
        w.player.position = w.scenario.entrance;
        w.observe_road_history();
        assert_eq!(w.known_deposits(), vec![deposit]);
        assert!(w.local_caches().is_empty());
        assert_eq!(w.road_history().unwrap().initial_digest(), initial);
        w.player.hp = 0;
        assert!(w.local_caches().is_empty());
    }

    #[test]
    fn handover_requires_willing_live_empty_handed_local_carrier_and_updates_order_and_regard() {
        let mut w = World::road_history_campaign(7);
        let actor = place_both(&mut w, 1);
        let deposit = w.local_caches()[0].deposit;
        assert!(w.local_caches()[0].can_entrust);
        let before = w.clone();
        let total = physical(&w);
        let relation = w.social.relationship(actor, PLAYER);
        let order = w.region().unwrap().order_remaining;
        assert_eq!(
            w.recover_cache(1, true),
            Ok(Outcome::CacheEntrusted {
                id: 1,
                to: actor,
                herbs: deposit.amount
            })
        );
        assert_eq!(w.npc(actor).unwrap().herbs, deposit.amount);
        assert_eq!(
            w.region().unwrap().order_remaining,
            order.saturating_sub(deposit.amount)
        );
        assert_eq!(
            w.social.relationship(actor, PLAYER).regard,
            (relation.regard + 1).min(8)
        );
        assert_eq!(w.player.herbs, before.player.herbs);
        assert_eq!(physical(&w), total);
        assert!(!w.collect_road_cache(actor, Goal::CollectFreight));
        for (hp, herbs) in [(0, 0), (1, 0), (w.npc(actor).unwrap().definition.hp, 1)] {
            let mut candidate = before.clone();
            let n = candidate.npcs.get_mut(&actor).unwrap();
            n.hp = hp;
            n.herbs = herbs;
            assert!(!candidate.local_caches()[0].can_entrust);
            let state = candidate.clone();
            assert_eq!(candidate.recover_cache(1, true), Err(Rejection::Refused));
            assert_eq!(candidate, state);
        }
        let mut full = before.clone();
        full.player.herbs = u32::MAX;
        let state = full.clone();
        assert_eq!(full.recover_cache(1, false), Err(Rejection::PackFull));
        assert_eq!(full, state);
    }

    #[test]
    fn autonomous_collection_uses_only_correct_local_duty_and_one_physical_transfer() {
        let mut w = World::road_history_campaign(42);
        for id in [1, 2] {
            let actor = place_both(&mut w, id);
            let n = *w.npc(actor).unwrap();
            let cache = *w.road_history().unwrap().cache(id).unwrap();
            let duty = cache.deposit.route.duty();
            assert_eq!(w.cache_livelihood(&n), Some(duty));
            let before = w.clone();
            assert!(!w.collect_road_cache(ActorId::new(2), duty));
            assert!(!w.collect_road_cache(actor, Goal::Rest));
            assert_eq!(w, before);
            let total = physical(&w);
            assert!(w.collect_road_cache(actor, duty));
            assert_eq!(w.npc(actor).unwrap().position, n.position);
            assert_eq!(w.npc(actor).unwrap().herbs, cache.remaining);
            assert_eq!(
                w.road_history()
                    .unwrap()
                    .cache(id)
                    .unwrap()
                    .resolution
                    .unwrap()
                    .method,
                Method::Collected
            );
            assert_eq!(physical(&w), total);
            assert!(!w.collect_road_cache(actor, duty));
        }
    }

    #[test]
    fn new_intents_are_structurally_rejected_by_previous_revision_and_tampered_history_is_rejected()
    {
        let mut old = World::settlement_production_campaign(7);
        let before = old.checkpoint();
        for intent in [Intent::RecoverCache(1), Intent::EntrustCache(2)] {
            assert_eq!(old.step(0, intent), Err(StepError::UnrecordableIntent));
            assert_eq!(old.checkpoint(), before);
        }
        let w = World::road_history_campaign(7);
        let mut bad = w.clone();
        bad.road_history
            .as_mut()
            .unwrap()
            .cache_mut(1)
            .unwrap()
            .discovered_at = Some(0);
        assert_ne!(bad.state_digest(), w.state_digest());
        assert_eq!(
            World::restore(&bad.checkpoint()),
            Err(crate::living::LoadError::StateMismatch)
        );
        let mut bad = w.clone();
        bad.road_history
            .as_mut()
            .unwrap()
            .cache_mut(1)
            .unwrap()
            .deposit
            .days_ago += 1;
        assert_eq!(
            World::restore(&bad.checkpoint()),
            Err(crate::living::LoadError::ScenarioMismatch)
        );
        let mut w = World::road_history_campaign(7);
        assert_eq!(
            w.step(0, Intent::RecoverCache(u32::MAX)).unwrap(),
            Outcome::Rejected(Rejection::NoEvidence)
        );
        assert_eq!(&w.checkpoint()[54..59], &[42, 255, 255, 255, 255]);
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
        w.step(w.tick(), Intent::Move(Direction::North)).unwrap();
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
    }

    #[test]
    fn only_directly_witnessed_resolutions_enter_player_memory() {
        let mut witnessed = World::road_history_campaign(7);
        let actor = place_both(&mut witnessed, 1);
        let site = witnessed
            .road_history()
            .unwrap()
            .cache(1)
            .unwrap()
            .deposit
            .position;
        assert!(witnessed.player_sees(site));
        assert!(witnessed.collect_road_cache(actor, Goal::CollectFreight));
        let actual = witnessed
            .road_history()
            .unwrap()
            .cache(1)
            .unwrap()
            .resolution;
        assert_eq!(witnessed.known_cache_resolution(1), actual);
        witnessed.player.position = witnessed.scenario.entrance;
        assert!(witnessed.local_caches().is_empty());
        assert_eq!(witnessed.known_cache_resolution(1), actual);

        let mut remote = World::road_history_campaign(7);
        remote.npcs.get_mut(&actor).unwrap().position = site;
        assert!(!remote.player_sees(site));
        assert!(remote.collect_road_cache(actor, Goal::CollectFreight));
        assert!(remote.known_cache_resolution(1).is_none());
        place_player(&mut remote, 1);
        assert_eq!(remote.local_caches()[0].remaining, 0);
        assert_eq!(remote.known_deposits().len(), 1);
        assert!(
            remote.known_cache_resolution(1).is_none(),
            "empty cache is not evidence of who took it"
        );
    }
}
