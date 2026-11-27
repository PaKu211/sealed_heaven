//! C5.8: situated, personal liaisons between associations and institutions.
//! A representative negotiates only their own cooperation. No shared mind,
//! universal faction reputation, coerced membership, goods or remote votes.
use super::{
    ActorId, PLAYER, Role, World,
    circles::Purpose,
    development::MIN_LEAD_CONFIDENCE,
    narrative::{Inquiry, InquiryEnd},
    navigation::in_reach,
    regional_news::Site,
    rumors::{Account, Sample, Status},
};
use std::collections::{BTreeMap, VecDeque};

pub const REVISION: u16 = 45;
pub const VERSION: u16 = 1;
pub const PACT_LIFETIME: u64 = 192;
pub const CASE_LIFETIME: u64 = 96;
pub const REPLY_LIFETIME: u64 = 144;
pub const PETITION_INTERVAL: u64 = 48;
pub const PETITION_LIFETIME: u64 = 48;
pub const HISTORY_CAP: usize = 192;
pub const OBSERVED_CAP: usize = 48;
pub const MAX_REPUTATION: u8 = 8;
type Pair = (ActorId, ActorId);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Party {
    Circle {
        id: u32,
        founder: ActorId,
        purpose: Purpose,
    },
    Hall,
    Stonebank,
    Qinghe,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Side {
    Inquiry,
    Partnership,
    Hall,
    Stonebank,
    Qinghe,
}
impl Party {
    fn side(self) -> Side {
        match self {
            Self::Circle {
                purpose: Purpose::Inquiry,
                ..
            } => Side::Inquiry,
            Self::Circle {
                purpose: Purpose::Partnership,
                ..
            } => Side::Partnership,
            Self::Hall => Side::Hall,
            Self::Stonebank => Side::Stonebank,
            Self::Qinghe => Side::Qinghe,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pact {
    pub a: Party,
    pub b: Party,
    pub opened: u64,
    pub until: u64,
    pub last: u64,
    pub dispute: Option<(Site, u32)>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Case {
    pub id: u32,
    pub reporter: ActorId,
    pub recipient: ActorId,
    pub from: Party,
    pub to: Party,
    pub report: Account,
    pub inquiry: u32,
    pub accepted: u64,
    pub until: u64,
    pub contested: bool,
    pub effort_ticks: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedPetition {
    pub case: Case,
    pub checked: Sample,
    pub until: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Reply {
    pub case: Case,
    pub checked: Sample,
    pub until: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Assessment {
    pub from: Party,
    pub to: Party,
    pub confirmed: u32,
    pub superseded: u32,
    pub score: u8,
    pub last: [Option<(u64, u8)>; 4],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Kind {
    Accord,
    Disputed,
    Received,
    Confirmed,
    Superseded,
    Lapsed,
    Reconciled,
    Petition,
    Withdrawn,
    Expired,
    Returned,
}
const KINDS: usize = 11;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Event {
    pub at: u64,
    pub kind: Kind,
    pub actor: ActorId,
    pub other: ActorId,
    pub from: Party,
    pub to: Party,
    pub site: Option<Site>,
    pub case: Option<u32>,
    pub contract: Option<u32>,
}
/// The public journal has no private party IDs, case IDs or remote status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Notice {
    pub at: u64,
    pub kind: Kind,
    pub actor: ActorId,
    pub other: ActorId,
    pub from: Side,
    pub to: Side,
}
impl From<Event> for Notice {
    fn from(e: Event) -> Self {
        Self {
            at: e.at,
            kind: e.kind,
            actor: e.actor,
            other: e.other,
            from: e.from.side(),
            to: e.to.side(),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Disclosure {
    pub speaker: ActorId,
    pub heard: u64,
    pub event: Notice,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Diplomacy {
    pacts: BTreeMap<Pair, Pact>,
    cases: BTreeMap<ActorId, Case>,
    replies: BTreeMap<Pair, Reply>,
    petitions: BTreeMap<ActorId, VerifiedPetition>,
    assessments: BTreeMap<Pair, Assessment>,
    personal: BTreeMap<ActorId, Notice>,
    known: BTreeMap<ActorId, Disclosure>,
    history: VecDeque<Event>,
    observed: VecDeque<Notice>,
    counts: [u32; KINDS],
    next_id: u32,
    effort_ticks: u32,
    exclusive_ticks: u32,
}
fn pair(a: ActorId, b: ActorId) -> Pair {
    (a.min(b), a.max(b))
}
fn stamp(s: Sample) -> (u64, u8) {
    (s.observed_at, s.phase)
}
impl Diplomacy {
    pub fn pacts(&self) -> &BTreeMap<Pair, Pact> {
        &self.pacts
    }
    pub fn cases(&self) -> &BTreeMap<ActorId, Case> {
        &self.cases
    }
    pub fn replies(&self) -> &BTreeMap<Pair, Reply> {
        &self.replies
    }
    pub fn petitions(&self) -> &BTreeMap<ActorId, VerifiedPetition> {
        &self.petitions
    }
    pub fn assessments(&self) -> &BTreeMap<Pair, Assessment> {
        &self.assessments
    }
    pub fn history(&self) -> &VecDeque<Event> {
        &self.history
    }
    pub fn observed(&self) -> &VecDeque<Notice> {
        &self.observed
    }
    pub fn known(&self) -> &BTreeMap<ActorId, Disclosure> {
        &self.known
    }
    pub fn effort_ticks(&self) -> u32 {
        self.effort_ticks
    }
    pub fn exclusive_ticks(&self) -> u32 {
        self.exclusive_ticks
    }
    pub fn count(&self, k: Kind) -> u32 {
        self.counts[k as usize]
    }
    fn emit(&mut self, e: Event, seen: bool, participants: &[ActorId]) {
        self.counts[e.kind as usize] += 1;
        if self.history.len() == HISTORY_CAP {
            self.history.pop_front();
        }
        self.history.push_back(e);
        let notice = Notice::from(e);
        for &a in participants {
            self.personal.insert(a, notice);
        }
        if seen {
            if self.observed.len() == OBSERVED_CAP {
                self.observed.pop_front();
            }
            self.observed.push_back(notice);
        }
    }
    fn matching(&self, a: ActorId, b: ActorId, from: Party, to: Party, now: u64) -> Option<&Pact> {
        let p = self.pacts.get(&pair(a, b))?;
        let sides = if a < b { (from, to) } else { (to, from) };
        (now < p.until && (p.a, p.b) == sides).then_some(p)
    }
    fn expire(&mut self, key: Pair, at: u64) {
        if let Some(p) = self.pacts.remove(&key) {
            self.emit(
                Event {
                    at,
                    kind: Kind::Expired,
                    actor: key.0,
                    other: key.1,
                    from: p.a,
                    to: p.b,
                    site: None,
                    case: None,
                    contract: None,
                },
                false,
                &[],
            );
        }
    }
    fn case_event(c: Case, at: u64, kind: Kind) -> Event {
        Event {
            at,
            kind,
            actor: c.reporter,
            other: c.recipient,
            from: c.from,
            to: c.to,
            site: Some(c.report.sample.site),
            case: Some(c.id),
            contract: None,
        }
    }
    fn lapse(&mut self, c: Case, at: u64) {
        self.emit(Self::case_event(c, at, Kind::Lapsed), false, &[c.recipient]);
    }
    fn assess(&mut self, c: Case, checked: Sample) {
        // A forwarded claim does not establish the immediate messenger's
        // firsthand reliability. Only independently checked direct evidence.
        if c.report.hops != 1 || c.report.sample.origin != c.reporter {
            return;
        }
        let a = self
            .assessments
            .entry((c.recipient, c.reporter))
            .or_insert(Assessment {
                from: c.from,
                to: c.to,
                confirmed: 0,
                superseded: 0,
                score: 0,
                last: [None; 4],
            });
        if (a.from, a.to) != (c.from, c.to) {
            *a = Assessment {
                from: c.from,
                to: c.to,
                confirmed: 0,
                superseded: 0,
                score: 0,
                last: [None; 4],
            };
        }
        let slot = &mut a.last[c.report.sample.site as usize];
        let s = stamp(c.report.sample);
        if slot.is_some_and(|old| old >= s) {
            return;
        }
        *slot = Some(s);
        if checked.scarce == c.report.sample.scarce {
            a.confirmed += 1;
            a.score = a.score.saturating_add(1).min(MAX_REPUTATION);
        } else {
            a.superseded += 1;
        } // stock may change: no automatic blame or penalty
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        let mut w = Wire(b"SH:C58:DIPLOMACY:v1\0".to_vec());
        w.u16(VERSION);
        w.u32(self.next_id);
        w.u32(self.effort_ticks);
        w.u32(self.exclusive_ticks);
        for n in self.counts {
            w.u32(n);
        }
        w.count(self.pacts.len());
        for (&(a, b), p) in &self.pacts {
            w.actor(a);
            w.actor(b);
            w.party(p.a);
            w.party(p.b);
            w.u64(p.opened);
            w.u64(p.until);
            w.u64(p.last);
            w.opt(p.dispute, |w, (site, id)| {
                w.site(site);
                w.u32(id);
            });
        }
        w.count(self.cases.len());
        for c in self.cases.values() {
            w.case(*c);
        }
        w.count(self.replies.len());
        for (&(a, b), r) in &self.replies {
            w.actor(a);
            w.actor(b);
            w.case(r.case);
            w.sample(r.checked);
            w.u64(r.until);
        }
        w.count(self.petitions.len());
        for p in self.petitions.values() {
            w.case(p.case);
            w.sample(p.checked);
            w.u64(p.until);
        }
        w.count(self.assessments.len());
        for (&(a, b), r) in &self.assessments {
            w.actor(a);
            w.actor(b);
            w.party(r.from);
            w.party(r.to);
            w.u32(r.confirmed);
            w.u32(r.superseded);
            w.u8(r.score);
            for t in r.last {
                w.opt(t, |w, (at, phase)| {
                    w.u64(at);
                    w.u8(phase);
                });
            }
        }
        w.count(self.personal.len());
        for (&a, &n) in &self.personal {
            w.actor(a);
            w.notice(n);
        }
        w.count(self.known.len());
        for d in self.known.values() {
            w.actor(d.speaker);
            w.u64(d.heard);
            w.notice(d.event);
        }
        w.count(self.history.len());
        for e in &self.history {
            w.u64(e.at);
            w.u8(e.kind as u8);
            w.actor(e.actor);
            w.actor(e.other);
            w.party(e.from);
            w.party(e.to);
            w.opt(e.site, Wire::site);
            w.opt(e.case, Wire::u32);
            w.opt(e.contract, Wire::u32);
        }
        w.count(self.observed.len());
        for &n in &self.observed {
            w.notice(n);
        }
        w.0
    }
}
struct Wire(Vec<u8>);
impl Wire {
    fn u8(&mut self, n: u8) {
        self.0.push(n);
    }
    fn u16(&mut self, n: u16) {
        self.0.extend(n.to_le_bytes());
    }
    fn u32(&mut self, n: u32) {
        self.0.extend(n.to_le_bytes());
    }
    fn u64(&mut self, n: u64) {
        self.0.extend(n.to_le_bytes());
    }
    fn actor(&mut self, a: ActorId) {
        self.u32(a.get());
    }
    fn site(&mut self, s: Site) {
        self.u8(s as u8);
    }
    fn count(&mut self, n: usize) {
        self.u32(n.try_into().expect("bounded diplomacy"));
    }
    fn opt<T>(&mut self, n: Option<T>, f: impl FnOnce(&mut Self, T)) {
        self.u8(u8::from(n.is_some()));
        if let Some(n) = n {
            f(self, n);
        }
    }
    fn party(&mut self, p: Party) {
        match p {
            Party::Circle {
                id,
                founder,
                purpose,
            } => {
                self.u8(0);
                self.u32(id);
                self.actor(founder);
                self.u8(purpose as u8);
            }
            Party::Hall => self.u8(1),
            Party::Stonebank => self.u8(2),
            Party::Qinghe => self.u8(3),
        }
    }
    fn sample(&mut self, s: Sample) {
        self.site(s.site);
        self.u8(u8::from(s.scarce));
        self.actor(s.origin);
        self.u64(s.observed_at);
        self.u8(s.phase);
    }
    fn account(&mut self, a: Account) {
        self.sample(a.sample);
        self.opt(a.via, Wire::actor);
        self.u8(a.hops);
        self.u64(a.acquired_at);
        self.u8(a.credibility);
    }
    fn case(&mut self, c: Case) {
        self.u32(c.id);
        self.actor(c.reporter);
        self.actor(c.recipient);
        self.party(c.from);
        self.party(c.to);
        self.account(c.report);
        self.u32(c.inquiry);
        self.u64(c.accepted);
        self.u64(c.until);
        self.u8(u8::from(c.contested));
        self.u32(c.effort_ticks);
    }
    fn notice(&mut self, n: Notice) {
        self.u64(n.at);
        self.u8(n.kind as u8);
        self.actor(n.actor);
        self.actor(n.other);
        self.u8(n.from as u8);
        self.u8(n.to as u8);
    }
}
impl World {
    pub fn diplomacy(&self) -> Option<&Diplomacy> {
        self.diplomacy.as_ref()
    }
    pub fn witnessed_diplomacy(&self) -> impl DoubleEndedIterator<Item = &Notice> {
        self.diplomacy().into_iter().flat_map(|d| d.observed.iter())
    }
    pub fn known_diplomacy(&self) -> impl Iterator<Item = &Disclosure> {
        self.diplomacy().into_iter().flat_map(|d| d.known.values())
    }
    fn diplomatic_party(&self, a: ActorId) -> Option<Party> {
        let n = self.npcs.get(&a)?;
        if !n.alive() {
            return None;
        }
        match n.definition.role {
            Role::RivalEnvoy => Some(Party::Stonebank),
            Role::Watch => Some(Party::Qinghe),
            Role::HallSteward | Role::HallDeputy => Some(Party::Hall),
            _ => self
                .circles()?
                .group_of(a)
                .filter(|g| self.circle_interest(a, g.purpose))
                .map(|g| Party::Circle {
                    id: g.id,
                    founder: g.founder,
                    purpose: g.purpose,
                }),
        }
    }
    fn diplomatic_parties(&self, a: ActorId, b: ActorId) -> Option<(Party, Party)> {
        self.diplomacy()?;
        if a == PLAYER || b == PLAYER || a == b {
            return None;
        }
        let (x, y) = (self.diplomatic_party(a)?, self.diplomatic_party(b)?);
        if !self.circle_contact(a, b)
            || !self.circle_available(a)
            || !self.circle_available(b)
            || !self.circle_familiar(a, b)
        {
            return None;
        }

        (x != y && (matches!(x, Party::Circle { .. }) || matches!(y, Party::Circle { .. })))
            .then_some((x, y))
    }
    pub(super) fn diplomatic_answer_priority(
        &self,
        a: ActorId,
        b: ActorId,
        report: Account,
    ) -> bool {
        let Some((from, to)) = self.diplomatic_parties(a, b) else {
            return false;
        };
        self.diplomacy()
            .unwrap()
            .replies
            .get(&(a, b))
            .is_some_and(|r| {
                self.tick < r.until
                    && (r.case.to, r.case.from) == (from, to)
                    && r.checked.site == report.sample.site
                    && report.sample.origin == a
                    && report.hops == 1
                    && report.confidence(self.tick) >= MIN_LEAD_CONFIDENCE
                    && stamp(report.sample) >= stamp(r.checked)
            })
    }
    pub(super) fn diplomatic_priority(&self, a: ActorId, b: ActorId, s: Site) -> u8 {
        let Some((x, y)) = self.diplomatic_parties(a, b) else {
            return 0;
        };
        let d = self.diplomacy().unwrap();
        let Some(p) = d.matching(a, b, x, y, self.tick) else {
            return 0;
        };
        if let Some((site, _)) = p.dispute {
            return if site == s { 3 } else { 0 };
        }
        if d.assessments
            .get(&(b, a))
            .is_some_and(|r| (r.from, r.to) == (x, y) && r.score > 0)
        {
            2
        } else {
            1
        }
    }
    pub(super) fn diplomatic_account(&mut self, a: ActorId, b: ActorId, report: Account) {
        let Some((from, to)) = self.diplomatic_parties(a, b) else {
            return;
        };
        let site = report.sample.site;
        // Exact committed receipt only; no proposal or remote inference.
        if report.via != Some(a)
            || report.acquired_at != self.tick
            || report.confidence(self.tick) < MIN_LEAD_CONFIDENCE
            || self
                .rumors()
                .unwrap()
                .reports()
                .get(&(b, site, report.sample.scarce))
                != Some(&report)
        {
            return;
        }
        let now = self.tick;
        let seen = self.circle_seen(&[a, b]);
        let key = pair(a, b);
        let contested = self.rumors().unwrap().view(b, site, now).status == Status::Contested;
        let inquiry = self
            .narrative()
            .unwrap()
            .active_inquiries()
            .get(&(b, site))
            .copied();
        let d = self.diplomacy.as_mut().unwrap();
        if d.matching(a, b, from, to, now)
            .is_some_and(|p| p.last == now)
        {
            return;
        }
        if d.matching(a, b, from, to, now).is_none() {
            d.expire(key, now);
            let (x, y) = if a < b { (from, to) } else { (to, from) };
            d.pacts.insert(
                key,
                Pact {
                    a: x,
                    b: y,
                    opened: now,
                    until: now + PACT_LIFETIME,
                    last: now,
                    dispute: None,
                },
            );
            d.emit(
                Event {
                    at: now,
                    kind: Kind::Accord,
                    actor: a,
                    other: b,
                    from,
                    to,
                    site: None,
                    case: None,
                    contract: None,
                },
                seen,
                &[a, b],
            );
        } else {
            let p = d.pacts.get_mut(&key).unwrap();
            p.until = now + PACT_LIFETIME;
            p.last = now;
        }
        // The original reporter learns the outcome only when the investigator
        // actually transfers newer firsthand evidence back in person.
        if let Some(reply) = d.replies.get(&(a, b)).copied().filter(|r| {
            now < r.until
                && (r.case.to, r.case.from) == (from, to)
                && r.checked.site == site
                && report.hops == 1
                && report.sample.origin == a
                && stamp(report.sample) >= stamp(r.checked)
        }) {
            d.replies.remove(&(a, b));
            let same = d.pacts[&key].dispute == Some((site, reply.case.id));
            let reconciled = same && !contested;
            if reconciled {
                d.pacts.get_mut(&key).unwrap().dispute = None;
            }
            d.emit(
                Event {
                    at: now,
                    kind: if reconciled {
                        Kind::Reconciled
                    } else {
                        Kind::Returned
                    },
                    actor: a,
                    other: b,
                    from,
                    to,
                    site: Some(site),
                    case: Some(reply.case.id),
                    contract: None,
                },
                seen,
                &[a, b],
            );
            // If the answer itself leaves contradictory accounts, it is
            // counterevidence, not settlement; a new local check may follow.
            if !contested {
                return;
            }
        }
        let Some(i) = inquiry else {
            return;
        };
        if d.cases.contains_key(&b) {
            return;
        }
        let id = d.next_id;
        d.next_id += 1;
        let c = Case {
            id,
            reporter: a,
            recipient: b,
            from,
            to,
            report,
            inquiry: i.id,
            accepted: now,
            until: now + CASE_LIFETIME,
            contested,
            effort_ticks: 0,
        };
        d.cases.insert(b, c);
        d.emit(Diplomacy::case_event(c, now, Kind::Received), false, &[b]);
        if contested {
            let p = d.pacts.get_mut(&key).unwrap();
            p.dispute = Some((site, id));
            d.emit(Diplomacy::case_event(c, now, Kind::Disputed), seen, &[a, b]);
        }
    }
    fn diplomatic_fresh_case(&self, a: ActorId, s: Site) -> Option<Case> {
        let c = *self.diplomacy()?.cases.get(&a)?;
        if c.report.sample.site != s
            || self.tick >= c.until
            || !c.report.fresh(self.tick)
            || c.report.confidence(self.tick) < MIN_LEAD_CONFIDENCE
            || self.diplomatic_party(a) != Some(c.to)
            || !self.social.relationship(a, c.reporter).trusts_news()
        {
            return None;
        }
        let r = self.rumors()?;
        if r.reports().get(&(a, s, c.report.sample.scarce)) != Some(&c.report) {
            return None;
        }
        matches!(
            r.view(a, s, self.tick).status,
            Status::Reported | Status::Contested
        )
        .then_some(c)
    }
    pub(super) fn diplomatic_report_lead(&self, a: ActorId, s: Site) -> Option<u8> {
        let c = self.diplomatic_fresh_case(a, s)?;
        let interested = match c.to {
            Party::Circle {
                purpose: Purpose::Inquiry,
                ..
            } => true,
            Party::Circle {
                purpose: Purpose::Partnership,
                ..
            } => matches!(s, Site::WillowfordMarket | Site::PinefordDepot),
            Party::Hall | Party::Stonebank => {
                matches!(s, Site::QingheShrine | Site::WillowfordMarket)
            }
            Party::Qinghe => s == Site::QingheShrine && c.report.sample.scarce,
        };
        if !interested
            || self.tick < self.development()?.profiles()[&a].inquiry_retry_at
            || self.rumors()?.view(a, s, self.tick).status != Status::Reported
        {
            return None;
        }
        Some(c.report.confidence(self.tick))
    }
    pub(super) fn note_diplomatic_effort(&mut self, a: ActorId, s: Site) {
        if self.diplomatic_report_lead(a, s).is_none() {
            return;
        }
        let exclusive =
            self.development_report_lead(a, s).is_none() && self.circle_report_lead(a, s).is_none();
        let d = self.diplomacy.as_mut().unwrap();
        d.cases.get_mut(&a).unwrap().effort_ticks += 1;
        d.effort_ticks += 1;
        if exclusive {
            d.exclusive_ticks += 1;
        }
    }
    /// A received complaint never opens a contract. An independently
    /// checked local shortage can shorten only the ordinary retry interval.
    pub(super) fn diplomatic_petition_ready(&self, a: ActorId, last_closed: u64) -> bool {
        let Some(p) = self.diplomacy().and_then(|d| d.petitions.get(&a)) else {
            return false;
        };
        last_closed > 0
            && self.tick.saturating_sub(last_closed) >= PETITION_INTERVAL
            && self.tick < p.until
            && self.circle_available(a)
            && self.social.relationship(a, p.case.reporter).trusts_news()
            && self.rumors().is_some_and(|r| {
                let v = r.view(a, Site::QingheShrine, self.tick);
                v.status == Status::Verified
                    && v.local
                        .is_some_and(|s| s.scarce && stamp(s) >= stamp(p.checked))
            })
    }
    pub(super) fn diplomatic_petition_resolved(
        &mut self,
        a: ActorId,
        contract: u32,
        accelerated: bool,
    ) {
        let Some(d) = self.diplomacy.as_mut() else {
            return;
        };
        if let Some(p) = d.petitions.remove(&a)
            && accelerated
        {
            let mut event = Diplomacy::case_event(p.case, self.tick, Kind::Petition);
            event.contract = Some(contract);
            d.emit(event, false, &[a]);
        }
    }
    pub(super) fn diplomatic_inquiry_end(&mut self, i: Inquiry, end: InquiryEnd, alive: bool) {
        let Some(d) = self.diplomacy.as_mut() else {
            return;
        };
        let Some(c) = d
            .cases
            .get(&i.actor)
            .copied()
            .filter(|c| c.inquiry == i.id && c.report.sample.site == i.site)
        else {
            return;
        };
        d.cases.remove(&i.actor);
        if let InquiryEnd::Checked(s) = end
            && alive
            && self.tick < c.until
            && s.origin == c.recipient
            && s.site == i.site
            && s.observed_at == self.tick
            && stamp(s) >= stamp(c.report.sample)
        {
            let kind = if s.scarce == c.report.sample.scarce {
                Kind::Confirmed
            } else {
                Kind::Superseded
            };
            d.assess(c, s);
            if c.to == Party::Qinghe
                && c.report.sample.site == Site::QingheShrine
                && c.report.sample.scarce
                && s.scarce
            {
                d.petitions.insert(
                    c.recipient,
                    VerifiedPetition {
                        case: c,
                        checked: s,
                        until: self.tick + PETITION_LIFETIME,
                    },
                );
            }
            d.emit(
                Diplomacy::case_event(c, self.tick, kind),
                false,
                &[c.recipient],
            );
            if c.contested {
                d.replies.insert(
                    (c.recipient, c.reporter),
                    Reply {
                        case: c,
                        checked: s,
                        until: self.tick + REPLY_LIFETIME,
                    },
                );
            }
        } else {
            d.lapse(c, self.tick);
        }
    }
    pub(super) fn advance_diplomacy(&mut self) {
        let Some(d) = self.diplomacy() else {
            return;
        };
        let stale = d
            .pacts
            .iter()
            .filter(|(_, p)| self.tick >= p.until)
            .map(|(&k, _)| k)
            .collect::<Vec<_>>();
        // Only a physically encountered refusal withdraws a pact early.
        let refused = d
            .pacts
            .iter()
            .filter_map(|(&(a, b), p)| {
                (self.tick < p.until
                    && self.circle_contact(a, b)
                    && (self.social.relationship(a, b).refuses_aid()
                        || self.social.relationship(b, a).refuses_aid()))
                .then_some(((a, b), *p, self.circle_seen(&[a, b])))
            })
            .collect::<Vec<_>>();
        let stale_cases = d
            .cases
            .values()
            .copied()
            .filter(|c| self.tick >= c.until)
            .collect::<Vec<_>>();
        let d = self.diplomacy.as_mut().unwrap();
        for k in stale {
            d.expire(k, self.tick);
        }
        for ((a, b), p, seen) in refused {
            d.pacts.remove(&(a, b));
            d.emit(
                Event {
                    at: self.tick,
                    kind: Kind::Withdrawn,
                    actor: a,
                    other: b,
                    from: p.a,
                    to: p.b,
                    site: None,
                    case: None,
                    contract: None,
                },
                seen,
                &[a, b],
            );
        }
        for c in stale_cases {
            d.cases.remove(&c.recipient);
            d.lapse(c, self.tick);
        }
        d.replies.retain(|_, r| self.tick < r.until);
        d.petitions.retain(|_, p| self.tick < p.until);
    }
    pub(super) fn disclose_diplomacy(&mut self, a: ActorId) {
        let Some(d) = self.diplomacy() else {
            return;
        };
        let n = &self.npcs[&a];
        if !self.player.alive()
            || !n.alive()
            || !in_reach(self.player.position, n.position)
            || !self.can_see(self.player.position, n.position)
            || self.social.relationship(a, PLAYER).refuses_aid()
            || self.social.relationship(PLAYER, a).refuses_aid()
        {
            return;
        }
        if let Some(&event) = d.personal.get(&a) {
            self.diplomacy.as_mut().unwrap().known.insert(
                a,
                Disclosure {
                    speaker: a,
                    heard: self.tick,
                    event,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::Intent;
    #[test]
    fn natural_diplomacy_is_deterministic_and_changes_real_simulation() {
        let mut totals = [0u32; KINDS];
        let mut differences = 0;
        let mut cross_group = 0;
        let mut institutional = 0;
        let mut exclusive = 0;
        for seed in [0, 3, 7, 42] {
            let mut w = World::diplomacy_campaign(seed);
            let mut twin = w.clone();
            let mut baseline = World::circles_campaign(seed);
            let mut different = 0;
            let mut cross = 0;
            let mut inst = 0;
            for _ in 0..2000 {
                assert_eq!(
                    w.step(w.tick(), Intent::Wait),
                    twin.step(twin.tick(), Intent::Wait)
                );
                baseline.step(baseline.tick(), Intent::Wait).unwrap();
                if w.npcs != baseline.npcs {
                    different += 1;
                }
                for e in w
                    .diplomacy()
                    .unwrap()
                    .history()
                    .iter()
                    .rev()
                    .take_while(|e| e.at == w.tick())
                    .filter(|e| e.kind == Kind::Accord)
                {
                    if matches!(e.from, Party::Circle { .. })
                        && matches!(e.to, Party::Circle { .. })
                    {
                        cross += 1;
                    } else {
                        inst += 1;
                    }
                }
            }
            assert_eq!(w, twin);
            assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            let d = w.diplomacy().unwrap();
            assert!(d.known().is_empty());
            assert_eq!(d.count(Kind::Accord), cross + inst);
            println!(
                "seed {seed}: counts {:?}; cross-group {cross}; institutional {inst}; differing NPC ticks {different}; pacts {} assessments {} known {} witnessed {}",
                d.counts,
                d.pacts.len(),
                d.assessments.len(),
                d.known.len(),
                d.observed.len()
            );
            for (x, n) in totals.iter_mut().zip(d.counts) {
                *x += n;
            }
            differences += different;
            cross_group += cross;
            institutional += inst;
            exclusive += d.exclusive_ticks();
            println!(
                "seed {seed}: diplomatic effort {} exclusive {}",
                d.effort_ticks(),
                d.exclusive_ticks()
            );
        }
        println!(
            "total {totals:?}; cross-group {cross_group}; institutional {institutional}; differing NPC ticks {differences}"
        );
        assert!(
            totals[Kind::Accord as usize] > 0
                && totals[Kind::Confirmed as usize] > 0
                && differences > 0
                && exclusive > 0
        );
    }
    #[test]
    fn corroboration_is_bounded_directional_and_never_counts_forwarded_or_duplicate_receipts() {
        let a = ActorId::new(1);
        let b = ActorId::new(2);
        let from = Party::Circle {
            id: 0,
            founder: a,
            purpose: Purpose::Inquiry,
        };
        let to = Party::Qinghe;
        let mut d = Diplomacy::default();
        for at in 1..20 {
            let sample = Sample {
                site: Site::QingheShrine,
                scarce: true,
                origin: a,
                observed_at: at,
                phase: 0,
            };
            let report = Account {
                sample,
                via: Some(a),
                hops: 1,
                acquired_at: at + 1,
                credibility: 65,
            };
            let c = Case {
                id: at as u32,
                reporter: a,
                recipient: b,
                from,
                to,
                report,
                inquiry: at as u32,
                accepted: at + 1,
                until: at + CASE_LIFETIME,
                contested: false,
                effort_ticks: 0,
            };
            let checked = Sample {
                origin: b,
                observed_at: at + 2,
                ..sample
            };
            d.assess(c, checked);
            let snapshot = d.clone();
            d.assess(c, checked);
            assert_eq!(d, snapshot);
            let mut forwarded = c;
            forwarded.report.hops = 2;
            forwarded.report.sample.observed_at += 1;
            d.assess(forwarded, checked);
            assert_eq!(d, snapshot);
        }
        assert_eq!(d.assessments[&(b, a)].score, MAX_REPUTATION);
        assert_eq!(d.assessments[&(b, a)].confirmed, 19);
        assert!(!d.assessments.contains_key(&(a, b)));
        let sample = Sample {
            site: Site::QingheShrine,
            scarce: true,
            origin: a,
            observed_at: 40,
            phase: 0,
        };
        let c = Case {
            id: 40,
            reporter: a,
            recipient: b,
            from,
            to,
            report: Account {
                sample,
                via: Some(a),
                hops: 1,
                acquired_at: 41,
                credibility: 65,
            },
            inquiry: 40,
            accepted: 41,
            until: 100,
            contested: false,
            effort_ticks: 0,
        };
        d.assess(
            c,
            Sample {
                origin: b,
                observed_at: 42,
                scarce: false,
                ..sample
            },
        );
        assert_eq!(d.assessments[&(b, a)].score, MAX_REPUTATION);
        assert_eq!(d.assessments[&(b, a)].superseded, 1);
    }
}
