//! C5.4: competing dated accounts of regional resource availability.
//! A report is not a world fact. Only situated checks establish direct
//! knowledge; neither hearsay nor its credibility may authorize inventory.
use super::{ActorId, Goal, Npc, PLAYER, World, navigation::in_reach, regional_news::Site};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, VecDeque},
};
pub const REVISION: u16 = 41;
pub const VERSION: u16 = 1;
pub const LIFETIME: u64 = 192;
pub const MAX_HOPS: u8 = 3;
pub const MIN_CONFIDENCE: u8 = 15;
pub const EXCHANGE_INTERVAL: u64 = 16;
pub const REFRESH_INTERVAL: u64 = 24;
pub const VERIFY_BUDGET: u8 = 48;
pub const VERIFY_YIELD: u64 = 48;
pub const HISTORY_CAP: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sample {
    pub site: Site,
    pub scarce: bool,
    pub origin: ActorId,
    pub observed_at: u64,
    pub phase: u8,
}
impl Sample {
    fn stamp(self) -> (u64, u8) {
        (self.observed_at, self.phase)
    }
    pub fn fresh(self, now: u64) -> bool {
        self.observed_at <= now && now - self.observed_at < LIFETIME
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Account {
    pub sample: Sample,
    pub via: Option<ActorId>,
    pub hops: u8,
    pub acquired_at: u64,
    pub credibility: u8,
}
impl Account {
    fn direct(sample: Sample) -> Self {
        Self {
            sample,
            via: None,
            hops: 0,
            acquired_at: sample.observed_at,
            credibility: 100,
        }
    }
    pub fn confidence(self, now: u64) -> u8 {
        self.credibility
            .saturating_sub((now.saturating_sub(self.sample.observed_at) / 4).min(100) as u8)
    }
    pub fn fresh(self, now: u64) -> bool {
        self.sample.fresh(now) && self.confidence(now) >= MIN_CONFIDENCE
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Unknown,
    Reported,
    Contested,
    Verified,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct View {
    pub status: Status,
    pub local: Option<Sample>,
    pub low: Option<Account>,
    pub adequate: Option<Account>,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Attempt {
    pub spent: u8,
    pub retry_at: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Kind {
    Heard,
    Contested,
    Verified,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Event {
    pub at: u64,
    pub recipient: ActorId,
    pub kind: Kind,
    pub account: Account,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Rumors {
    local: BTreeMap<(ActorId, Site), Sample>,
    reports: BTreeMap<(ActorId, Site, bool), Account>,
    last_exchange: BTreeMap<ActorId, u64>,
    attempts: BTreeMap<(ActorId, Site), Attempt>,
    history: VecDeque<Event>,
    heard: u32,
    contested: u32,
    verified: u32,
}
impl Rumors {
    pub fn reports(&self) -> &BTreeMap<(ActorId, Site, bool), Account> {
        &self.reports
    }
    pub fn local(&self) -> &BTreeMap<(ActorId, Site), Sample> {
        &self.local
    }
    pub fn attempts(&self) -> &BTreeMap<(ActorId, Site), Attempt> {
        &self.attempts
    }
    pub fn history(&self) -> &VecDeque<Event> {
        &self.history
    }
    pub fn counts(&self) -> (u32, u32, u32) {
        (self.heard, self.contested, self.verified)
    }
    pub fn view(&self, actor: ActorId, site: Site, now: u64) -> View {
        let local = self
            .local
            .get(&(actor, site))
            .copied()
            .filter(|s| s.fresh(now));
        let low = self
            .reports
            .get(&(actor, site, true))
            .copied()
            .filter(|a| a.fresh(now));
        let adequate = self
            .reports
            .get(&(actor, site, false))
            .copied()
            .filter(|a| a.fresh(now));
        let newest_report = low
            .into_iter()
            .chain(adequate)
            .map(|a| a.sample.stamp())
            .max();
        let status = if local.is_some_and(|s| newest_report.is_none_or(|r| s.stamp() >= r)) {
            Status::Verified
        } else if (low.is_some() || local.is_some_and(|s| s.scarce))
            && (adequate.is_some() || local.is_some_and(|s| !s.scarce))
        {
            Status::Contested
        } else if newest_report.is_some() {
            Status::Reported
        } else {
            Status::Unknown
        };
        View {
            status,
            local,
            low,
            adequate,
        }
    }
    fn event(&mut self, e: Event) {
        if self.history.len() == HISTORY_CAP {
            self.history.pop_front();
        }
        self.history.push_back(e);
    }
    fn observe(&mut self, actor: ActorId, s: Sample) {
        let before = self.view(actor, s.site, s.observed_at).status;
        self.local.insert((actor, s.site), s);
        self.attempts.remove(&(actor, s.site));
        if matches!(before, Status::Reported | Status::Contested) {
            self.verified += 1;
            self.event(Event {
                at: s.observed_at,
                recipient: actor,
                kind: Kind::Verified,
                account: Account::direct(s),
            });
        }
    }
    fn ready(&self, actor: ActorId, at: u64) -> bool {
        self.last_exchange
            .get(&actor)
            .is_none_or(|&t| at.saturating_sub(t) >= EXCHANGE_INTERVAL)
    }
    fn accept(&self, recipient: ActorId, a: Account, now: u64) -> bool {
        let s = a.sample;
        if !a.fresh(now) || s.origin == recipient {
            return false;
        }
        // A local observation newer than or equal to this account already
        // resolves the question. Never import a weaker old version.
        if self
            .local
            .get(&(recipient, s.site))
            .is_some_and(|old| old.stamp() >= s.stamp())
        {
            return false;
        }
        self.reports
            .get(&(recipient, s.site, s.scarce))
            .is_none_or(|old| {
                !old.fresh(now)
                    || s.observed_at >= old.sample.observed_at.saturating_add(REFRESH_INTERVAL)
                    || (s.stamp() == old.sample.stamp()
                        && a.hops < old.hops
                        && a.credibility >= old.credibility)
            })
    }
    fn receive(&mut self, recipient: ActorId, sender: ActorId, a: Account, now: u64) -> bool {
        if !self.accept(recipient, a, now) {
            return false;
        }
        let before = self.view(recipient, a.sample.site, now).status;
        self.reports
            .insert((recipient, a.sample.site, a.sample.scarce), a);
        self.last_exchange.insert(recipient, now);
        self.last_exchange.insert(sender, now);
        self.heard += 1;
        self.event(Event {
            at: now,
            recipient,
            kind: Kind::Heard,
            account: a,
        });
        if before != Status::Contested
            && self.view(recipient, a.sample.site, now).status == Status::Contested
        {
            self.contested += 1;
            self.event(Event {
                at: now,
                recipient,
                kind: Kind::Contested,
                account: a,
            });
        }
        true
    }
    fn prune(&mut self, now: u64) {
        self.local.retain(|_, s| s.fresh(now));
        self.reports.retain(|_, a| a.fresh(now));
        // Verification state is bounded per actor/site and removed when no
        // competing account remains relevant.
        let stale: Vec<_> = self
            .attempts
            .keys()
            .copied()
            .filter(|&(a, s)| self.view(a, s, now).status != Status::Contested)
            .collect();
        for k in stale {
            self.attempts.remove(&k);
        }
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        fn sample(out: &mut Vec<u8>, s: Sample) {
            out.extend([s.site as u8, u8::from(s.scarce)]);
            out.extend(s.origin.get().to_le_bytes());
            out.extend(s.observed_at.to_le_bytes());
            out.push(s.phase);
        }
        fn account(out: &mut Vec<u8>, a: Account) {
            sample(out, a.sample);
            out.push(u8::from(a.via.is_some()));
            if let Some(v) = a.via {
                out.extend(v.get().to_le_bytes());
            }
            out.extend([a.hops, a.credibility]);
            out.extend(a.acquired_at.to_le_bytes());
        }
        let mut out = b"SH:C54:RUMORS:v1\0".to_vec();
        out.extend(VERSION.to_le_bytes());
        for n in [
            self.heard,
            self.contested,
            self.verified,
            self.local.len() as u32,
        ] {
            out.extend(n.to_le_bytes());
        }
        for (&(id, _), &s) in &self.local {
            out.extend(id.get().to_le_bytes());
            sample(&mut out, s);
        }
        out.extend((self.reports.len() as u32).to_le_bytes());
        for (&(id, _, _), &a) in &self.reports {
            out.extend(id.get().to_le_bytes());
            account(&mut out, a);
        }
        out.extend((self.last_exchange.len() as u32).to_le_bytes());
        for (&id, &at) in &self.last_exchange {
            out.extend(id.get().to_le_bytes());
            out.extend(at.to_le_bytes());
        }
        out.extend((self.attempts.len() as u32).to_le_bytes());
        for (&(id, site), a) in &self.attempts {
            out.extend(id.get().to_le_bytes());
            out.extend([site as u8, a.spent]);
            out.extend(a.retry_at.to_le_bytes());
        }
        out.extend((self.history.len() as u32).to_le_bytes());
        for e in &self.history {
            out.extend(e.at.to_le_bytes());
            out.extend(e.recipient.get().to_le_bytes());
            out.push(e.kind as u8);
            account(&mut out, e.account);
        }
        out
    }
}
impl World {
    pub fn rumors(&self) -> Option<&Rumors> {
        self.rumors.as_ref()
    }
    pub fn known_rumor(&self, site: Site) -> Option<View> {
        Some(self.rumors()?.view(PLAYER, site, self.tick))
    }
    pub(super) fn observe_rumor(&mut self, actor: ActorId, site: Site, scarce: bool, phase: u8) {
        let sample = Sample {
            site,
            scarce,
            origin: actor,
            observed_at: self.tick,
            phase,
        };
        let uncertain = self.narrative.is_some()
            && self.rumors().is_some_and(|r| {
                matches!(
                    r.view(actor, site, self.tick).status,
                    Status::Reported | Status::Contested
                )
            });
        if let Some(r) = self.rumors.as_mut() {
            r.observe(actor, sample);
        }
        if uncertain {
            self.story_check(actor, sample);
        }
    }
    fn gossip_position(&self, a: ActorId) -> Option<super::Position> {
        if a == PLAYER {
            self.player.alive().then_some(self.player.position)
        } else {
            self.npcs
                .get(&a)
                .filter(|n| {
                    n.alive() && !matches!(n.activity.goal, Goal::Retreat | Goal::Retaliate)
                })
                .map(|n| n.position)
        }
    }
    fn rumor_offer(&self, sender: ActorId, recipient: ActorId, explicit: bool) -> Option<Account> {
        let r = self.rumors()?;
        if sender == recipient {
            return None;
        }
        let (a, b) = (
            self.gossip_position(sender)?,
            self.gossip_position(recipient)?,
        );
        let relation = self.social.relationship(recipient, sender);
        if !in_reach(a, b)
            || !self.can_see(a, b)
            || self.social.relationship(sender, recipient).refuses_aid()
            || relation.refuses_aid()
            || (!explicit && !relation.trusts_news())
            || (sender != PLAYER && !self.npcs[&sender].definition.shares_news)
        {
            return None;
        }
        let limit =
            (65i16 + 4 * i16::from(relation.regard) + 2 * i16::from(relation.familiarity.min(5)))
                .clamp(0, 100) as u8;
        let candidates = Site::ALL.into_iter().flat_map(|site| {
            let v = r.view(sender, site, self.tick);
            v.local.map(Account::direct).into_iter().chain(
                [v.low, v.adequate]
                    .into_iter()
                    .flatten()
                    .filter(move |a| v.local.is_none_or(|local| a.sample.stamp() > local.stamp())),
            )
        });
        candidates
            .filter_map(|old| {
                if old.hops >= MAX_HOPS
                    || old.sample.origin == recipient
                    || old.via == Some(recipient)
                    || !old.fresh(self.tick)
                {
                    return None;
                }
                let forwarded = Account {
                    via: Some(sender),
                    hops: old.hops + 1,
                    acquired_at: self.tick,
                    credibility: old.credibility.saturating_sub(15).min(limit),
                    ..old
                };
                r.accept(recipient, forwarded, self.tick)
                    .then_some(forwarded)
            })
            .max_by_key(|a| {
                let v = r.view(recipient, a.sample.site, self.tick);
                let opposing = if a.sample.scarce {
                    v.adequate.is_some() || v.local.is_some_and(|s| !s.scarce)
                } else {
                    v.low.is_some() || v.local.is_some_and(|s| s.scarce)
                };
                (
                    opposing,
                    a.sample.scarce,
                    a.sample.stamp(),
                    a.credibility,
                    Reverse(a.hops),
                    Reverse(a.sample.site),
                    Reverse(a.sample.origin),
                )
            })
    }
    pub(super) fn rumor_conversation(&mut self, id: ActorId) {
        if self.rumors.is_none() {
            return;
        }
        // Both offers are made from the pre-conversation snapshot.
        let a = self.rumor_offer(id, PLAYER, true);
        let b = self.rumor_offer(PLAYER, id, true);
        if let Some(a) = a
            && self
                .rumors
                .as_mut()
                .unwrap()
                .receive(PLAYER, id, a, self.tick)
        {
            self.story_account(PLAYER, a);
        }
        if let Some(b) = b
            && self
                .rumors
                .as_mut()
                .unwrap()
                .receive(id, PLAYER, b, self.tick)
        {
            self.story_account(id, b);
        }
    }
    pub(super) fn advance_rumors(&mut self) {
        if self.rumors.is_none() {
            return;
        }
        self.rumors.as_mut().unwrap().prune(self.tick);
        let local: BTreeMap<_, _> = self
            .npcs
            .values()
            .filter(|n| n.alive())
            .map(|n| (n.position, n.id()))
            .collect();
        let mut proposals = Vec::new();
        for n in self.npcs.values() {
            let a = n.id();
            if !self.rumors().unwrap().ready(a, self.tick) {
                continue;
            }
            for d in super::DIRECTIONS {
                let Some(b) = super::navigation::moved(n.position, d)
                    .and_then(|p| local.get(&p))
                    .copied()
                else {
                    continue;
                };
                if self.rumors().unwrap().ready(b, self.tick)
                    && let Some(report) = self.rumor_offer(a, b, false)
                {
                    proposals.push((a, b, report));
                }
            }
        }
        proposals
            .sort_by_key(|&(a, b, r)| (Reverse(r.sample.scarce), Reverse(r.sample.stamp()), a, b));
        let mut used = BTreeSet::new();
        for (a, b, report) in proposals {
            if used.contains(&a) || used.contains(&b) {
                continue;
            }
            if self
                .rumors
                .as_mut()
                .unwrap()
                .receive(b, a, report, self.tick)
            {
                used.insert(a);
                used.insert(b);
                self.story_account(b, report);
            }
        }
    }
    fn verification_eligible(&self, n: &Npc) -> bool {
        n.alive()
            && n.definition.shares_news
            && n.hp == n.definition.hp
            && n.herbs == 0
            && !self.injuries().unwrap().people[&n.id()].injured()
            && !self.hobbled(n.id())
            && self.companion(n.id()).is_none()
            && !self.sect_duty_active(n.id())
    }
    pub(super) fn rumor_goal(&self, n: &Npc, duty: Goal) -> Option<Goal> {
        let r = self.rumors()?;
        if !matches!(duty, Goal::Idle | Goal::Patrol)
            || !self.verification_eligible(n)
            || n.fatigue >= n.definition.rest_threshold
        {
            return None;
        }
        Site::ALL
            .into_iter()
            .find(|&s| {
                r.view(n.id(), s, self.tick).status == Status::Contested
                    && r.attempts
                        .get(&(n.id(), s))
                        .is_none_or(|a| self.tick >= a.retry_at)
            })
            .map(Goal::VerifyRumor)
    }
    pub(super) fn work_verification(&mut self, id: ActorId, site: Site) {
        let n = self.npcs[&id];
        if !self.verification_eligible(&n)
            || self.rumors().unwrap().view(id, site, self.tick).status != Status::Contested
        {
            return;
        }
        let target = site.position(self);
        self.travel(id, target, 1);
        let a = self
            .rumors
            .as_mut()
            .unwrap()
            .attempts
            .entry((id, site))
            .or_default();
        a.spent += 1;
        if a.spent >= VERIFY_BUDGET {
            a.spent = 0;
            a.retry_at = self.tick + VERIFY_YIELD;
        }
        // The normal post-movement observation barrier supplies proof.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Intent, Position, Role};
    const A: ActorId = ActorId::new(1);
    const B: ActorId = ActorId::new(2);
    const C: ActorId = ActorId::new(3);
    const D: ActorId = ActorId::new(31);
    const S: Site = Site::WillowfordMarket;
    fn empty() -> World {
        let mut w = World::rumor_campaign(3);
        w.rumors = Some(Rumors::default());
        w.tick = 10;
        w
    }
    fn place(w: &mut World, a: ActorId, p: Position) {
        if a == PLAYER {
            w.player.position = p;
        } else {
            let n = w.npcs.get_mut(&a).unwrap();
            n.position = p;
            n.activity.goal = Goal::Idle;
            n.fatigue = 0;
        }
    }
    fn row(x: u32) -> Position {
        Position { x, y: 0 }
    }
    fn known(w: &mut World, a: ActorId, scarce: bool, at: u64) {
        w.tick = at;
        w.observe_rumor(a, S, scarce, 0);
    }
    fn transfer(w: &mut World, from: ActorId, to: ActorId) -> Account {
        let a = w
            .rumor_offer(from, to, false)
            .expect("physical familiar contact");
        let now = w.tick;
        assert!(w.rumors.as_mut().unwrap().receive(to, from, a, now));
        a
    }

    #[test]
    fn c55_actual_conversations_open_player_local_inquiries_and_visits_close_them() {
        use crate::living::narrative::InquiryEnd;
        let mut w = World::narrative_campaign(3);
        w.rumors = Some(Rumors::default());
        let site = S.position(&w);
        w.tick = 10;
        w.region.as_mut().unwrap().market_herbs = 0;
        place(&mut w, A, site);
        w.observe_regional(0);
        place(&mut w, A, row(1));
        place(&mut w, B, row(2));
        w.tick = 20;
        w.region.as_mut().unwrap().market_herbs = 20;
        place(&mut w, C, site);
        w.observe_regional(0);
        place(&mut w, C, row(3));
        // Establish a separate private inquiry; it cannot affect the
        // player's local episode ID or publish itself.
        w.social.meet(A, B);
        let private = transfer(&mut w, A, B);
        w.story_account(B, private);
        assert_eq!(w.known_story_inquiries().count(), 0);
        place(&mut w, PLAYER, row(2));
        w.rumor_conversation(A);
        let i = *w
            .known_story_inquiries()
            .next()
            .expect("player heard account");
        assert_eq!((i.id, i.actor, i.opened), (0, PLAYER, 20));
        assert_eq!(i.first.sample.origin, A);
        assert!(i.conflict.is_none() && i.end.is_none());
        w.tick = 21;
        w.rumor_conversation(C);
        let i = *w.known_story_inquiries().next().unwrap();
        assert_eq!(i.id, 0);
        let conflict = i.conflict.unwrap();
        assert!(conflict.low.scarce && !conflict.adequate.scarce);
        assert_eq!(
            (conflict.low.observed_at, conflict.adequate.observed_at),
            (10, 20)
        );
        assert_eq!(w.known_story_inquiries().count(), 1);
        w.tick = 22;
        place(&mut w, PLAYER, site);
        w.observe_regional(1);
        let closed = *w.known_story_inquiries().next().unwrap();
        assert!(
            matches!(closed.end,Some(InquiryEnd::Checked(s)) if s.origin==PLAYER&&s.observed_at==22&&!s.scarce)
        );
        assert_eq!(w.narrative().unwrap().career(PLAYER).checked, 1);
        assert!(
            !w.narrative()
                .unwrap()
                .active_inquiries()
                .contains_key(&(PLAYER, S))
        );
        let before = w.narrative().unwrap().clone();
        w.observe_regional(1);
        assert_eq!(w.narrative().unwrap(), &before);
        assert_eq!(w.known_story_inquiries().count(), 1);
    }

    #[test]
    fn dated_competing_accounts_require_local_evidence_and_never_modify_world_stock() {
        let mut w = empty();
        // Two historical source observations, on distinct ticks. The fixture
        // controls stock; the actual observation API must establish provenance.
        let site = S.position(&w);
        w.region.as_mut().unwrap().market_herbs = 0;
        place(&mut w, A, site);
        w.observe_regional(0);
        assert!(w.rumors().unwrap().view(A, S, w.tick).local.unwrap().scarce);
        place(&mut w, A, row(1));
        w.tick = 20;
        w.region.as_mut().unwrap().market_herbs = 20;
        place(&mut w, C, site);
        w.observe_regional(0);
        assert!(!w.rumors().unwrap().view(C, S, w.tick).local.unwrap().scarce);
        place(&mut w, C, row(3));
        place(&mut w, B, row(2));
        place(&mut w, PLAYER, row(0));
        w.social.meet(A, B);
        w.social.meet(C, B);
        let first = transfer(&mut w, A, B);
        assert_eq!(first.sample.observed_at, 10);
        assert_eq!(first.sample.origin, A);
        assert_eq!(first.via, Some(A));
        assert_eq!(first.hops, 1);
        assert_eq!(
            w.rumors().unwrap().view(B, S, w.tick).status,
            Status::Reported
        );
        let second = transfer(&mut w, C, B);
        assert_eq!(second.sample.observed_at, 20);
        assert_eq!(
            w.rumors().unwrap().view(B, S, w.tick).status,
            Status::Contested
        );
        assert_eq!(w.known_rumor(S).unwrap().status, Status::Unknown);
        assert_eq!(
            w.rumor_goal(&w.npcs[&B], Goal::Patrol),
            Some(Goal::VerifyRumor(S))
        );
        assert!(w.rumor_goal(&w.npcs[&B], Goal::DeliverFreight).is_none());
        let mut remote = w.clone();
        remote.region.as_mut().unwrap().market_herbs = 0;
        assert_eq!(
            remote.rumor_goal(&remote.npcs[&B], Goal::Patrol),
            w.rumor_goal(&w.npcs[&B], Goal::Patrol)
        );
        let relation = w.social.relationship(B, A);
        let stock = w.region().unwrap().market_herbs;
        let pos = w.npcs[&B].position;
        w.traffic.as_mut().unwrap().begin_tick();
        w.work_verification(B, S);
        assert!(super::super::navigation::distance(pos, w.npcs[&B].position) <= 1);
        assert_eq!(
            w.rumors().unwrap().view(B, S, w.tick).status,
            Status::Contested
        );
        assert_eq!(w.rumors().unwrap().attempts()[&(B, S)].spent, 1);
        w.tick += 1;
        place(&mut w, B, site);
        w.observe_regional(1);
        assert_eq!(
            w.rumors().unwrap().view(B, S, w.tick).status,
            Status::Verified
        );
        assert_eq!(w.rumors().unwrap().counts(), (2, 1, 1));
        assert!(!w.rumors().unwrap().attempts().contains_key(&(B, S)));
        assert!(w.rumor_goal(&w.npcs[&B], Goal::Patrol).is_none());
        assert_eq!(w.region().unwrap().market_herbs, stock);
        assert_eq!(w.social.relationship(B, A), relation);
        assert!(
            w.rumors()
                .unwrap()
                .history()
                .iter()
                .any(|e| e.kind == Kind::Verified && e.recipient == B && e.account.hops == 0)
        );
    }

    #[test]
    fn propagation_is_situated_bounded_and_preserves_origin_without_cascade() {
        let mut w = empty();
        for (a, x) in [(A, 1), (B, 2), (C, 3), (D, 4)] {
            place(&mut w, a, row(x));
        }
        place(&mut w, PLAYER, row(0));
        known(&mut w, A, true, 10);
        // The authored baseline already includes familiar contacts.
        // Isolate the unknown-contact case instead of changing production.
        w.social.relations.remove(&(A, B));
        w.social.relations.remove(&(B, A));
        assert!(w.rumor_offer(A, B, false).is_none());
        for (a, b) in [(A, B), (B, C), (C, D)] {
            w.social.meet(a, b);
        }
        w.advance_rumors();
        let r = w.rumors().unwrap();
        assert_eq!(r.counts(), (1, 0, 0));
        assert_eq!(r.view(C, S, w.tick).status, Status::Unknown);
        let first = r.reports()[&(B, S, true)];
        w.advance_rumors();
        assert_eq!(w.rumors().unwrap().counts().0, 1);
        w.tick += EXCHANGE_INTERVAL;
        w.advance_rumors();
        let second = w.rumors().unwrap().reports()[&(C, S, true)];
        assert_eq!(
            (second.sample.origin, second.via, second.hops),
            (A, Some(B), 2)
        );
        assert!(second.credibility < first.credibility);
        w.tick += EXCHANGE_INTERVAL;
        w.advance_rumors();
        let third = w.rumors().unwrap().reports()[&(D, S, true)];
        assert_eq!(
            (third.sample.origin, third.via, third.hops),
            (A, Some(C), 3)
        );
        assert!(third.credibility < second.credibility);
        assert_eq!(third.sample.observed_at, 10);
        assert!(third.confidence(w.tick + 4) < third.confidence(w.tick));
        assert!(w.rumor_offer(D, C, false).is_none());
        assert_eq!(w.known_rumor(S).unwrap().status, Status::Unknown);
        // An actual player conversation may acquire information; merely
        // standing next to the source did not. Repeating the same report
        // cannot refresh original time or increase confidence.
        w.rumor_conversation(A);
        let player = w.known_rumor(S).unwrap().low.unwrap();
        assert_eq!(player.sample.origin, A);
        let count = w.rumors().unwrap().counts().0;
        w.rumor_conversation(A);
        assert_eq!(w.rumors().unwrap().counts().0, count);
        assert_eq!(w.known_rumor(S).unwrap().low, Some(player));
        w.tick = 10 + LIFETIME;
        w.advance_rumors();
        assert_eq!(w.known_rumor(S).unwrap().status, Status::Unknown);
        assert!(w.rumors().unwrap().reports().is_empty());
    }

    #[test]
    fn trust_changes_offer_confidence_and_superseded_accounts_are_not_forwarded() {
        let mut w = empty();
        place(&mut w, A, row(1));
        place(&mut w, B, row(2));
        place(&mut w, C, row(3));
        known(&mut w, A, true, 10);
        w.social.meet(A, B);
        let neutral = w.rumor_offer(A, B, false).unwrap();
        w.social.relations.get_mut(&(B, A)).unwrap().regard = 4;
        let trusted = w.rumor_offer(A, B, false).unwrap();
        assert!(trusted.credibility > neutral.credibility);
        w.social.relations.get_mut(&(B, A)).unwrap().regard = -1;
        assert!(w.rumor_offer(A, B, false).is_none());
        w.social.relations.get_mut(&(B, A)).unwrap().regard = 0;
        transfer(&mut w, A, B);
        known(&mut w, B, false, 11);
        w.social.meet(B, C);
        let corrected = w.rumor_offer(B, C, false).unwrap();
        assert!(!corrected.sample.scarce);
        assert_eq!(corrected.sample.origin, B);
        place(&mut w, C, row(9));
        assert!(w.rumor_offer(B, C, false).is_none());
    }

    #[test]
    fn verification_is_optional_bounded_and_defers_to_safety() {
        let mut w = empty();
        place(&mut w, A, row(1));
        place(&mut w, B, row(2));
        place(&mut w, C, row(3));
        known(&mut w, A, true, 10);
        known(&mut w, C, false, 20);
        w.social.meet(A, B);
        w.social.meet(C, B);
        transfer(&mut w, A, B);
        transfer(&mut w, C, B);
        assert_eq!(w.npcs[&B].definition.role, Role::Watch);
        w.rumors.as_mut().unwrap().attempts.insert(
            (B, S),
            Attempt {
                spent: VERIFY_BUDGET - 1,
                retry_at: 0,
            },
        );
        w.traffic.as_mut().unwrap().begin_tick();
        w.work_verification(B, S);
        assert_eq!(
            w.rumors().unwrap().attempts()[&(B, S)],
            Attempt {
                spent: 0,
                retry_at: w.tick + VERIFY_YIELD
            }
        );
        assert!(w.rumor_goal(&w.npcs[&B], Goal::Patrol).is_none());
        w.tick += VERIFY_YIELD;
        assert!(w.rumor_goal(&w.npcs[&B], Goal::Patrol).is_some());
        w.npcs.get_mut(&B).unwrap().hp -= 1;
        assert!(w.rumor_goal(&w.npcs[&B], Goal::Patrol).is_none());
        w.npcs.get_mut(&B).unwrap().hp += 1;
        let threshold = w.npcs[&B].definition.rest_threshold;
        w.npcs.get_mut(&B).unwrap().fatigue = threshold;
        assert!(w.rumor_goal(&w.npcs[&B], Goal::Patrol).is_none());
    }

    #[test]
    fn natural_accounts_and_verification_replay() {
        let mut totals = (0, 0, 0);
        let mut active_checks = 0;
        for seed in [0, 3, 7, 42] {
            let mut w = World::rumor_campaign(seed);
            let mut twin = w.clone();
            for _ in 0..2000 {
                assert_eq!(
                    w.step(w.tick(), Intent::Wait),
                    twin.step(twin.tick(), Intent::Wait)
                );
                active_checks += w
                    .npcs()
                    .filter(|n| matches!(n.activity.goal, Goal::VerifyRumor(_)))
                    .count();
            }
            assert_eq!(w, twin);
            assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            let (a, b, c) = w.rumors().unwrap().counts();
            println!("seed {seed}: {:?}", (a, b, c));
            totals.0 += a;
            totals.1 += b;
            totals.2 += c;
        }
        println!("total {totals:?}; active verification goal ticks {active_checks}");
        assert!(totals.0 > 0 && totals.1 > 0 && totals.2 > 0 && active_checks > 0);
    }
}
