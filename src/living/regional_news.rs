//! C4.2: situated knowledge and bounded regional history. No world-state
//! lookup is used as a remote stock report. The canonical economy is separate
//! from each actor's dated, potentially obsolete knowledge of that economy.
use super::{
    ActorId, Goal, Npc, PLAYER, Position, Region, Role, World, content, expansion,
    navigation::in_reach,
};
use std::collections::{BTreeMap, VecDeque};

pub const REVISION: u16 = 29;
pub const STOCK_LIFETIME: u64 = 120;
pub const EVENT_NEWS_LIFETIME: u64 = 640;
pub const REPORT_DELAY: u64 = 2;
pub const MAX_HOPS: u8 = 3;
pub const EVENT_CAP: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum Site {
    QingheShrine,
    WillowfordMarket,
    PinefordDepot,
    PinefordGrove,
}
impl Site {
    pub const ALL: [Self; 4] = [
        Self::QingheShrine,
        Self::WillowfordMarket,
        Self::PinefordDepot,
        Self::PinefordGrove,
    ];
    pub fn position(self, world: &World) -> Position {
        match self {
            Self::QingheShrine => world.scenario().shrine,
            Self::WillowfordMarket => Region::sites().market,
            Self::PinefordDepot => expansion::DEPOT,
            Self::PinefordGrove => expansion::GROVE,
        }
    }
    fn stock(self, w: &World) -> u32 {
        match self {
            Self::QingheShrine => w.supply.shrine,
            Self::WillowfordMarket => w.region().unwrap().market_herbs,
            Self::PinefordDepot => w.frontier().unwrap().depot_herbs,
            Self::PinefordGrove => w.frontier().unwrap().grove_herbs,
        }
    }
    fn scarce(self, w: &World) -> bool {
        let stock = self.stock(w);
        match self {
            Self::QingheShrine => stock < w.scenario().shrine_target,
            Self::WillowfordMarket => stock <= content::MARKET_RESERVE,
            Self::PinefordDepot => stock <= expansion::DEPOT_RESERVE,
            Self::PinefordGrove => stock == 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum EventKind {
    Shortage,
    Recovered,
    Harvested,
    DepotStocked,
    FreightLoaded,
    FreightArrived,
    Bought,
    Sold,
    MarketStocked,
    ShrineContributed,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionalEvent {
    pub id: u32,
    pub at: u64,
    pub site: Site,
    pub kind: EventKind,
    pub actor: Option<ActorId>,
    pub amount: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub origin: ActorId,
    pub via: Option<ActorId>,
    pub hops: u8,
    pub observed_at: u64,
    /// Observation phase within a tick: before or after NPC commitments.
    pub phase: u8,
    pub acquired_at: u64,
}
impl Receipt {
    fn direct(actor: ActorId, tick: u64, phase: u8) -> Self {
        Self {
            origin: actor,
            via: None,
            hops: 0,
            observed_at: tick,
            phase,
            acquired_at: tick,
        }
    }
    fn forwarded(self, sender: ActorId, due: u64) -> Self {
        Self {
            via: Some(sender),
            hops: self.hops + 1,
            acquired_at: due,
            ..self
        }
    }
    fn better_than(self, other: Self) -> bool {
        (self.observed_at, self.phase, std::cmp::Reverse(self.hops))
            > (
                other.observed_at,
                other.phase,
                std::cmp::Reverse(other.hops),
            )
    }
    pub fn fresh(self, now: u64, lifetime: u64) -> bool {
        self.observed_at <= now && now - self.observed_at <= lifetime
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StockKnowledge {
    pub amount: u32,
    pub receipt: Receipt,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingStock {
    knowledge: StockKnowledge,
    due: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingEvent {
    event: u32,
    receipt: Receipt,
    due: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionalNews {
    next_event: u32,
    bands: [bool; 4],
    events: VecDeque<RegionalEvent>,
    stocks: BTreeMap<(ActorId, Site), StockKnowledge>,
    known: BTreeMap<(ActorId, u32), Receipt>,
    pending_stocks: BTreeMap<(ActorId, Site), PendingStock>,
    pending_events: BTreeMap<ActorId, PendingEvent>,
}
impl RegionalNews {
    fn new(bands: [bool; 4]) -> Self {
        Self {
            next_event: 0,
            bands,
            events: VecDeque::new(),
            stocks: BTreeMap::new(),
            known: BTreeMap::new(),
            pending_stocks: BTreeMap::new(),
            pending_events: BTreeMap::new(),
        }
    }
    pub fn events(&self) -> &VecDeque<RegionalEvent> {
        &self.events
    }
    pub fn next_event(&self) -> u32 {
        self.next_event
    }
    pub fn stock(&self, actor: ActorId, site: Site) -> Option<StockKnowledge> {
        self.stocks.get(&(actor, site)).copied()
    }
    pub fn knowledge(&self, actor: ActorId, id: u32) -> Option<Receipt> {
        self.known.get(&(actor, id)).copied()
    }
    pub fn known_events(
        &self,
        actor: ActorId,
    ) -> impl DoubleEndedIterator<Item = (&RegionalEvent, Receipt)> + '_ {
        self.events
            .iter()
            .filter_map(move |event| self.knowledge(actor, event.id).map(|r| (event, r)))
    }
    pub fn pending_count(&self) -> usize {
        self.pending_stocks.len() + self.pending_events.len()
    }
    pub fn receipt_count(&self) -> usize {
        self.known.len()
    }
    fn fresh_stock(&self, actor: ActorId, site: Site, tick: u64) -> Option<u32> {
        self.stock(actor, site)
            .filter(|k| k.receipt.fresh(tick, STOCK_LIFETIME))
            .map(|k| k.amount)
    }
    fn insert_stock(&mut self, actor: ActorId, site: Site, k: StockKnowledge) {
        let key = (actor, site);
        if self
            .stocks
            .get(&key)
            .is_none_or(|old| k.receipt.better_than(old.receipt) || k.receipt == old.receipt)
        {
            self.stocks.insert(key, k);
        }
        if self
            .pending_stocks
            .get(&key)
            .is_some_and(|p| !p.knowledge.receipt.better_than(self.stocks[&key].receipt))
        {
            self.pending_stocks.remove(&key);
        }
    }
    fn append(&mut self, mut event: RegionalEvent, observers: &[ActorId]) {
        if self.events.len() == EVENT_CAP {
            let retired = self.events.pop_front().unwrap().id;
            self.known.retain(|&(_, id), _| id != retired);
            self.pending_events.retain(|_, p| p.event != retired);
        }
        event.id = self.next_event;
        // At most a fixed number of events per committed turn (65,536 turns).
        self.next_event = self
            .next_event
            .checked_add(1)
            .expect("bounded event identity");
        for &actor in observers {
            self.known
                .insert((actor, event.id), Receipt::direct(actor, event.at, 0));
        }
        self.events.push_back(event);
    }
    /// Canonical independent wire encoding; map iteration and tags are fixed.
    /// This payload is hashed inside the revision-29 world state, not loaded
    /// from untrusted serialized state. Restore reconstructs it by replay.
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        fn u32b(out: &mut Vec<u8>, v: u32) {
            out.extend(v.to_le_bytes());
        }
        fn u64b(out: &mut Vec<u8>, v: u64) {
            out.extend(v.to_le_bytes());
        }
        fn actor(out: &mut Vec<u8>, a: ActorId) {
            u32b(out, a.get());
        }
        fn optional(out: &mut Vec<u8>, a: Option<ActorId>) {
            out.push(u8::from(a.is_some()));
            if let Some(a) = a {
                actor(out, a);
            }
        }
        fn receipt(out: &mut Vec<u8>, r: Receipt) {
            actor(out, r.origin);
            optional(out, r.via);
            out.push(r.hops);
            u64b(out, r.observed_at);
            out.push(r.phase);
            u64b(out, r.acquired_at);
        }
        fn stock(out: &mut Vec<u8>, k: StockKnowledge) {
            u32b(out, k.amount);
            receipt(out, k.receipt);
        }
        let mut out = b"SH:C42:NEWS:v1\0".to_vec();
        u32b(&mut out, self.next_event);
        out.extend(self.bands.map(u8::from));
        u32b(&mut out, self.events.len() as u32);
        for e in &self.events {
            u32b(&mut out, e.id);
            u64b(&mut out, e.at);
            out.extend([e.site as u8, e.kind as u8]);
            optional(&mut out, e.actor);
            u32b(&mut out, e.amount);
        }
        u32b(&mut out, self.stocks.len() as u32);
        for (&(a, s), &k) in &self.stocks {
            actor(&mut out, a);
            out.push(s as u8);
            stock(&mut out, k);
        }
        u32b(&mut out, self.known.len() as u32);
        for (&(a, id), &r) in &self.known {
            actor(&mut out, a);
            u32b(&mut out, id);
            receipt(&mut out, r);
        }
        u32b(&mut out, self.pending_stocks.len() as u32);
        for (&(a, s), &p) in &self.pending_stocks {
            actor(&mut out, a);
            out.push(s as u8);
            stock(&mut out, p.knowledge);
            u64b(&mut out, p.due);
        }
        u32b(&mut out, self.pending_events.len() as u32);
        for (&a, &p) in &self.pending_events {
            actor(&mut out, a);
            u32b(&mut out, p.event);
            receipt(&mut out, p.receipt);
            u64b(&mut out, p.due);
        }
        out
    }
}

impl World {
    pub fn regional_news(&self) -> Option<&RegionalNews> {
        self.regional_news.as_ref()
    }
    fn regional_position(&self, actor: ActorId) -> Option<Position> {
        if actor == PLAYER {
            self.player.alive().then_some(self.player.position)
        } else {
            self.npcs
                .get(&actor)
                .filter(|n| n.alive())
                .map(|n| n.position)
        }
    }
    fn regional_observers(&self, site: Site, actor: Option<ActorId>) -> Vec<ActorId> {
        let position = site.position(self);
        let source = actor.and_then(|id| self.regional_position(id));
        std::iter::once(PLAYER)
            .chain(self.npcs.keys().copied())
            .filter(|&id| {
                self.regional_position(id).is_some_and(|at| {
                    in_reach(at, position)
                        && self.can_see(at, position)
                        && source.is_none_or(|p| self.perceives(at, p))
                })
            })
            .collect()
    }
    pub(super) fn init_regional_news(&mut self) {
        let bands = Site::ALL.map(|site| site.scarce(self));
        self.regional_news = Some(RegionalNews::new(bands));
        self.observe_regional(0);
    }
    pub(super) fn record_regional_event(
        &mut self,
        site: Site,
        kind: EventKind,
        actor: Option<ActorId>,
        amount: u32,
    ) {
        if self.regional_news.is_none() {
            return;
        }
        let observers = self.regional_observers(site, actor);
        let event = RegionalEvent {
            id: 0,
            at: self.tick,
            site,
            kind,
            actor,
            amount,
        };
        self.regional_news
            .as_mut()
            .unwrap()
            .append(event, &observers);
    }
    /// Sampling barriers, not an omniscient actor notification: the global
    /// chronicle records stock-band changes, only present actors learn them.
    pub(super) fn observe_regional(&mut self, phase: u8) {
        if self.regional_news.is_none() {
            return;
        }
        for site in Site::ALL {
            let amount = site.stock(self);
            let scarce = site.scarce(self);
            if self.regional_news().unwrap().bands[site as usize] != scarce {
                self.regional_news.as_mut().unwrap().bands[site as usize] = scarce;
                self.record_regional_event(
                    site,
                    if scarce {
                        EventKind::Shortage
                    } else {
                        EventKind::Recovered
                    },
                    None,
                    amount,
                );
            }
            let observers = self.regional_observers(site, None);
            let tick = self.tick;
            for actor in observers {
                self.regional_news.as_mut().unwrap().insert_stock(
                    actor,
                    site,
                    StockKnowledge {
                        amount,
                        receipt: Receipt::direct(actor, tick, phase),
                    },
                );
            }
        }
    }
    fn regional_contact(&self, sender: ActorId, recipient: ActorId) -> bool {
        if sender == recipient {
            return false;
        }
        let (Some(a), Some(b)) = (
            self.regional_position(sender),
            self.regional_position(recipient),
        ) else {
            return false;
        };
        in_reach(a, b)
            && self.can_see(a, b)
            && (sender == PLAYER || self.npcs[&sender].definition.shares_news)
            && !self.social.relationship(sender, recipient).refuses_aid()
            && !self.social.relationship(recipient, sender).refuses_aid()
    }
    fn queue_regional_from(&mut self, sender: ActorId, recipient: ActorId) {
        if self.regional_news.is_none() || !self.regional_contact(sender, recipient) {
            return;
        }
        let tick = self.tick;
        let due = tick + REPORT_DELAY;
        let news = self.regional_news.as_mut().unwrap();
        for site in Site::ALL {
            let key = (recipient, site);
            if news.pending_stocks.contains_key(&key) {
                continue;
            }
            let Some(k) = news.stock(sender, site) else {
                continue;
            };
            if k.receipt.hops >= MAX_HOPS
                || k.receipt.origin == recipient
                || !k.receipt.fresh(tick, STOCK_LIFETIME)
                || news
                    .stock(recipient, site)
                    .is_some_and(|old| !k.receipt.better_than(old.receipt))
            {
                continue;
            }
            let knowledge = StockKnowledge {
                receipt: k.receipt.forwarded(sender, due),
                ..k
            };
            if news
                .stock(recipient, site)
                .is_some_and(|old| !knowledge.receipt.better_than(old.receipt))
            {
                continue;
            }
            news.pending_stocks
                .insert(key, PendingStock { knowledge, due });
        }
        if news.pending_events.contains_key(&recipient) {
            return;
        }
        let candidate = news.events.iter().rev().find_map(|event| {
            let r = news.knowledge(sender, event.id)?;
            (r.hops < MAX_HOPS
                && r.origin != recipient
                && r.fresh(tick, EVENT_NEWS_LIFETIME)
                && news.knowledge(recipient, event.id).is_none())
            .then_some((event.id, r))
        });
        if let Some((event, r)) = candidate {
            news.pending_events.insert(
                recipient,
                PendingEvent {
                    event,
                    receipt: r.forwarded(sender, due),
                    due,
                },
            );
        }
    }
    pub(super) fn regional_conversation(&mut self, id: ActorId) {
        self.queue_regional_from(id, PLAYER);
        self.queue_regional_from(PLAYER, id);
    }
    pub(super) fn queue_regional_reports(&mut self) {
        if self.regional_news.is_none() {
            return;
        }
        // Neighbor indexing avoids quadratic all-pairs contact scans. Actor
        // IDs still define ordering (same tie-break as the reference scan).
        let actors: Vec<_> = self
            .npcs
            .values()
            .filter(|n| n.alive())
            .map(|n| (n.id(), n.position))
            .collect();
        let occupied: BTreeMap<_, _> = actors.iter().map(|&(id, p)| (p, id)).collect();
        for (sender, position) in actors {
            let mut nearby: Vec<_> = super::DIRECTIONS
                .into_iter()
                .filter_map(|d| super::navigation::moved(position, d))
                .filter_map(|p| occupied.get(&p).copied())
                .collect();
            nearby.sort_unstable();
            for recipient in nearby {
                self.queue_regional_from(sender, recipient);
            }
        }
    }
    pub(super) fn deliver_regional_reports(&mut self) {
        let Some(news) = self.regional_news.as_ref() else {
            return;
        };
        let stocks: Vec<_> = news
            .pending_stocks
            .iter()
            .filter(|(_, p)| p.due <= self.tick)
            .map(|(&k, &p)| (k, p))
            .collect();
        let events: Vec<_> = news
            .pending_events
            .iter()
            .filter(|(_, p)| p.due <= self.tick)
            .map(|(&a, &p)| (a, p))
            .collect();
        for ((actor, site), p) in stocks {
            let alive = self.regional_position(actor).is_some();
            let news = self.regional_news.as_mut().unwrap();
            news.pending_stocks.remove(&(actor, site));
            if alive && p.knowledge.receipt.fresh(self.tick, STOCK_LIFETIME) {
                news.insert_stock(actor, site, p.knowledge);
            }
        }
        for (actor, p) in events {
            let alive = self.regional_position(actor).is_some();
            let news = self.regional_news.as_mut().unwrap();
            news.pending_events.remove(&actor);
            if alive
                && p.receipt.fresh(self.tick, EVENT_NEWS_LIFETIME)
                && news.events.iter().any(|e| e.id == p.event)
            {
                news.known.entry((actor, p.event)).or_insert(p.receipt);
            }
        }
    }
    pub(super) fn known_stock(&self, actor: ActorId, site: Site) -> Option<u32> {
        self.regional_news()?.fresh_stock(actor, site, self.tick)
    }
    /// New frontier roles plan exclusively from their own dated stock receipts.
    /// Physical inventory is revalidated at the work site on commit.
    pub(super) fn situated_frontier_duty(&self, npc: &Npc) -> Goal {
        let stock = |site| self.known_stock(npc.id(), site);
        match npc.definition.role {
            Role::FrontierCarrier if npc.herbs > 0 => Goal::DeliverFrontierFreight,
            Role::FrontierCarrier => match stock(Site::WillowfordMarket) {
                None => Goal::SurveyWillowford,
                Some(n) if n < content::MARKET_TARGET + expansion::WILLOWFORD_BUFFER => {
                    match stock(Site::PinefordDepot) {
                        Some(n) if n <= expansion::DEPOT_RESERVE => Goal::Idle,
                        _ => Goal::CollectFrontierFreight,
                    }
                }
                _ => Goal::Idle,
            },
            Role::FrontierGrower if npc.herbs > 0 => Goal::StockFrontierDepot,
            Role::FrontierGrower => match stock(Site::PinefordDepot) {
                None => Goal::SurveyPinefordDepot,
                Some(n) if n < expansion::DEPOT_TARGET && stock(Site::PinefordGrove) != Some(0) => {
                    Goal::GatherFrontier
                }
                _ => Goal::Idle,
            },
            _ => Goal::Idle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::expansion::{CARRIER, GROWER};
    const COURIER: ActorId = ActorId::new(32);
    const WATCH: ActorId = ActorId::new(2);
    const MEDIC: ActorId = ActorId::new(1);
    fn at(w: &mut World, actor: ActorId, p: Position) {
        if actor == PLAYER {
            w.player.position = p;
        } else {
            w.npcs.get_mut(&actor).unwrap().position = p;
        }
    }
    fn contact(w: &mut World, actor: ActorId, recipient: ActorId) {
        at(w, actor, Position { x: 46, y: 7 });
        at(w, recipient, Position { x: 45, y: 7 });
    }
    fn see_market(w: &mut World, actor: ActorId, stock: u32) {
        w.region.as_mut().unwrap().market_herbs = stock;
        at(w, actor, Region::sites().market);
        w.observe_regional(0);
    }
    #[test]
    fn remote_stock_changes_cannot_drive_uninformed_frontier_plans_or_loads() {
        let mut w = World::regional_knowledge_campaign(7);
        assert!(w.known_stock(CARRIER, Site::WillowfordMarket).is_none());
        assert_eq!(w.frontier_duty(&w.npcs[&CARRIER]), Goal::SurveyWillowford);
        w.region.as_mut().unwrap().market_herbs = 0;
        assert_eq!(w.frontier_duty(&w.npcs[&CARRIER]), Goal::SurveyWillowford);
        let before = *w.frontier().unwrap();
        w.work_frontier(CARRIER, Goal::CollectFrontierFreight);
        assert_eq!(*w.frontier().unwrap(), before);
        // Acquire real direct evidence at the market, then depart.
        w.tick = 10;
        see_market(&mut w, CARRIER, 1);
        at(&mut w, CARRIER, expansion::DEPOT);
        let mut changed = w.clone();
        changed.region.as_mut().unwrap().market_herbs = 100;
        for candidate in [&mut w, &mut changed] {
            assert_eq!(
                candidate.frontier_duty(&candidate.npcs[&CARRIER]),
                Goal::CollectFrontierFreight
            );
            candidate.work_frontier(CARRIER, Goal::CollectFrontierFreight);
            assert_eq!(candidate.npcs[&CARRIER].herbs, 2);
            assert_eq!(
                candidate.frontier().unwrap().depot_herbs,
                before.depot_herbs - 2
            );
        }
        assert_eq!(w.regional_news(), changed.regional_news());
        // Stale memories do not authorize loading and trigger another survey.
        let mut stale = World::regional_knowledge_campaign(7);
        stale.tick = 10;
        see_market(&mut stale, CARRIER, 1);
        at(&mut stale, CARRIER, expansion::DEPOT);
        stale.tick = 131;
        let before = *stale.frontier().unwrap();
        assert_eq!(
            stale.frontier_duty(&stale.npcs[&CARRIER]),
            Goal::SurveyWillowford
        );
        stale.work_frontier(CARRIER, Goal::CollectFrontierFreight);
        assert_eq!(*stale.frontier().unwrap(), before);
        // The grower likewise cannot use a distant depot inventory.
        let mut grower = World::regional_knowledge_campaign(7);
        assert_eq!(
            grower.frontier_duty(&grower.npcs[&GROWER]),
            Goal::SurveyPinefordDepot
        );
        let before = *grower.frontier().unwrap();
        grower.work_frontier(GROWER, Goal::GatherFrontier);
        assert_eq!(*grower.frontier().unwrap(), before);
    }

    #[test]
    fn gossip_requires_contact_and_delivery_and_retains_origin_across_three_hops() {
        let mut w = World::regional_knowledge_campaign(0);
        w.tick = 10;
        see_market(&mut w, CARRIER, 1);
        let id = w.regional_news().unwrap().next_event() - 1;
        assert_eq!(
            w.regional_news().unwrap().events().back().unwrap().kind,
            EventKind::Shortage
        );
        assert!(w.regional_news().unwrap().knowledge(CARRIER, id).is_some());
        assert!(w.regional_news().unwrap().knowledge(GROWER, id).is_none());
        assert!(
            w.regional_news()
                .unwrap()
                .stock(GROWER, Site::WillowfordMarket)
                .is_none()
        );
        w.queue_regional_from(CARRIER, GROWER);
        assert_eq!(w.regional_news().unwrap().pending_count(), 0);
        for (i, (sender, recipient)) in [(CARRIER, GROWER), (GROWER, COURIER), (COURIER, PLAYER)]
            .into_iter()
            .enumerate()
        {
            contact(&mut w, sender, recipient);
            let sent = w.tick;
            w.queue_regional_from(sender, recipient);
            assert!(
                w.regional_news()
                    .unwrap()
                    .pending_events
                    .contains_key(&recipient)
            );
            w.tick = sent + 1;
            w.deliver_regional_reports();
            assert!(
                w.regional_news()
                    .unwrap()
                    .knowledge(recipient, id)
                    .is_none()
            );
            // Delivery is recollection of an actual conversation, not remote
            // contact at receipt time. Sender may already have departed.
            at(&mut w, sender, Position { x: 3, y: 4 });
            w.tick = sent + 2;
            w.deliver_regional_reports();
            let r = w.regional_news().unwrap().knowledge(recipient, id).unwrap();
            assert_eq!(
                (r.origin, r.via, r.hops, r.observed_at, r.acquired_at),
                (CARRIER, Some(sender), i as u8 + 1, 10, sent + 2)
            );
            let k = w
                .regional_news()
                .unwrap()
                .stock(recipient, Site::WillowfordMarket)
                .unwrap();
            assert_eq!(k.amount, 1);
            assert_eq!(
                (
                    k.receipt.origin,
                    k.receipt.via,
                    k.receipt.hops,
                    k.receipt.observed_at
                ),
                (CARRIER, Some(sender), i as u8 + 1, 10)
            );
        }
        contact(&mut w, PLAYER, WATCH);
        w.queue_regional_from(PLAYER, WATCH);
        assert!(
            !w.regional_news()
                .unwrap()
                .pending_events
                .contains_key(&WATCH)
        );
        assert!(
            !w.regional_news()
                .unwrap()
                .pending_stocks
                .contains_key(&(WATCH, Site::WillowfordMarket))
        );
        assert!(w.regional_news().unwrap().knowledge(WATCH, id).is_none());
    }

    #[test]
    fn direct_newer_observation_wins_and_dead_recipient_does_not_learn() {
        let mut w = World::regional_knowledge_campaign(7);
        w.tick = 5;
        see_market(&mut w, CARRIER, 1);
        contact(&mut w, CARRIER, GROWER);
        w.queue_regional_from(CARRIER, GROWER);
        assert!(
            w.regional_news()
                .unwrap()
                .pending_stocks
                .contains_key(&(GROWER, Site::WillowfordMarket))
        );
        w.tick = 6;
        see_market(&mut w, GROWER, 4);
        let direct = w
            .regional_news()
            .unwrap()
            .stock(GROWER, Site::WillowfordMarket)
            .unwrap();
        assert_eq!(direct.amount, 4);
        assert!(direct.receipt.via.is_none());
        assert!(
            !w.regional_news()
                .unwrap()
                .pending_stocks
                .contains_key(&(GROWER, Site::WillowfordMarket))
        );
        w.tick = 7;
        w.deliver_regional_reports();
        assert_eq!(
            w.regional_news()
                .unwrap()
                .stock(GROWER, Site::WillowfordMarket),
            Some(direct)
        );
        contact(&mut w, GROWER, COURIER);
        w.queue_regional_from(GROWER, COURIER);
        w.npcs.get_mut(&COURIER).unwrap().hp = 0;
        w.tick = 9;
        w.deliver_regional_reports();
        assert!(
            w.regional_news()
                .unwrap()
                .stock(COURIER, Site::WillowfordMarket)
                .is_none()
        );
        assert!(
            !w.regional_news()
                .unwrap()
                .pending_stocks
                .contains_key(&(COURIER, Site::WillowfordMarket))
        );
    }

    #[test]
    fn chronicle_never_reveals_unwitnessed_history_and_eviction_cleans_all_references() {
        let mut w = World::regional_knowledge_campaign(42);
        w.tick = 1;
        // No actor within reach of this location after moving the grower.
        at(&mut w, GROWER, expansion::DEPOT);
        w.record_regional_event(Site::PinefordGrove, EventKind::Shortage, None, 0);
        assert_eq!(w.regional_news().unwrap().next_event(), 1);
        assert_eq!(w.regional_news().unwrap().receipt_count(), 0);
        // Arriving later allows a stock observation, never retroactive witness.
        at(&mut w, PLAYER, expansion::GROVE);
        w.tick = 2;
        w.observe_regional(0);
        assert!(
            w.regional_news()
                .unwrap()
                .stock(PLAYER, Site::PinefordGrove)
                .is_some()
        );
        assert!(w.regional_news().unwrap().knowledge(PLAYER, 0).is_none());
        // A witness learns a new event at the actual site.
        w.record_regional_event(Site::PinefordGrove, EventKind::Harvested, Some(PLAYER), 1);
        assert!(w.regional_news().unwrap().knowledge(PLAYER, 1).is_some());
        contact(&mut w, PLAYER, MEDIC);
        w.queue_regional_from(PLAYER, MEDIC);
        assert_eq!(w.regional_news().unwrap().pending_events[&MEDIC].event, 1);
        for _ in 0..EVENT_CAP + 2 {
            w.record_regional_event(Site::PinefordGrove, EventKind::Harvested, None, 1);
        }
        let news = w.regional_news().unwrap();
        assert_eq!(news.events().len(), EVENT_CAP);
        assert!(news.events().front().unwrap().id > 1);
        assert!(news.knowledge(PLAYER, 1).is_none());
        assert!(!news.pending_events.contains_key(&MEDIC));
        assert_eq!(news.events().back().unwrap().id + 1, news.next_event());
    }

    #[test]
    fn expired_or_hostile_contacts_do_not_enqueue_reports() {
        let mut w = World::regional_knowledge_campaign(0);
        w.tick = 1;
        see_market(&mut w, CARRIER, 0);
        let id = w.regional_news().unwrap().next_event() - 1;
        contact(&mut w, CARRIER, GROWER);
        w.tick = EVENT_NEWS_LIFETIME + 2;
        w.queue_regional_from(CARRIER, GROWER);
        assert_eq!(w.regional_news().unwrap().pending_count(), 0);
        // Refresh with an observation but establish hostility via the real
        // violence system, not a direct edit to the private relation ledger.
        w.tick = EVENT_NEWS_LIFETIME + 3;
        see_market(&mut w, CARRIER, 0);
        contact(&mut w, CARRIER, PLAYER);
        w.damage(PLAYER, CARRIER, 1, false);
        assert!(w.social.relationship(CARRIER, PLAYER).refuses_aid());
        w.regional_conversation(CARRIER);
        assert!(
            w.regional_news()
                .unwrap()
                .stock(PLAYER, Site::WillowfordMarket)
                .is_none()
        );
        assert!(
            !w.regional_news()
                .unwrap()
                .pending_events
                .contains_key(&PLAYER)
        );
        assert!(w.regional_news().unwrap().knowledge(GROWER, id).is_none());
    }

    #[test]
    fn spatial_contact_index_matches_ordered_pair_reference() {
        let mut w = World::regional_knowledge_campaign(7);
        for _ in 0..180 {
            w.step(w.tick(), crate::living::Intent::Wait).unwrap();
        }
        for _ in 0..100 {
            let mut fast = w.clone();
            let mut reference = w.clone();
            for world in [&mut fast, &mut reference] {
                let news = world.regional_news.as_mut().unwrap();
                news.pending_stocks.clear();
                news.pending_events.clear();
            }
            fast.queue_regional_reports();
            let ids: Vec<_> = reference
                .npcs
                .values()
                .filter(|n| n.alive())
                .map(Npc::id)
                .collect();
            for &a in &ids {
                for &b in &ids {
                    reference.queue_regional_from(a, b);
                }
            }
            assert_eq!(fast.regional_news(), reference.regional_news());
            w.step(w.tick(), crate::living::Intent::Wait).unwrap();
        }
    }

    #[test]
    fn expiring_report_is_not_delivered_and_pending_state_is_hashed() {
        let mut w = World::regional_knowledge_campaign(7);
        w.tick = 1;
        see_market(&mut w, CARRIER, 1);
        contact(&mut w, CARRIER, GROWER);
        w.tick = 1 + STOCK_LIFETIME;
        let before = w.state_digest();
        w.queue_regional_from(CARRIER, GROWER);
        assert_ne!(w.state_digest(), before);
        assert!(
            w.regional_news()
                .unwrap()
                .pending_stocks
                .contains_key(&(GROWER, Site::WillowfordMarket))
        );
        w.tick += REPORT_DELAY;
        w.deliver_regional_reports();
        assert!(
            w.regional_news()
                .unwrap()
                .stock(GROWER, Site::WillowfordMarket)
                .is_none()
        );
        assert!(
            !w.regional_news()
                .unwrap()
                .pending_stocks
                .contains_key(&(GROWER, Site::WillowfordMarket))
        );
    }
}
