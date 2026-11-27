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
    pub(super) fn hub(self) -> Position {
        match self {
            Self::Qinghe => Region::sites().market,
            Self::Willowford => expansion::DEPOT,
        }
    }
    pub(super) fn destination(self, w: &World) -> Position {
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
    fn read_register(&mut self, route: Route) -> Vec<Witnessed> {
        let last = self.last_read.get(&route).copied();
        let receipts: Vec<_> = self
            .register(route)
            .filter(|r| last.is_none_or(|id| r.id > id))
            .copied()
            .collect();
        for &r in &receipts {
            self.remember(r);
            self.last_read.insert(route, r.id);
        }
        receipts
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
                let receipts = self.cooperation.as_mut().unwrap().read_register(route);
                for r in receipts {
                    self.story_register(r);
                }
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
            || !self.can_offer_commitment(grower, carrier)
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
        self.open_obligation(team);
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
        let outcome = if success {
            Outcome::Delivered
        } else {
            Outcome::CargoChanged
        };
        self.cooperation.as_mut().unwrap().close(
            carrier,
            self.tick,
            place,
            outcome,
            success && seen,
        );
        self.close_obligation(team, outcome, place);
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
        for (id, pos, outcome) in closed {
            let team = self.cooperation().unwrap().active()[&id];
            self.cooperation
                .as_mut()
                .unwrap()
                .close(id, self.tick, pos, outcome, false);
            self.close_obligation(team, outcome, pos);
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
        fixture_for(World::cooperation_campaign(0), route)
    }
    fn fixture_for(mut w: World, route: Route) -> (World, ActorId, ActorId, Vec<Planned>) {
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
    fn c57_partnership_requires_mutually_known_delivery_and_links_later_real_projects() {
        use crate::living::circles::{Kind, Purpose};
        let (mut w, a, b, plans) = fixture_for(World::circles_campaign(0), Route::Qinghe);
        w.commit_cooperative_dispatches(&plans);
        let first = w.cooperation().unwrap().active()[&b].id;
        assert!(w.circles().unwrap().groups().is_empty());
        w.tick += 1;
        w.npcs.get_mut(&b).unwrap().position = w.scenario.shrine;
        w.unload_regional_cargo(b, false);
        w.advance_circles();
        assert!(w.circles().unwrap().groups().is_empty());
        let hub = w.npcs[&a].position;
        let cell = DIRECTIONS
            .into_iter()
            .filter_map(|d| navigation::moved(hub, d))
            .find(|&p| w.walkable(p))
            .unwrap();
        w.tick += 1;
        w.npcs.get_mut(&b).unwrap().position = cell;
        w.advance_obligations();
        w.advance_circles();
        let c = w.circles().unwrap();
        let g = c.group_of(a).unwrap();
        assert_eq!(g.purpose, Purpose::Partnership);
        assert!(g.members.contains_key(&b));
        assert!(
            c.history()
                .iter()
                .any(|e| e.kind == Kind::Formed && e.evidence == Some(first))
        );
        let group = g.id;
        w.tick += 1;
        w.traffic.as_mut().unwrap().begin_tick();
        w.npcs.get_mut(&a).unwrap().herbs = 2;
        w.npcs.get_mut(&b).unwrap().herbs = 0;
        w.region.as_mut().unwrap().market_herbs = content::MARKET_RESERVE;
        w.region.as_mut().unwrap().order_remaining = 2;
        let stock = w.supply.shrine;
        assert_eq!(w.dispatch_amount(Route::Qinghe, a, b), Some((2, 2)));
        w.commit_cooperative_dispatches(&plans);
        let second = w.cooperation().unwrap().active()[&b].id;
        assert_ne!(first, second);
        assert_eq!(w.circles().unwrap().projects()[&second].circle, group);
        w.tick += 1;
        w.npcs.get_mut(&b).unwrap().position = w.scenario.shrine;
        w.unload_regional_cargo(b, false);
        assert_eq!(w.supply.shrine, stock + 2);
        let c = w.circles().unwrap();
        assert!(c.projects().is_empty());
        assert_eq!(c.count(Kind::Project), 1);
        assert_eq!(c.count(Kind::Delivered), 1);
        assert!(
            c.history()
                .iter()
                .any(|e| e.kind == Kind::Delivered && e.evidence == Some(second))
        );
    }

    #[test]
    fn c56_success_changes_personal_focus_only_when_each_partner_knows() {
        use crate::living::development::{Cause, Focus, ProjectEvidence};
        let (mut w, a, b, plans) = fixture_for(World::development_campaign(0), Route::Qinghe);
        w.commit_cooperative_dispatches(&plans);
        let id = w.cooperation().unwrap().active()[&b].id;
        let initial = w.development().unwrap().profiles()[&a];
        w.tick += 1;
        w.npcs.get_mut(&b).unwrap().position = w.scenario.shrine;
        w.unload_regional_cargo(b, false);
        let d = w.development().unwrap();
        assert_eq!(d.profiles()[&a], initial);
        assert_eq!(d.profiles()[&b].focus, Focus::Partnership);
        assert_eq!(d.profiles()[&b].successful_projects, 1);
        assert_eq!(d.partners()[&(b, a)].last_project, id);
        assert!(!d.partners().contains_key(&(a, b)));
        assert!(matches!(
            d.history().back().unwrap().cause,
            Cause::Project {
                evidence: ProjectEvidence::OwnWork,
                ..
            }
        ));
        assert_eq!(w.development_partner_bonus(b, a), 2);
        w.advance_obligations();
        assert_eq!(w.development().unwrap().profiles()[&a], initial);
        let near = w.npcs[&a].position;
        let cell = DIRECTIONS
            .into_iter()
            .filter_map(|d| navigation::moved(near, d))
            .find(|&p| w.walkable(p))
            .unwrap();
        w.tick += 1;
        w.npcs.get_mut(&b).unwrap().position = cell;
        w.advance_obligations();
        let d = w.development().unwrap();
        assert_eq!(d.profiles()[&a].focus, Focus::Partnership);
        assert_eq!(d.profiles()[&a].successful_projects, 1);
        assert_eq!(d.profiles()[&b].successful_projects, 1);
        assert_eq!(d.partners()[&(a, b)].learned_at, 12);
        assert!(matches!(
            d.history().back().unwrap().cause,
            Cause::Project {
                evidence: ProjectEvidence::Conversation,
                ..
            }
        ));
        let snapshot = d.clone();
        w.advance_obligations();
        assert_eq!(w.development().unwrap(), &snapshot);
        assert_eq!(w.development().unwrap().known().len(), 0);
    }

    #[test]
    fn c56_disruption_creates_bounded_directional_caution_not_remote_blame() {
        use crate::living::development::{Cause, PARTNER_YIELD};
        use crate::living::obligations::ResultKind;
        let (mut w, a, b, plans) = fixture_for(World::development_campaign(0), Route::Qinghe);
        w.commit_cooperative_dispatches(&plans);
        let initial = w.social.relationship(a, b);
        let hub = w.npcs[&a].position;
        w.tick += 1;
        // Controlled interruption: return one item to actual market stock.
        w.npcs.get_mut(&b).unwrap().herbs -= 1;
        w.region.as_mut().unwrap().market_herbs += 1;
        w.npcs.get_mut(&b).unwrap().position = w.scenario.shrine;
        w.reconcile_cooperation();
        assert!(!w.development_can_partner(b, a));
        assert!(w.development_can_partner(a, b));
        assert_eq!(w.social.relationship(a, b), initial);
        assert_eq!(
            w.development().unwrap().profiles()[&a].disrupted_projects,
            0
        );
        w.advance_obligations();
        assert!(w.development_can_partner(a, b));
        w.tick += 1;
        let cell = DIRECTIONS
            .into_iter()
            .filter_map(|d| navigation::moved(hub, d))
            .find(|&p| w.walkable(p))
            .unwrap();
        w.npcs.get_mut(&b).unwrap().position = cell;
        w.advance_obligations();
        assert_eq!(w.social.relationship(a, b), initial);
        assert_eq!(
            w.development().unwrap().profiles()[&a].disrupted_projects,
            1
        );
        assert!(!w.development_can_partner(a, b));
        assert!(!w.can_offer_commitment(a, b));
        let d = w.development().unwrap();
        assert_eq!(d.partners()[&(b, a)].retry_at, 11 + PARTNER_YIELD);
        assert_eq!(d.partners()[&(a, b)].retry_at, 12 + PARTNER_YIELD);
        assert!(matches!(
            d.history().back().unwrap().cause,
            Cause::Project {
                result: ResultKind::ChangedCargo,
                ..
            }
        ));
        w.tick = 11 + PARTNER_YIELD;
        assert!(w.development_can_partner(b, a) && !w.development_can_partner(a, b));
        w.tick += 1;
        assert!(w.can_offer_commitment(a, b));
        assert_eq!(w.social.relationship(a, b), initial);
    }

    #[test]
    fn c56_carrier_loss_does_not_teach_a_dead_carrier_or_uninformed_partner() {
        let (mut w, a, b, plans) = fixture_for(World::development_campaign(0), Route::Qinghe);
        w.commit_cooperative_dispatches(&plans);
        let snapshot = w.development().unwrap().clone();
        w.tick += 1;
        w.npcs.get_mut(&b).unwrap().hp = 0;
        w.reconcile_cooperation();
        assert_eq!(w.development().unwrap(), &snapshot);
        assert!(w.development_can_partner(a, b));
    }

    #[test]
    fn c55_real_project_has_causal_chain_and_evidence_only_projection() {
        use crate::living::{communication::Kind, obligations::ResultKind};
        let (mut w, a, b, plans) = fixture_for(World::narrative_campaign(0), Route::Qinghe);
        let stock = w.supply.shrine;
        w.commit_cooperative_dispatches(&plans);
        let t = w.cooperation().unwrap().active()[&b];
        let n = w.narrative().unwrap();
        assert_eq!(n.active_projects()[&t.id].team, t);
        assert!(n.known_projects()[&t.id].dispatch.is_some());
        assert!(n.known_projects()[&t.id].delivery.is_none());
        let known = n.known_projects().clone();
        w.tick += 1;
        w.npcs.get_mut(&b).unwrap().position = w.scenario.shrine;
        // Player remains at the distant dispatch hub, not at delivery.
        assert!(!w.player_sees(w.scenario.shrine));
        w.unload_regional_cargo(b, false);
        assert_eq!(w.supply.shrine, stock + 2);
        assert_eq!(
            w.narrative().unwrap().active_projects()[&t.id]
                .outcome
                .unwrap()
                .result,
            ResultKind::Fulfilled
        );
        assert_eq!(w.narrative().unwrap().known_projects(), &known);
        w.tick += 1;
        let near = w.npcs[&a].position;
        let cell = DIRECTIONS
            .into_iter()
            .filter_map(|d| navigation::moved(near, d))
            .find(|&p| w.walkable(p))
            .unwrap();
        w.npcs.get_mut(&b).unwrap().position = cell;
        // Private conversation closes the world arc without disclosing it.
        w.player.position = w.scenario.shrine;
        assert!(!w.player_sees(near));
        w.advance_obligations();
        let n = w.narrative().unwrap();
        assert_eq!(n.counts(), (1, 1, 0, 0, 0));
        assert!(n.active_projects().is_empty());
        let arc = n.project_history().back().unwrap();
        assert_eq!(arc.team, t);
        assert_eq!(arc.outcome.unwrap().at, 11);
        assert_eq!(arc.end.unwrap().at, 12);
        assert_eq!(arc.end.unwrap().kind, Kind::Direct);
        assert_eq!(n.career(b).delivered, 1);
        assert_eq!(n.career(a).confirmed, 1);
        assert_eq!(n.known_projects(), &known);
        // Reading the actual posted manifest learns delivery, not the
        // private acknowledgement or relationship change.
        w.observe_cooperation_receipts();
        let k = w.narrative().unwrap().known_projects()[&t.id];
        let d = k.delivery.unwrap();
        assert_eq!(
            (d.at, d.learned_at, d.shipment, d.evidence),
            (11, 12, 2, Evidence::Register)
        );
        assert!(k.report.is_none() && k.handoff.is_none());
        let snapshot = w.clone();
        w.observe_cooperation_receipts();
        w.advance_obligations();
        assert_eq!(w, snapshot);
    }

    #[test]
    fn c55_loss_is_not_a_public_report() {
        use crate::living::{communication::Kind, obligations::ResultKind};
        let (mut w, _a, b, plans) = fixture_for(World::narrative_campaign(0), Route::Qinghe);
        w.commit_cooperative_dispatches(&plans);
        let id = w.cooperation().unwrap().active()[&b].id;
        let known = w.narrative().unwrap().known_projects().clone();
        w.tick += 1;
        w.npcs.get_mut(&b).unwrap().hp = 0;
        w.reconcile_cooperation();
        let n = w.narrative().unwrap();
        let p = n.project_history().back().unwrap();
        assert_eq!(p.team.id, id);
        assert_eq!(p.outcome.unwrap().result, ResultKind::CarrierLost);
        let end = p.end.unwrap();
        assert_eq!(end.kind, Kind::Unreported);
        assert!(end.place.is_none() && end.holder.is_none());
        assert!(n.active_projects().is_empty());
        assert_eq!(n.known_projects(), &known);
        assert_eq!(n.career(b).unreported, 1);
        assert!(w.communication().unwrap().messages().is_empty());
    }

    #[test]
    fn c53_real_delivery_creates_one_report_and_search_uses_only_known_sites() {
        use crate::living::communication::{Kind, SEARCH_BUDGET};
        use crate::living::navigation::distance;
        let (mut w, a, b, plans) = fixture_for(World::communication_campaign(0), Route::Qinghe);
        let supply = w.supply.shrine;
        w.commit_cooperative_dispatches(&plans);
        assert!(w.communication().unwrap().messages().is_empty());
        w.tick += 1;
        w.npcs.get_mut(&b).unwrap().position = w.scenario.shrine;
        w.unload_regional_cargo(b, false);
        assert_eq!(w.supply.shrine, supply + 2);
        assert_eq!(w.communication().unwrap().counts(), (1, 0, 0, 0));
        let m = w.communication().unwrap().messages()[&b];
        assert_eq!((m.origin, m.recipient, m.holder), (b, a, b));
        assert_eq!(
            w.message_plan(&w.npcs[&b]),
            Some((45, Goal::CarryMessage(m.project)))
        );
        assert!(matches!(w.choose_goal(&w.npcs[&b]), Goal::CarryMessage(_)));
        let before = w.npcs[&b].position;
        w.traffic.as_mut().unwrap().begin_tick();
        w.work_message(b, m.project);
        assert_eq!(distance(before, w.npcs[&b].position), 1);
        assert_eq!(w.communication().unwrap().messages()[&b].effort, 1);
        assert_eq!(w.supply.shrine, supply + 2);
        w.npcs.get_mut(&b).unwrap().hp -= 1;
        assert!(w.message_plan(&w.npcs[&b]).is_none());
        w.npcs.get_mut(&b).unwrap().hp += 1;
        let threshold = w.npcs[&b].definition.rest_threshold;
        w.npcs.get_mut(&b).unwrap().fatigue = threshold;
        assert!(w.message_plan(&w.npcs[&b]).is_none());
        assert!(w.message_camp(&w.npcs[&b]));
        w.npcs.get_mut(&b).unwrap().fatigue = 0;
        let near = w.npcs[&a].position;
        let cell = DIRECTIONS
            .into_iter()
            .filter_map(|d| navigation::moved(near, d))
            .find(|&p| w.walkable(p))
            .unwrap();
        w.npcs.get_mut(&b).unwrap().position = cell;
        w.player.position = near;
        let initial = w.social.relationship(a, b).regard;
        w.advance_obligations();
        assert_eq!(w.communication().unwrap().counts(), (1, 1, 0, 0));
        assert_eq!(
            w.communication().unwrap().history().back().unwrap().kind,
            Kind::Direct
        );
        assert_eq!(w.witnessed_messages().count(), 1);
        assert_eq!(w.social.relationship(a, b).regard, (initial + 1).min(8));
        assert!(w.communication().unwrap().messages().is_empty());
        assert!(w.message_plan(&w.npcs[&b]).is_none());
        let snapshot = w.clone();
        w.advance_obligations();
        w.delegate_messages();
        assert_eq!(w, snapshot);
        assert!(m.effort < SEARCH_BUDGET);
    }

    #[test]
    fn c53_natural_reports_reach_partners_with_physical_effort_and_replay() {
        let mut delivered = 0;
        for seed in [0, 3, 7, 42] {
            let mut w = World::communication_campaign(seed);
            let mut twin = w.clone();
            let mut effort = 0u32;
            for _ in 0..2000 {
                assert_eq!(
                    w.step(w.tick(), Intent::Wait),
                    twin.step(twin.tick(), Intent::Wait)
                );
                if w.npcs()
                    .any(|n| matches!(n.activity.goal, Goal::CarryMessage(_)))
                {
                    effort += 1;
                }
            }
            assert_eq!(w, twin);
            assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            let c = w.communication().unwrap();
            let (created, done, expired, handoffs) = c.counts();
            assert_eq!(created, done + expired + c.messages().len() as u32);
            assert_eq!(done, w.obligations().unwrap().counts().2);
            delivered += done;
            println!(
                "seed {seed}: messages={:?} goal_ticks={effort} events={:?}",
                c.counts(),
                c.history()
            );
            if created > 0 {
                assert!(effort > 0);
            }
            assert!(handoffs <= created);
        }
        assert!(
            delivered >= 2,
            "active report delivery should improve the prior natural 1/3 baseline"
        );
    }

    #[test]
    fn c52_completion_requires_local_evidence_and_rewards_once_directionally() {
        use crate::living::obligations::{Evidence as Proof, ResultKind};
        for route in [Route::Qinghe, Route::Willowford] {
            let (mut w, a, b, plans) = fixture_for(World::obligations_campaign(0), route);
            let original = w.social.relationship(a, b).regard;
            let reverse = w.social.relationship(b, a);
            let home = w.npcs[&b].position;
            w.commit_cooperative_dispatches(&plans);
            assert_eq!(w.obligations().unwrap().counts(), (1, 0, 0, 0));
            assert_eq!(
                w.obligation_delivery_goal(&w.npcs[&b]),
                Some(match route {
                    Route::Qinghe => Goal::DeliverFreight,
                    Route::Willowford => Goal::DeliverFrontierFreight,
                })
            );
            let saved = w.npcs[&b];
            w.npcs.get_mut(&b).unwrap().hp -= 1;
            assert!(w.obligation_delivery_goal(&w.npcs[&b]).is_none());
            w.npcs.insert(b, saved);
            w.npcs.get_mut(&b).unwrap().fatigue = saved.definition.rest_threshold;
            assert!(w.obligation_delivery_goal(&w.npcs[&b]).is_none());
            w.npcs.insert(b, saved);
            w.tick += 1;
            w.npcs.get_mut(&b).unwrap().position = route.destination(&w);
            match route {
                Route::Qinghe => w.unload_regional_cargo(b, false),
                Route::Willowford => w.work_frontier(b, Goal::DeliverFrontierFreight),
            }
            assert_eq!(w.obligations().unwrap().counts(), (1, 1, 0, 0));
            assert_eq!(
                w.obligations().unwrap().pending()[&b].result,
                ResultKind::Fulfilled
            );
            assert!(!w.can_offer_commitment(a, b));
            let snapshot = w.clone();
            w.advance_obligations();
            assert_eq!(w, snapshot);
            assert_eq!(w.social.relationship(a, b).regard, original);
            let mut manifest = w.clone();
            manifest.npcs.get_mut(&a).unwrap().position = route.destination(&w);
            manifest.npcs.get_mut(&b).unwrap().position = home;
            manifest.advance_obligations();
            assert_eq!(manifest.obligations().unwrap().counts(), (1, 1, 1, 0));
            assert_eq!(manifest.witnessed_obligations().count(), 0);
            let crate::living::obligations::Event::Acknowledged(ack) =
                manifest.obligations().unwrap().history().back().unwrap()
            else {
                panic!("missing manifest acknowledgement")
            };
            assert_eq!(ack.evidence, Proof::Register);
            w.npcs.get_mut(&b).unwrap().position = home;
            w.player.position = route.hub();
            w.tick += 1;
            w.advance_obligations();
            assert_eq!(w.social.relationship(a, b).regard, (original + 1).min(8));
            assert_eq!(w.social.relationship(b, a), reverse);
            assert_eq!(w.obligations().unwrap().reliability(a, b).fulfilled, 1);
            assert_eq!(w.obligations().unwrap().counts(), (1, 1, 1, 0));
            assert!(w.can_offer_commitment(a, b));
            assert_eq!(
                w.witnessed_obligations().last().unwrap().evidence,
                Proof::Conversation
            );
            let snapshot = w.clone();
            w.advance_obligations();
            assert_eq!(w, snapshot);
        }
    }

    #[test]
    fn c52_unexcused_release_cools_retry_but_disruptions_never_create_blame() {
        use crate::living::obligations::{REPORT_LIFETIME, RETRY_INTERVAL, ResultKind};
        for outcome in [
            Outcome::Withdrawn,
            Outcome::CargoChanged,
            Outcome::Expired,
            Outcome::CarrierLost,
        ] {
            let (mut w, a, b, plans) = fixture_for(World::obligations_campaign(0), Route::Qinghe);
            w.social.link(a, b);
            let initial = w.social.relationship(a, b).regard;
            let near = w.npcs[&b].position;
            w.commit_cooperative_dispatches(&plans);
            let team = w.cooperation().unwrap().active()[&b];
            w.npcs.get_mut(&b).unwrap().position = w.scenario.shrine;
            w.tick += 1;
            match outcome {
                Outcome::Withdrawn => {
                    w.company.as_mut().unwrap().journey = Some(crate::living::Journey {
                        actor: b,
                        seen_at: w.tick,
                        last_seen: w.player.position,
                    });
                }
                Outcome::CargoChanged => {
                    w.npcs.get_mut(&b).unwrap().herbs -= 1;
                    w.player.herbs += 1;
                }
                Outcome::Expired => w.tick = team.deadline,
                Outcome::CarrierLost => w.npcs.get_mut(&b).unwrap().hp = 0,
                _ => unreachable!(),
            }
            w.reconcile_cooperation();
            w.advance_obligations();
            assert_eq!(w.social.relationship(a, b).regard, initial);
            assert_eq!(w.obligations().unwrap().counts().1, 1);
            if outcome == Outcome::CarrierLost {
                assert_eq!(w.obligations().unwrap().counts(), (1, 1, 0, 1));
                continue;
            }
            let mut missed = w.clone();
            missed.tick += REPORT_LIFETIME;
            missed.advance_obligations();
            assert_eq!(missed.obligations().unwrap().counts(), (1, 1, 0, 1));
            assert_eq!(missed.social.relationship(a, b).regard, initial);
            w.npcs.get_mut(&b).unwrap().position = near;
            w.player.position = Route::Qinghe.hub();
            w.advance_obligations();
            let r = w.obligations().unwrap().reliability(a, b);
            assert_eq!(w.obligations().unwrap().counts(), (1, 1, 1, 0));
            if outcome == Outcome::Withdrawn {
                assert_eq!(w.social.relationship(a, b).regard, (initial - 1).max(0));
                assert_eq!(r.released, 1);
                assert!(!w.can_offer_commitment(a, b));
                assert_eq!(
                    w.witnessed_obligations().last().unwrap().result,
                    ResultKind::Released
                );
                w.tick += RETRY_INTERVAL;
                assert!(w.can_offer_commitment(a, b));
            } else {
                assert_eq!(w.social.relationship(a, b).regard, initial);
                assert_eq!(r.disrupted + r.expired, 1);
                assert!(w.can_offer_commitment(a, b));
            }
        }
    }

    #[test]
    fn c52_natural_commitment_outcomes_and_local_acknowledgement_replay() {
        let mut opened = 0;
        let mut acknowledged = 0;
        for seed in [0, 3, 7, 42] {
            let mut w = World::obligations_campaign(seed);
            let mut twin = w.clone();
            for _ in 0..2000 {
                assert_eq!(
                    w.step(w.tick(), Intent::Wait),
                    twin.step(twin.tick(), Intent::Wait)
                );
            }
            assert_eq!(w, twin);
            assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            let o = w.obligations().unwrap();
            let (start, closed, ack, unreported) = o.counts();
            assert_eq!(
                start,
                closed + w.cooperation().unwrap().active().len() as u32
            );
            assert_eq!(closed, ack + unreported + o.pending().len() as u32);
            opened += start;
            acknowledged += ack;
            println!(
                "seed {seed}: {:?}, pending {}",
                o.counts(),
                o.pending().len()
            );
        }
        assert!(
            opened > 0 && acknowledged > 0,
            "need natural acknowledgement, not only fixtures"
        );
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
