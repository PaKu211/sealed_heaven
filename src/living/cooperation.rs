//! C5.1: coordinated public freight dispatches using existing work, stock,
//! reserve and demand rules. A pair owns no additional inventory.
use super::{
    Activity, ActorId, Goal, Npc, Position, Region, Role, World,
    character_agency::{Decision, DecisionReason},
    content, expansion,
    navigation::in_reach,
    regional_news::Site,
};
use std::collections::{BTreeMap, VecDeque};

pub const REVISION: u16 = 38;
pub const VERSION: u16 = 1;
pub const DEADLINE: u64 = 240;
pub const HISTORY_CAP: usize = 96;
pub const OBSERVED_CAP: usize = 48;
pub const REGISTER_CAP: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum Route {
    Qinghe,
    Willowford,
}
impl Route {
    fn roles(self) -> (Role, Role) {
        match self {
            Self::Qinghe => (Role::Grower, Role::Courier),
            Self::Willowford => (Role::FrontierGrower, Role::FrontierCarrier),
        }
    }
    fn goals(self) -> (Goal, Goal) {
        match self {
            Self::Qinghe => (Goal::StockMarket, Goal::CollectFreight),
            Self::Willowford => (Goal::StockFrontierDepot, Goal::CollectFrontierFreight),
        }
    }
    fn hub(self) -> Position {
        match self {
            Self::Qinghe => Region::sites().market,
            Self::Willowford => expansion::DEPOT,
        }
    }
    fn destination(self, w: &World) -> Position {
        match self {
            Self::Qinghe => w.scenario.shrine,
            Self::Willowford => Region::sites().market,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Team {
    pub id: u32,
    pub route: Route,
    pub grower: ActorId,
    pub carrier: ActorId,
    pub opened: u64,
    pub deadline: u64,
    pub contributed: u32,
    pub shipment: u32,
    pub opening_seen: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Outcome {
    Dispatch,
    Delivered,
    CargoChanged,
    CarrierLost,
    Withdrawn,
    Expired,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Event {
    pub team: Team,
    pub at: u64,
    pub place: Position,
    pub outcome: Outcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Evidence {
    Witnessed,
    Register,
}

/// No private balances or live actor state in the known journal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Witnessed {
    pub id: u32,
    pub at: u64,
    pub route: Route,
    pub grower: ActorId,
    pub carrier: ActorId,
    pub contributed: u32,
    pub shipment: u32,
    pub place: Position,
    pub delivered: bool,
    pub evidence: Evidence,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Cooperation {
    active: BTreeMap<ActorId, Team>,
    history: VecDeque<Event>,
    observed: VecDeque<Witnessed>,
    registers: BTreeMap<Route, VecDeque<Witnessed>>,
    last_read: BTreeMap<Route, u32>,
    next_id: u32,
    completed: u32,
    cancelled: u32,
}
impl Cooperation {
    pub fn active(&self) -> &BTreeMap<ActorId, Team> {
        &self.active
    }
    pub fn history(&self) -> &VecDeque<Event> {
        &self.history
    }
    pub fn observed(&self) -> &VecDeque<Witnessed> {
        &self.observed
    }
    pub fn register(&self, route: Route) -> impl Iterator<Item = &Witnessed> {
        self.registers.get(&route).into_iter().flatten()
    }
    pub fn counts(&self) -> (u32, u32, u32) {
        (self.next_id, self.completed, self.cancelled)
    }
    fn occupied(&self, id: ActorId) -> bool {
        self.active
            .values()
            .any(|t| t.grower == id || t.carrier == id)
    }
    fn record(&mut self, team: Team, at: u64, place: Position, outcome: Outcome, seen: bool) {
        if self.history.len() == HISTORY_CAP {
            self.history.pop_front();
        }
        self.history.push_back(Event {
            team,
            at,
            place,
            outcome,
        });
        if matches!(outcome, Outcome::Dispatch | Outcome::Delivered) {
            let mut receipt = Witnessed {
                id: team.id,
                at,
                route: team.route,
                grower: team.grower,
                carrier: team.carrier,
                contributed: team.contributed,
                shipment: team.shipment,
                place,
                delivered: outcome == Outcome::Delivered,
                evidence: Evidence::Witnessed,
            };
            if seen {
                self.remember(receipt);
            }
            if receipt.delivered {
                receipt.evidence = Evidence::Register;
                let register = self.registers.entry(team.route).or_default();
                if register.len() == REGISTER_CAP {
                    register.pop_front();
                }
                register.push_back(receipt);
            }
        }
    }
    fn remember(&mut self, receipt: Witnessed) {
        if self
            .observed
            .iter()
            .any(|e| e.id == receipt.id && e.delivered == receipt.delivered)
        {
            return;
        }
        if self.observed.len() == OBSERVED_CAP {
            self.observed.pop_front();
        }
        self.observed.push_back(receipt);
    }
    fn read_register(&mut self, route: Route) {
        let last = self.last_read.get(&route).copied();
        let receipts: Vec<_> = self
            .register(route)
            .filter(|r| last.is_none_or(|id| r.id > id))
            .copied()
            .collect();
        for r in receipts {
            self.remember(r);
            self.last_read.insert(route, r.id);
        }
    }
    fn close(&mut self, carrier: ActorId, at: u64, place: Position, outcome: Outcome, seen: bool) {
        let Some(team) = self.active.remove(&carrier) else {
            return;
        };
        if outcome == Outcome::Delivered {
            self.completed += 1;
        } else {
            self.cancelled += 1;
        }
        self.record(team, at, place, outcome, seen && team.opening_seen);
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        fn team(out: &mut Vec<u8>, t: Team) {
            out.extend(t.id.to_le_bytes());
            out.push(t.route as u8);
            out.extend(t.grower.get().to_le_bytes());
            out.extend(t.carrier.get().to_le_bytes());
            out.extend(t.opened.to_le_bytes());
            out.extend(t.deadline.to_le_bytes());
            out.extend(t.contributed.to_le_bytes());
            out.extend(t.shipment.to_le_bytes());
            out.push(u8::from(t.opening_seen));
        }
        fn pos(out: &mut Vec<u8>, p: Position) {
            out.extend(p.x.to_le_bytes());
            out.extend(p.y.to_le_bytes());
        }
        let mut out = b"SH:C51:COOPERATION:v1\0".to_vec();
        out.extend(VERSION.to_le_bytes());
        for n in [
            self.next_id,
            self.completed,
            self.cancelled,
            self.active.len() as u32,
        ] {
            out.extend(n.to_le_bytes());
        }
        for t in self.active.values() {
            team(&mut out, *t);
        }
        out.extend((self.history.len() as u32).to_le_bytes());
        for e in &self.history {
            team(&mut out, e.team);
            out.extend(e.at.to_le_bytes());
            pos(&mut out, e.place);
            out.push(e.outcome as u8);
        }
        fn receipts(out: &mut Vec<u8>, rows: &VecDeque<Witnessed>) {
            out.extend((rows.len() as u32).to_le_bytes());
            for e in rows {
                out.extend(e.id.to_le_bytes());
                out.extend(e.at.to_le_bytes());
                out.push(e.route as u8);
                out.extend(e.grower.get().to_le_bytes());
                out.extend(e.carrier.get().to_le_bytes());
                out.extend(e.contributed.to_le_bytes());
                out.extend(e.shipment.to_le_bytes());
                pos(out, e.place);
                out.push(u8::from(e.delivered));
                out.push(e.evidence as u8);
            }
        }
        receipts(&mut out, &self.observed);
        out.extend((self.registers.len() as u32).to_le_bytes());
        for (&route, rows) in &self.registers {
            out.push(route as u8);
            receipts(&mut out, rows);
        }
        out.extend((self.last_read.len() as u32).to_le_bytes());
        for (&route, &id) in &self.last_read {
            out.push(route as u8);
            out.extend(id.to_le_bytes());
        }
        out
    }
}

type Planned = (ActorId, (Decision, bool, bool));
impl World {
    /// Engine/developer view. Player surfaces must use known_cooperation.
    pub fn cooperation(&self) -> Option<&Cooperation> {
        self.cooperation.as_ref()
    }
    pub fn known_cooperation(&self) -> impl DoubleEndedIterator<Item = &Witnessed> {
        self.cooperation()
            .into_iter()
            .flat_map(|c| c.observed.iter())
    }
    pub fn witnessed_cooperation(&self) -> impl DoubleEndedIterator<Item = &Witnessed> {
        self.known_cooperation()
            .filter(|e| e.evidence == Evidence::Witnessed)
    }
    /// Successful dispatch manifests are physically posted at the destination.
    /// Read them only with direct local access, never via the private graph.
    pub(super) fn observe_cooperation_receipts(&mut self) {
        if self.cooperation.is_none() || !self.player.alive() {
            return;
        }
        for route in [Route::Qinghe, Route::Willowford] {
            let site = route.destination(self);
            if in_reach(self.player.position, site) && self.can_see(self.player.position, site) {
                self.cooperation.as_mut().unwrap().read_register(route);
            }
        }
    }
    fn ready_worker(&self, n: &Npc) -> bool {
        n.alive()
            && n.hp == n.definition.hp
            && !self.injuries().unwrap().people[&n.id()].injured()
            && n.fatigue < n.definition.rest_threshold
            && self.companion(n.id()).is_none()
            && !self.sect_duty_active(n.id())
            && !self.traffic().unwrap().was_committed(n.id())
            && self.character_agency().unwrap().people[&n.id()].livelihood_herbs == 0
    }
    /// Amount allocated from the just-deposited load, after protecting reserve.
    fn dispatch_amount(
        &self,
        route: Route,
        grower: ActorId,
        carrier: ActorId,
    ) -> Option<(u32, u32)> {
        let c = self.cooperation()?;
        let a = &self.npcs[&grower];
        let b = &self.npcs[&carrier];
        if grower == carrier
            || c.occupied(grower)
            || c.occupied(carrier)
            || !self.ready_worker(a)
            || !self.ready_worker(b)
            || (a.definition.role, b.definition.role) != route.roles()
            || a.herbs == 0
            || b.herbs != 0
            || !in_reach(a.position, route.hub())
            || !in_reach(b.position, route.hub())
            || !self.can_see(a.position, b.position)
            || self.social.relationship(grower, carrier).refuses_aid()
            || self.social.relationship(carrier, grower).refuses_aid()
            || self.cache_livelihood(b).is_some()
        {
            return None;
        }
        let (stock, reserve, limit, demand) = match route {
            Route::Qinghe => {
                let r = self.region()?;
                (
                    r.market_herbs,
                    content::MARKET_RESERVE,
                    2,
                    r.order_remaining,
                )
            }
            Route::Willowford => {
                let known = self.known_stock(carrier, Site::WillowfordMarket)?;
                (
                    self.frontier()?.depot_herbs,
                    expansion::DEPOT_RESERVE,
                    expansion::CARAVAN_LOAD,
                    (content::MARKET_TARGET + expansion::WILLOWFORD_BUFFER).saturating_sub(known),
                )
            }
        };
        let load = stock
            .checked_add(a.herbs)?
            .saturating_sub(reserve)
            .min(limit)
            .min(demand);
        let fresh = load.min(a.herbs.saturating_sub(reserve.saturating_sub(stock)));
        (fresh > 0 && load > 0).then_some((fresh, load))
    }
    /// Pair two snapshot livelihood decisions; only Idle may be replaced by
    /// local loading when a fresh contribution makes it possible. Both work
    /// actions commit once, in grower-before-carrier order, through old ledgers.
    pub(super) fn commit_cooperative_dispatches(&mut self, plans: &[Planned]) {
        if self.cooperation.is_none() || self.tick >= super::MAX_TICKS {
            return;
        }
        for route in [Route::Qinghe, Route::Willowford] {
            let (stock, collect) = route.goals();
            let Some(&(a, (ad, _, _))) = plans.iter().find(|(id, (d, _, _))| {
                d.goal == stock && d.score <= 23 && self.npcs[id].definition.role == route.roles().0
            }) else {
                continue;
            };
            let Some(&(b, (mut bd, _, _))) = plans.iter().find(|(id, (d, _, _))| {
                (d.goal == Goal::Idle || d.goal == collect)
                    && d.score <= 23
                    && self.npcs[id].definition.role == route.roles().1
            }) else {
                continue;
            };
            if let Some((fresh, load)) = self.dispatch_amount(route, a, b) {
                if bd.goal != collect {
                    bd.goal = collect;
                    bd.reason = DecisionReason::Livelihood;
                }
                self.commit_dispatch(route, (a, ad), (b, bd), (fresh, load));
            }
        }
    }
    fn commit_dispatch(
        &mut self,
        route: Route,
        grower: (ActorId, Decision),
        carrier: (ActorId, Decision),
        amounts: (u32, u32),
    ) {
        let (a, ad) = grower;
        let (b, bd) = carrier;
        let (fresh, load) = amounts;
        let seen = self.player.alive()
            && self.player_sees(self.npcs[&a].position)
            && self.player_sees(self.npcs[&b].position);
        let at = self.tick;
        for (id, d) in [(a, ad), (b, bd)] {
            self.commit_decision(id, d);
            let n = self.npcs.get_mut(&id).unwrap();
            if n.activity.goal != d.goal {
                n.activity = Activity {
                    goal: d.goal,
                    since: at,
                };
            }
            n.fatigue = n.fatigue.saturating_add(1).min(40);
        }
        match route {
            Route::Qinghe => {
                self.unload_regional_cargo(a, true);
                self.load_regional_cargo(b, false);
            }
            Route::Willowford => {
                self.work_frontier(a, ad.goal);
                self.work_frontier(b, bd.goal);
            }
        }
        // Prevalidated atomic public stock/load operations, no new inventory.
        assert_eq!(self.npcs[&a].herbs, 0);
        assert_eq!(self.npcs[&b].herbs, load);
        for id in [a, b] {
            self.traffic.as_mut().unwrap().mark_committed(id);
        }
        let c = self.cooperation.as_mut().unwrap();
        let team = Team {
            id: c.next_id,
            route,
            grower: a,
            carrier: b,
            opened: at,
            deadline: (at + DEADLINE).min(super::MAX_TICKS),
            contributed: fresh,
            shipment: load,
            opening_seen: seen,
        };
        c.next_id += 1;
        c.active.insert(b, team);
        c.record(team, at, route.hub(), Outcome::Dispatch, seen);
    }
    pub(super) fn complete_cooperative_delivery(
        &mut self,
        carrier: ActorId,
        route: Route,
        amount: u32,
    ) {
        let Some(team) = self
            .cooperation()
            .and_then(|c| c.active.get(&carrier))
            .copied()
        else {
            return;
        };
        let place = self.npcs[&carrier].position;
        let success = team.route == route && team.shipment == amount && self.tick < team.deadline;
        let seen = self.player.alive() && self.player_sees(place);
        self.cooperation.as_mut().unwrap().close(
            carrier,
            self.tick,
            place,
            if success {
                Outcome::Delivered
            } else {
                Outcome::CargoChanged
            },
            success && seen,
        );
    }
    /// No physical consequences on cancellation. Actual cargo stays wherever
    /// existing work/combat put it, and blame cannot be inferred remotely.
    pub(super) fn reconcile_cooperation(&mut self) {
        let Some(c) = self.cooperation() else {
            return;
        };
        let closed: Vec<_> = c
            .active
            .values()
            .filter_map(|t| {
                let n = &self.npcs[&t.carrier];
                let outcome = if !n.alive() {
                    Outcome::CarrierLost
                } else if n.herbs != t.shipment {
                    Outcome::CargoChanged
                } else if self.companion(t.carrier).is_some() {
                    Outcome::Withdrawn
                } else if self.tick >= t.deadline {
                    Outcome::Expired
                } else {
                    return None;
                };
                Some((t.carrier, n.position, outcome))
            })
            .collect();
        let c = self.cooperation.as_mut().unwrap();
        for (id, pos, outcome) in closed {
            c.close(id, self.tick, pos, outcome, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{DIRECTIONS, Intent, PLAYER, navigation};

    fn find_workers(w: &World, route: Route) -> (ActorId, ActorId) {
        let (a, b) = route.roles();
        (
            w.npcs().find(|n| n.definition.role == a).unwrap().id(),
            w.npcs().find(|n| n.definition.role == b).unwrap().id(),
        )
    }
    fn fixture(route: Route) -> (World, ActorId, ActorId, Vec<Planned>) {
        let mut w = World::cooperation_campaign(0);
        let (a, b) = find_workers(&w, route);
        let hub = route.hub();
        let pos = DIRECTIONS
            .into_iter()
            .filter_map(|d| navigation::moved(hub, d))
            .find(|&p| w.walkable(p))
            .unwrap();
        w.npcs.get_mut(&a).unwrap().position = hub;
        w.npcs.get_mut(&b).unwrap().position = pos;
        w.npcs.get_mut(&a).unwrap().herbs = 2;
        w.npcs.get_mut(&b).unwrap().herbs = 0;
        w.player.position = hub;
        w.tick = 10;
        match route {
            Route::Qinghe => {
                let r = w.region.as_mut().unwrap();
                r.market_herbs = content::MARKET_RESERVE;
                r.order_remaining = 2;
            }
            Route::Willowford => {
                // A genuine direct dated receipt, then physical arrival at depot.
                w.npcs.get_mut(&b).unwrap().position = Region::sites().market;
                w.region.as_mut().unwrap().market_herbs = 0;
                w.observe_regional(0);
                w.npcs.get_mut(&b).unwrap().position = pos;
                w.frontier.as_mut().unwrap().depot_herbs = expansion::DEPOT_RESERVE;
            }
        }
        let (stock, collect) = route.goals();
        let plans = vec![
            (
                a,
                (
                    Decision {
                        goal: stock,
                        reason: DecisionReason::Livelihood,
                        score: 20,
                    },
                    false,
                    false,
                ),
            ),
            (
                b,
                (
                    Decision {
                        goal: Goal::Idle,
                        reason: DecisionReason::Routine,
                        score: 20,
                    },
                    false,
                    false,
                ),
            ),
        ];
        assert_eq!(w.dispatch_amount(route, a, b), Some((2, 2)), "{route:?}");
        assert_eq!(collect, route.goals().1);
        (w, a, b, plans)
    }

    #[test]
    fn both_routes_commit_real_work_once_and_complete_only_at_actual_destination() {
        for route in [Route::Qinghe, Route::Willowford] {
            let (mut w, a, b, plans) = fixture(route);
            let before = match route {
                Route::Qinghe => w.region().unwrap().market_herbs,
                Route::Willowford => w.frontier().unwrap().depot_herbs,
            };
            let regard = w.social.relationship(a, b).regard;
            let familiarity = w.social.relationship(a, b).familiarity;
            let fa = w.npcs[&a].fatigue;
            let fb = w.npcs[&b].fatigue;
            w.commit_cooperative_dispatches(&plans);
            assert_eq!(w.cooperation().unwrap().counts(), (1, 0, 0));
            assert_eq!(w.npcs[&a].herbs, 0);
            assert_eq!(w.npcs[&b].herbs, 2);
            assert_eq!(w.npcs[&a].fatigue, fa + 1);
            assert_eq!(w.npcs[&b].fatigue, fb + 1);
            assert!(w.traffic().unwrap().was_committed(a) && w.traffic().unwrap().was_committed(b));
            assert_eq!(w.social.relationship(a, b).regard, regard);
            assert_eq!(w.social.relationship(a, b).familiarity, familiarity);
            assert_eq!(w.witnessed_cooperation().count(), 1);
            let team = w.cooperation().unwrap().active()[&b];
            assert_eq!((team.contributed, team.shipment), (2, 2));
            match route {
                Route::Qinghe => {
                    assert_eq!(w.region().unwrap().market_herbs, before);
                    assert_eq!(w.region().unwrap().order_remaining, 0);
                }
                Route::Willowford => {
                    assert_eq!(w.frontier().unwrap().depot_herbs, before);
                }
            }
            let snapshot = w.clone();
            w.commit_cooperative_dispatches(&plans);
            assert_eq!(w, snapshot);
            let target = match route {
                Route::Qinghe => w.scenario.shrine,
                Route::Willowford => Region::sites().market,
            };
            w.tick += 1;
            w.npcs.get_mut(&b).unwrap().position = target;
            w.player.position = target;
            let stock = match route {
                Route::Qinghe => w.supply.shrine,
                Route::Willowford => w.region().unwrap().market_herbs,
            };
            match route {
                Route::Qinghe => w.unload_regional_cargo(b, false),
                Route::Willowford => w.work_frontier(b, Goal::DeliverFrontierFreight),
            }
            assert_eq!(w.cooperation().unwrap().counts(), (1, 1, 0));
            assert!(w.cooperation().unwrap().active().is_empty());
            assert_eq!(w.npcs[&b].herbs, 0);
            assert_eq!(
                match route {
                    Route::Qinghe => w.supply.shrine,
                    Route::Willowford => w.region().unwrap().market_herbs,
                },
                stock + 2
            );
            assert_eq!(w.witnessed_cooperation().count(), 2);
            assert!(w.witnessed_cooperation().last().unwrap().delivered);
        }
    }

    #[test]
    fn eligibility_reserve_and_no_remote_knowledge_or_fabricated_success() {
        let (mut w, a, b, plans) = fixture(Route::Qinghe);
        w.region.as_mut().unwrap().market_herbs = 0;
        let reserve = content::MARKET_RESERVE;
        w.npcs.get_mut(&a).unwrap().herbs = reserve;
        assert!(w.dispatch_amount(Route::Qinghe, a, b).is_none());
        w.npcs.get_mut(&a).unwrap().herbs = reserve + 1;
        assert_eq!(w.dispatch_amount(Route::Qinghe, a, b), Some((1, 1)));
        w.social.link(a, b);
        w.social.relations.get_mut(&(b, a)).unwrap().regard = -1;
        let snapshot = w.clone();
        w.commit_cooperative_dispatches(&plans);
        assert_eq!(w, snapshot);
        w.social.relations.get_mut(&(b, a)).unwrap().regard = 0;
        w.character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&a)
            .unwrap()
            .livelihood_herbs = 1;
        let snapshot = w.clone();
        w.commit_cooperative_dispatches(&plans);
        assert_eq!(w, snapshot);
        w.character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&a)
            .unwrap()
            .livelihood_herbs = 0;
        w.npcs.get_mut(&b).unwrap().hp -= 1;
        let snapshot = w.clone();
        w.commit_cooperative_dispatches(&plans);
        assert_eq!(w, snapshot);
        w.npcs.get_mut(&b).unwrap().hp += 1;
        w.player.position = w.scenario.patch;
        assert!(!w.player_sees(w.npcs[&b].position));
        w.commit_cooperative_dispatches(&plans);
        assert_eq!(w.cooperation().unwrap().counts(), (1, 0, 0));
        assert_eq!(w.witnessed_cooperation().count(), 0);
        w.npcs.get_mut(&b).unwrap().position = w.scenario.shrine;
        w.player.position = w.scenario.shrine;
        w.tick += 1;
        w.unload_regional_cargo(b, false);
        assert_eq!(w.cooperation().unwrap().counts(), (1, 1, 0));
        // Seeing the delivery alone cannot reveal its unknown partner.
        assert_eq!(w.witnessed_cooperation().count(), 0);
        assert_eq!(w.known_cooperation().count(), 0);
        w.observe_cooperation_receipts();
        assert_eq!(w.witnessed_cooperation().count(), 0);
        let receipt = w.known_cooperation().last().unwrap();
        assert_eq!(receipt.evidence, Evidence::Register);
        assert!(receipt.delivered);
        assert_eq!(receipt.grower, a);
        let snap = w.clone();
        w.observe_cooperation_receipts();
        assert_eq!(w, snap);
        w.player.position = w.scenario.patch;
        assert_eq!(w.known_cooperation().count(), 1);
        let (mut f, fa, fb, _) = fixture(Route::Willowford);
        f.tick += 10000;
        assert!(f.dispatch_amount(Route::Willowford, fa, fb).is_none());
        assert!(f.social.relationship(PLAYER, fa).familiarity <= 8);
    }

    #[test]
    fn failure_and_timeout_never_destroy_or_replace_cargo() {
        for failure in [
            Outcome::CargoChanged,
            Outcome::CarrierLost,
            Outcome::Expired,
        ] {
            let (mut w, _, b, plans) = fixture(Route::Qinghe);
            w.commit_cooperative_dispatches(&plans);
            let team = w.cooperation().unwrap().active()[&b];
            match failure {
                Outcome::CargoChanged => {
                    w.npcs.get_mut(&b).unwrap().herbs -= 1;
                    w.player.herbs += 1;
                }
                Outcome::CarrierLost => w.npcs.get_mut(&b).unwrap().hp = 0,
                Outcome::Expired => w.tick = team.deadline,
                _ => unreachable!(),
            }
            let cargo = w.npcs[&b].herbs;
            let stock = w.supply;
            w.reconcile_cooperation();
            assert_eq!(w.cooperation().unwrap().counts(), (1, 0, 1));
            assert_eq!(
                w.cooperation().unwrap().history().back().unwrap().outcome,
                failure
            );
            assert_eq!(w.witnessed_cooperation().count(), 1);
            assert_eq!(w.npcs[&b].herbs, cargo);
            assert_eq!(w.supply, stock);
            let snapshot = w.clone();
            w.reconcile_cooperation();
            assert_eq!(w, snapshot);
        }
    }

    #[test]
    fn register_is_bounded_and_repeated_reads_never_duplicate_receipts() {
        let (w, a, b, _) = fixture(Route::Qinghe);
        let mut c = Cooperation::default();
        for id in 0..(REGISTER_CAP as u32 + 10) {
            let t = Team {
                id,
                route: Route::Qinghe,
                grower: a,
                carrier: b,
                opened: u64::from(id),
                deadline: 100,
                contributed: 1,
                shipment: 1,
                opening_seen: false,
            };
            c.record(
                t,
                u64::from(id) + 1,
                w.scenario.shrine,
                Outcome::Delivered,
                false,
            );
        }
        assert_eq!(c.register(Route::Qinghe).count(), REGISTER_CAP);
        assert!(c.observed().is_empty());
        c.read_register(Route::Willowford);
        assert!(c.observed().is_empty());
        c.read_register(Route::Qinghe);
        assert_eq!(c.observed().len(), REGISTER_CAP);
        assert!(
            c.observed()
                .iter()
                .all(|r| r.evidence == Evidence::Register)
        );
        let snap = c.clone();
        c.read_register(Route::Qinghe);
        assert_eq!(c, snap);
        assert_eq!(c.observed().front().unwrap().id, 10);
    }

    #[test]
    fn natural_passive_teams_and_replay() {
        let mut total = 0;
        let mut delivered = 0;
        for seed in [0, 1, 7, 42] {
            let mut w = World::cooperation_campaign(seed);
            let mut twin = w.clone();
            for _ in 0..2000 {
                assert_eq!(
                    w.step(w.tick(), Intent::Wait),
                    twin.step(twin.tick(), Intent::Wait)
                );
            }
            assert_eq!(w, twin);
            assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            let c = w.cooperation().unwrap();
            let (opened, done, failed) = c.counts();
            total += opened;
            delivered += done;
            assert_eq!(opened, done + failed + c.active().len() as u32);
            println!(
                "seed {seed}: opened={opened} delivered={done} cancelled={failed} active={} observed={}",
                c.active().len(),
                c.observed().len()
            );
        }
        assert!(
            total > 0 && delivered > 0,
            "cooperative work must emerge naturally, not only in fixtures"
        );
    }
}
