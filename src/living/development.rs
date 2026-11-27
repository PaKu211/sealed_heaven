//! C5.6: durable personal learning from actor-owned evidence only.
//! Existing vocation, inventories, relationships and safety rules stay authoritative.
use super::{
    ActorId, Npc, World,
    communication::{Event as Message, Kind as MessageKind},
    narrative::{Inquiry, InquiryEnd},
    navigation::in_reach,
    obligations::{Resolution, ResultKind},
    regional_news::Site,
    rumors::Status,
};
use std::collections::{BTreeMap, VecDeque};

pub const REVISION: u16 = 43;
pub const VERSION: u16 = 1;
pub const MAX_POINTS: u8 = 8;
pub const FOCUS_THRESHOLD: u8 = 3;
pub const CREDIT_INTERVAL: u64 = 32;
pub const SETBACK_YIELD: u64 = 48;
pub const CHECK_YIELD: u64 = 24;
pub const PARTNER_YIELD: u64 = 96;
pub const MIN_LEAD_CONFIDENCE: u8 = 45;
pub const HISTORY_CAP: usize = 192;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum Focus {
    #[default]
    Vocation,
    Inquiry,
    Partnership,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Profile {
    pub focus: Focus,
    pub inquiry_points: u8,
    pub partnership_points: u8,
    pub checked: u32,
    pub attempted_setbacks: u32,
    pub successful_projects: u32,
    pub disrupted_projects: u32,
    pub practice_ticks: u32,
    pub extended_ticks: u32,
    pub inquiry_retry_at: u64,
    pub last_credit: [Option<u64>; 4],
}
impl Profile {
    fn reconsider(&mut self) {
        let eligible = |points: u8| points >= FOCUS_THRESHOLD;
        let (i, p) = (self.inquiry_points, self.partnership_points);
        self.focus = match self.focus {
            Focus::Inquiry if eligible(i) && p < i.saturating_add(2) => Focus::Inquiry,
            Focus::Partnership if eligible(p) && i < p.saturating_add(2) => Focus::Partnership,
            _ if eligible(i) && i >= p => Focus::Inquiry,
            _ if eligible(p) => Focus::Partnership,
            _ => Focus::Vocation,
        };
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Effort {
    pub inquiry: u32,
    pub ticks: u32,
    pub extended_ticks: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PartnerMemory {
    pub last_project: u32,
    pub learned_at: u64,
    pub successful: u32,
    pub disrupted: u32,
    pub retry_at: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ProjectEvidence {
    OwnWork,
    Conversation,
    Register,
    Relay,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cause {
    Checked {
        site: Site,
        inquiry: u32,
        effort: u32,
        credit: u8,
    },
    Setback {
        site: Site,
        inquiry: u32,
        effort: u32,
    },
    Project {
        project: u32,
        peer: ActorId,
        result: ResultKind,
        evidence: ProjectEvidence,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Event {
    pub actor: ActorId,
    pub at: u64,
    pub cause: Cause,
    pub before: Focus,
    pub after: Focus,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Disclosure {
    pub actor: ActorId,
    pub at: u64,
    pub focus: Focus,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Development {
    profiles: BTreeMap<ActorId, Profile>,
    efforts: BTreeMap<(ActorId, Site), Effort>,
    partners: BTreeMap<(ActorId, ActorId), PartnerMemory>,
    history: VecDeque<Event>,
    known: BTreeMap<ActorId, Disclosure>,
    changes: u32,
}
impl Development {
    pub(super) fn new(ids: impl Iterator<Item = ActorId>) -> Self {
        Self {
            profiles: ids.map(|id| (id, Profile::default())).collect(),
            efforts: BTreeMap::new(),
            partners: BTreeMap::new(),
            history: VecDeque::new(),
            known: BTreeMap::new(),
            changes: 0,
        }
    }
    pub fn profiles(&self) -> &BTreeMap<ActorId, Profile> {
        &self.profiles
    }
    pub fn efforts(&self) -> &BTreeMap<(ActorId, Site), Effort> {
        &self.efforts
    }
    pub fn partners(&self) -> &BTreeMap<(ActorId, ActorId), PartnerMemory> {
        &self.partners
    }
    pub fn history(&self) -> &VecDeque<Event> {
        &self.history
    }
    pub fn known(&self) -> &BTreeMap<ActorId, Disclosure> {
        &self.known
    }
    pub fn changes(&self) -> u32 {
        self.changes
    }
    fn event(&mut self, e: Event) {
        if e.before != e.after {
            self.changes += 1;
        }
        if self.history.len() == HISTORY_CAP {
            self.history.pop_front();
        }
        self.history.push_back(e);
    }
    fn effort(&mut self, i: Inquiry, extended: bool) {
        let Some(p) = self.profiles.get_mut(&i.actor) else {
            return;
        };
        let e = self.efforts.entry((i.actor, i.site)).or_insert(Effort {
            inquiry: i.id,
            ticks: 0,
            extended_ticks: 0,
        });
        assert_eq!(e.inquiry, i.id);
        e.ticks += 1;
        p.practice_ticks += 1;
        if extended {
            e.extended_ticks += 1;
            p.extended_ticks += 1;
        }
    }
    fn inquiry_end(&mut self, i: Inquiry, end: InquiryEnd, alive: bool) {
        let effort = self.efforts.remove(&(i.actor, i.site));
        if let Some(e) = effort {
            assert_eq!(e.inquiry, i.id);
        }
        let extended = effort.is_some_and(|e| e.extended_ticks > 0);
        let effort = effort.map_or(0, |e| e.ticks);
        if !alive {
            return;
        }
        let Some(p) = self.profiles.get_mut(&i.actor) else {
            return;
        };
        let before = p.focus;
        let (at, cause) = match end {
            InquiryEnd::Checked(s) => {
                assert_eq!(s.origin, i.actor);
                assert_eq!(s.site, i.site);
                let at = s.observed_at;
                p.checked += 1;
                if extended {
                    p.inquiry_retry_at = p.inquiry_retry_at.max(at + CHECK_YIELD);
                }
                let slot = &mut p.last_credit[i.site as usize];
                let credit = if slot.is_none_or(|t| at.saturating_sub(t) >= CREDIT_INTERVAL) {
                    *slot = Some(at);
                    if effort > 0 { 2 } else { 1 }
                } else {
                    0
                };
                p.inquiry_points = p.inquiry_points.saturating_add(credit).min(MAX_POINTS);
                (
                    at,
                    Cause::Checked {
                        site: i.site,
                        inquiry: i.id,
                        effort,
                        credit,
                    },
                )
            }
            InquiryEnd::Lapsed(at) if effort > 0 => {
                p.attempted_setbacks += 1;
                p.inquiry_points = p.inquiry_points.saturating_sub(1);
                p.inquiry_retry_at = at + SETBACK_YIELD;
                (
                    at,
                    Cause::Setback {
                        site: i.site,
                        inquiry: i.id,
                        effort,
                    },
                )
            }
            InquiryEnd::Lapsed(_) => return, // simply losing interest is not failure
        };
        p.reconsider();
        let after = p.focus;
        self.event(Event {
            actor: i.actor,
            at,
            cause,
            before,
            after,
        });
    }
    fn project(
        &mut self,
        actor: ActorId,
        peer: ActorId,
        project: u32,
        at: u64,
        result: ResultKind,
        evidence: ProjectEvidence,
    ) {
        if result == ResultKind::CarrierLost {
            return;
        }
        let Some(p) = self.profiles.get_mut(&actor) else {
            return;
        };
        let m = self.partners.entry((actor, peer)).or_insert(PartnerMemory {
            last_project: project,
            learned_at: at,
            successful: 0,
            disrupted: 0,
            retry_at: 0,
        });
        if m.successful + m.disrupted > 0 && project <= m.last_project {
            return;
        }
        m.last_project = project;
        m.learned_at = at;
        let before = p.focus;
        if result == ResultKind::Fulfilled {
            p.successful_projects += 1;
            m.successful += 1;
            m.retry_at = 0;
            p.partnership_points = p.partnership_points.saturating_add(3).min(MAX_POINTS);
        } else {
            p.disrupted_projects += 1;
            m.disrupted += 1;
            let released = result == ResultKind::Released;
            p.partnership_points =
                p.partnership_points
                    .saturating_sub(if released { 1 } else { 2 });
            m.retry_at = at
                + if released {
                    SETBACK_YIELD
                } else {
                    PARTNER_YIELD
                };
        }
        p.reconsider();
        let after = p.focus;
        self.event(Event {
            actor,
            at,
            cause: Cause::Project {
                project,
                peer,
                result,
                evidence,
            },
            before,
            after,
        });
    }
    fn disclose(&mut self, actor: ActorId, at: u64) {
        let Some(p) = self.profiles.get(&actor) else {
            return;
        };
        self.known.insert(
            actor,
            Disclosure {
                actor,
                at,
                focus: p.focus,
            },
        );
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        let mut w = Wire(b"SH:C56:DEVELOPMENT:v1\0".to_vec());
        w.u16(VERSION);
        w.u32(self.changes);
        w.count(self.profiles.len());
        for (&a, p) in &self.profiles {
            w.actor(a);
            w.u8(p.focus as u8);
            w.u8(p.inquiry_points);
            w.u8(p.partnership_points);
            for n in [
                p.checked,
                p.attempted_setbacks,
                p.successful_projects,
                p.disrupted_projects,
                p.practice_ticks,
                p.extended_ticks,
            ] {
                w.u32(n);
            }
            w.u64(p.inquiry_retry_at);
            for t in p.last_credit {
                w.u8(u8::from(t.is_some()));
                if let Some(t) = t {
                    w.u64(t);
                }
            }
        }
        w.count(self.efforts.len());
        for (&(a, s), e) in &self.efforts {
            w.actor(a);
            w.u8(s as u8);
            w.u32(e.inquiry);
            w.u32(e.ticks);
            w.u32(e.extended_ticks);
        }
        w.count(self.partners.len());
        for (&(a, b), p) in &self.partners {
            w.actor(a);
            w.actor(b);
            w.u32(p.last_project);
            w.u64(p.learned_at);
            w.u32(p.successful);
            w.u32(p.disrupted);
            w.u64(p.retry_at);
        }
        w.count(self.history.len());
        for e in &self.history {
            w.actor(e.actor);
            w.u64(e.at);
            w.u8(e.before as u8);
            w.u8(e.after as u8);
            match e.cause {
                Cause::Checked {
                    site,
                    inquiry,
                    effort,
                    credit,
                } => {
                    w.u8(0);
                    w.u8(site as u8);
                    w.u32(inquiry);
                    w.u32(effort);
                    w.u8(credit);
                }
                Cause::Setback {
                    site,
                    inquiry,
                    effort,
                } => {
                    w.u8(1);
                    w.u8(site as u8);
                    w.u32(inquiry);
                    w.u32(effort);
                }
                Cause::Project {
                    project,
                    peer,
                    result,
                    evidence,
                } => {
                    w.u8(2);
                    w.u32(project);
                    w.actor(peer);
                    w.u8(result as u8);
                    w.u8(evidence as u8);
                }
            }
        }
        w.count(self.known.len());
        for p in self.known.values() {
            w.actor(p.actor);
            w.u64(p.at);
            w.u8(p.focus as u8);
        }
        w.0
    }
}
struct Wire(Vec<u8>);
impl Wire {
    fn u8(&mut self, x: u8) {
        self.0.push(x);
    }
    fn u16(&mut self, x: u16) {
        self.0.extend(x.to_le_bytes());
    }
    fn u32(&mut self, x: u32) {
        self.0.extend(x.to_le_bytes());
    }
    fn u64(&mut self, x: u64) {
        self.0.extend(x.to_le_bytes());
    }
    fn count(&mut self, x: usize) {
        self.u32(x.try_into().expect("bounded development"));
    }
    fn actor(&mut self, x: ActorId) {
        self.u32(x.get());
    }
}
impl World {
    pub fn development(&self) -> Option<&Development> {
        self.development.as_ref()
    }
    pub fn known_pursuits(&self) -> impl Iterator<Item = &Disclosure> {
        self.development()
            .into_iter()
            .flat_map(|d| d.known.values())
    }
    pub(super) fn note_inquiry_effort(&mut self, actor: ActorId, site: Site, extended: bool) {
        if self.development.is_none() {
            return;
        }
        if let Some(i) = self
            .narrative()
            .and_then(|n| n.active_inquiries().get(&(actor, site)))
            .copied()
        {
            self.development.as_mut().unwrap().effort(i, extended);
        }
    }
    pub(super) fn learn_inquiry_end(&mut self, i: Inquiry, end: InquiryEnd) {
        if self.development.is_none() {
            return;
        }
        let alive = self.npcs.get(&i.actor).is_some_and(|n| n.alive());
        self.development
            .as_mut()
            .unwrap()
            .inquiry_end(i, end, alive);
    }
    pub(super) fn learn_own_resolution(&mut self, r: Resolution) {
        if self.development.is_none() || !self.npcs[&r.team.carrier].alive() {
            return;
        }
        self.development.as_mut().unwrap().project(
            r.team.carrier,
            r.team.grower,
            r.team.id,
            r.at,
            r.result,
            ProjectEvidence::OwnWork,
        );
    }
    pub(super) fn learn_reported_resolution(&mut self, m: Message) {
        if self.development.is_none() || !self.npcs[&m.recipient].alive() {
            return;
        }
        let evidence = match m.kind {
            MessageKind::Direct => ProjectEvidence::Conversation,
            MessageKind::Register => ProjectEvidence::Register,
            MessageKind::Relayed => ProjectEvidence::Relay,
            _ => return,
        };
        self.development.as_mut().unwrap().project(
            m.recipient,
            m.origin,
            m.project,
            m.at,
            m.result,
            evidence,
        );
    }
    pub(super) fn development_can_partner(&self, a: ActorId, b: ActorId) -> bool {
        self.development()
            .and_then(|d| d.partners.get(&(a, b)))
            .is_none_or(|m| self.tick >= m.retry_at)
    }
    pub(super) fn development_partner_bonus(&self, a: ActorId, b: ActorId) -> u16 {
        if self
            .development()
            .and_then(|d| d.profiles.get(&a))
            .is_none_or(|p| p.focus != Focus::Partnership)
        {
            return 0;
        }
        let r = self.social.relationship(a, b);
        if r.refuses_aid() {
            0
        } else if r.trusts_news() {
            2
        } else {
            1
        }
    }
    pub(super) fn development_delivery_bonus(&self, n: &Npc) -> u16 {
        if self.development.is_none() {
            return 0;
        }
        self.cooperation()
            .and_then(|c| c.active().get(&n.id()))
            .map_or(0, |t| self.development_partner_bonus(n.id(), t.grower))
    }
    pub(super) fn development_report_lead(&self, actor: ActorId, site: Site) -> Option<u8> {
        let p = self.development()?.profiles.get(&actor)?;
        if p.focus != Focus::Inquiry || self.tick < p.inquiry_retry_at {
            return None;
        }
        let v = self.rumors()?.view(actor, site, self.tick);
        if v.status != Status::Reported {
            return None;
        }
        let a = v.low.or(v.adequate)?;
        let confidence = a.confidence(self.tick);
        (confidence >= MIN_LEAD_CONFIDENCE && self.social.relationship(actor, a.via?).trusts_news())
            .then_some(confidence)
    }
    pub(super) fn disclose_development(&mut self, actor: ActorId) {
        if self.development.is_none() || !self.player.alive() {
            return;
        }
        let n = &self.npcs[&actor];
        if !n.alive()
            || !in_reach(self.player.position, n.position)
            || !self.can_see(self.player.position, n.position)
            || self.social.relationship(actor, super::PLAYER).refuses_aid()
            || self.social.relationship(super::PLAYER, actor).refuses_aid()
        {
            return;
        }
        self.development
            .as_mut()
            .unwrap()
            .disclose(actor, self.tick);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{
        Intent, PLAYER, Position,
        rumors::{Account, Sample},
    };
    const A: ActorId = ActorId::new(1);
    const B: ActorId = ActorId::new(2);
    const S: Site = Site::WillowfordMarket;
    fn inquiry(id: u32, at: u64) -> Inquiry {
        let sample = Sample {
            site: S,
            scarce: true,
            origin: A,
            observed_at: at,
            phase: 0,
        };
        Inquiry {
            id,
            actor: B,
            site: S,
            opened: at + 1,
            first: Account {
                sample,
                via: Some(A),
                hops: 1,
                acquired_at: at + 1,
                credibility: 67,
            },
            conflict: None,
            end: None,
        }
    }
    fn check(d: &mut Development, id: u32, at: u64, attempted: bool) {
        let i = inquiry(id, at - 2);
        if attempted {
            d.effort(i, true);
        }
        d.inquiry_end(
            i,
            InquiryEnd::Checked(Sample {
                site: S,
                scarce: true,
                origin: B,
                observed_at: at,
                phase: 1,
            }),
            true,
        );
    }

    #[test]
    fn practice_credit_hysteresis_and_setbacks_are_bounded() {
        let mut d = Development::new([A, B].into_iter());
        check(&mut d, 0, 10, true);
        assert_eq!(d.profiles()[&B].inquiry_points, 2);
        check(&mut d, 1, 11, true);
        assert_eq!(d.profiles()[&B].inquiry_points, 2);
        check(&mut d, 2, 42, false);
        assert_eq!(d.profiles()[&B].focus, Focus::Inquiry);
        d.project(B, A, 0, 43, ResultKind::Fulfilled, ProjectEvidence::OwnWork);
        // Tie keeps the current focus, and duplicated evidence is ignored.
        assert_eq!(d.profiles()[&B].focus, Focus::Inquiry);
        let before = d.clone();
        d.project(B, A, 0, 44, ResultKind::Fulfilled, ProjectEvidence::OwnWork);
        assert_eq!(d, before);
        d.project(B, A, 1, 45, ResultKind::Fulfilled, ProjectEvidence::OwnWork);
        assert_eq!(d.profiles()[&B].focus, Focus::Partnership);
        d.project(
            B,
            A,
            2,
            46,
            ResultKind::ChangedCargo,
            ProjectEvidence::OwnWork,
        );
        assert_eq!(d.profiles()[&B].focus, Focus::Partnership);
        let idle = inquiry(3, 50);
        let before = d.profiles()[&B];
        d.inquiry_end(idle, InquiryEnd::Lapsed(250), true);
        assert_eq!(d.profiles()[&B], before);
        let failed = inquiry(4, 251);
        d.effort(failed, false);
        d.inquiry_end(failed, InquiryEnd::Lapsed(450), true);
        assert_eq!(d.profiles()[&B].attempted_setbacks, 1);
        assert_eq!(d.profiles()[&B].inquiry_retry_at, 450 + SETBACK_YIELD);
        let dead = inquiry(5, 451);
        d.effort(dead, false);
        let before = d.profiles()[&B];
        d.inquiry_end(dead, InquiryEnd::Lapsed(650), false);
        assert_eq!(d.profiles()[&B], before);
        for i in 0..250 {
            check(&mut d, 6 + i, 700 + u64::from(i) * CREDIT_INTERVAL, true);
        }
        assert_eq!(d.profiles()[&B].inquiry_points, MAX_POINTS);
        assert_eq!(d.profiles()[&B].focus, Focus::Inquiry);
        assert_eq!(d.history().len(), HISTORY_CAP);
        assert!(d.efforts().is_empty());
    }

    #[test]
    fn disclosures_are_dated_actor_owned_snapshots_not_live_biographies() {
        let mut w = World::development_campaign(3);
        w.tick = 10;
        w.player.position = Position { x: 0, y: 0 };
        w.npcs.get_mut(&B).unwrap().position = Position { x: 1, y: 0 };
        assert!(w.known_pursuits().next().is_none());
        assert!(w.talk(B).is_ok());
        let first = *w.known_pursuits().next().unwrap();
        assert_eq!(
            (first.actor, first.at, first.focus),
            (B, 10, Focus::Vocation)
        );
        for (id, at) in [(0, 50), (1, 90), (2, 130)] {
            check(w.development.as_mut().unwrap(), id, at, false);
        }
        assert_eq!(
            w.development().unwrap().profiles()[&B].focus,
            Focus::Inquiry
        );
        assert_eq!(w.known_pursuits().next(), Some(&first));
        w.tick = 131;
        w.player.position = Position { x: 8, y: 0 };
        assert!(w.talk(B).is_err());
        assert_eq!(w.known_pursuits().next(), Some(&first));
        w.player.position = Position { x: 0, y: 0 };
        assert!(w.talk(B).is_ok());
        let last = w.known_pursuits().next().unwrap();
        assert_eq!((last.actor, last.at, last.focus), (B, 131, Focus::Inquiry));
        assert!(!w.development().unwrap().profiles().contains_key(&PLAYER));
    }

    #[test]
    fn natural_personal_development_changes_choices_and_replays() {
        let mut changes = 0;
        let mut extended = 0;
        let mut differences = 0;
        for seed in [0, 3, 7, 42] {
            let mut w = World::development_campaign(seed);
            let mut twin = w.clone();
            let mut baseline = World::narrative_campaign(seed);
            let mut differing_ticks = 0;
            for _ in 0..2000 {
                assert_eq!(
                    w.step(w.tick(), Intent::Wait),
                    twin.step(twin.tick(), Intent::Wait)
                );
                baseline.step(baseline.tick(), Intent::Wait).unwrap();
                if w.npcs != baseline.npcs {
                    differing_ticks += 1;
                }
            }
            assert_eq!(w, twin);
            assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            let d = w.development().unwrap();
            let ext = d.profiles().values().map(|p| p.extended_ticks).sum::<u32>();
            let practice = d.profiles().values().map(|p| p.practice_ticks).sum::<u32>();
            let setbacks = d
                .profiles()
                .values()
                .map(|p| p.attempted_setbacks)
                .sum::<u32>();
            let checked = d.profiles().values().map(|p| p.checked).sum::<u32>();
            let partnerships = d
                .profiles()
                .values()
                .map(|p| p.successful_projects)
                .sum::<u32>();
            let focus = d
                .profiles()
                .values()
                .filter(|p| p.focus != Focus::Vocation)
                .count();
            println!(
                "seed {seed}: changes={} focus={focus} checks={checked} practice={practice} extended={ext} setbacks={setbacks} successes={partnerships} differing_ticks={differing_ticks} known={}",
                d.changes(),
                d.known().len()
            );
            assert!(d.known().is_empty());
            changes += d.changes();
            extended += ext;
            differences += differing_ticks;
        }
        println!(
            "total changes {changes}; extended {extended}; differing NPC-state ticks {differences}"
        );
        assert!(changes > 0 && extended > 0 && differences > 0);
    }
}
