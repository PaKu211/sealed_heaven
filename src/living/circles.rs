//! C5.7: small voluntary, physically negotiated associations.
//! No pooled goods, remote stock knowledge, group orders or automatic loyalty.
use super::{
    ActorId, DIRECTIONS, Goal, PLAYER, Position, World,
    cooperation::Team,
    development::{Focus, MIN_LEAD_CONFIDENCE},
    narrative::{Inquiry, InquiryEnd},
    navigation,
    obligations::{Resolution, ResultKind},
    regional_news::Site,
    rumors::{Account, Status},
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
pub const REVISION: u16 = 44;
pub const VERSION: u16 = 1;
pub const MEMBER_CAP: usize = 3;
pub const HISTORY_CAP: usize = 192;
pub const OBSERVED_CAP: usize = 48;
pub const INACTIVITY: u64 = 480;
pub const MISMATCH_GRACE: u64 = 48;
pub const REJOIN_YIELD: u64 = 96;
pub const ELECTION_INTERVAL: u64 = 96;
pub const LEAD_LIFETIME: u64 = 96;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum Purpose {
    Inquiry,
    Partnership,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Member {
    pub joined: u64,
    pub met: u64,
    pub mismatch_since: Option<u64>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Circle {
    pub id: u32,
    pub founder: ActorId,
    pub purpose: Purpose,
    pub created: u64,
    pub coordinator: Option<ActorId>,
    pub election_at: u64,
    pub members: BTreeMap<ActorId, Member>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Lead {
    pub circle: u32,
    pub actor: ActorId,
    pub sponsor: ActorId,
    pub site: Site,
    pub inquiry: u32,
    pub accepted: u64,
    pub until: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Kind {
    Formed,
    Joined,
    Elected,
    ChangedFocus,
    Disagreed,
    Separated,
    Lost,
    Lead,
    Checked,
    Lapsed,
    Project,
    Delivered,
    Disrupted,
    Dissolved,
}
const KIND_COUNT: usize = 14;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Event {
    pub at: u64,
    pub circle: u32,
    pub purpose: Purpose,
    pub actor: ActorId,
    pub other: Option<ActorId>,
    pub kind: Kind,
    pub site: Option<Site>,
    pub evidence: Option<u32>,
}
/// Only a physically learned self-description, not a live remote roster.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Observed {
    pub at: u64,
    pub purpose: Purpose,
    pub actor: ActorId,
    pub other: Option<ActorId>,
    pub kind: Kind,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Disclosure {
    pub actor: ActorId,
    pub at: u64,
    pub affiliation: Option<(Purpose, ActorId, bool)>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectLink {
    pub circle: u32,
    pub team: Team,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Circles {
    groups: BTreeMap<u32, Circle>,
    membership: BTreeMap<ActorId, u32>,
    retry: BTreeMap<ActorId, u64>,
    leads: BTreeMap<ActorId, Lead>,
    projects: BTreeMap<u32, ProjectLink>,
    history: VecDeque<Event>,
    observed: VecDeque<Observed>,
    known: BTreeMap<ActorId, Disclosure>,
    next_id: u32,
    counts: [u32; KIND_COUNT],
}
impl Circles {
    pub fn groups(&self) -> &BTreeMap<u32, Circle> {
        &self.groups
    }
    pub fn membership(&self) -> &BTreeMap<ActorId, u32> {
        &self.membership
    }
    pub fn leads(&self) -> &BTreeMap<ActorId, Lead> {
        &self.leads
    }
    pub fn projects(&self) -> &BTreeMap<u32, ProjectLink> {
        &self.projects
    }
    pub fn history(&self) -> &VecDeque<Event> {
        &self.history
    }
    pub fn observed(&self) -> &VecDeque<Observed> {
        &self.observed
    }
    pub fn known(&self) -> &BTreeMap<ActorId, Disclosure> {
        &self.known
    }
    pub fn count(&self, k: Kind) -> u32 {
        self.counts[k as usize]
    }
    pub fn group_of(&self, a: ActorId) -> Option<&Circle> {
        self.membership.get(&a).and_then(|id| self.groups.get(id))
    }
    fn ready(&self, a: ActorId, now: u64) -> bool {
        self.retry.get(&a).is_none_or(|&t| now >= t)
    }
    fn emit(&mut self, e: Event, seen: bool) {
        self.counts[e.kind as usize] += 1;
        if self.history.len() == HISTORY_CAP {
            self.history.pop_front();
        }
        self.history.push_back(e);
        if seen {
            if self.observed.len() == OBSERVED_CAP {
                self.observed.pop_front();
            }
            self.observed.push_back(Observed {
                at: e.at,
                purpose: e.purpose,
                actor: e.actor,
                other: e.other,
                kind: e.kind,
            });
        }
    }
    #[cfg(test)]
    pub(super) fn test_form(&mut self, a: ActorId, b: ActorId, purpose: Purpose, at: u64) {
        self.form(a, b, purpose, a, at, (false, None));
    }
    fn form(
        &mut self,
        a: ActorId,
        b: ActorId,
        purpose: Purpose,
        leader: ActorId,
        at: u64,
        observation: (bool, Option<u32>),
    ) {
        let (seen, evidence) = observation;
        assert!(self.group_of(a).is_none() && self.group_of(b).is_none() && a != b);
        let id = self.next_id;
        self.next_id += 1;
        let member = Member {
            joined: at,
            met: at,
            mismatch_since: None,
        };
        self.groups.insert(
            id,
            Circle {
                id,
                founder: a,
                purpose,
                created: at,
                coordinator: Some(leader),
                election_at: at,
                members: BTreeMap::from([(a, member), (b, member)]),
            },
        );
        self.membership.insert(a, id);
        self.membership.insert(b, id);
        self.emit(
            Event {
                at,
                circle: id,
                purpose,
                actor: a,
                other: Some(b),
                kind: Kind::Formed,
                site: None,
                evidence,
            },
            seen,
        );
        self.emit(
            Event {
                at,
                circle: id,
                purpose,
                actor: leader,
                other: None,
                kind: Kind::Elected,
                site: None,
                evidence: None,
            },
            seen,
        );
    }
    fn join(&mut self, id: u32, a: ActorId, sponsor: ActorId, at: u64, seen: bool) {
        assert!(self.group_of(a).is_none());
        let g = self.groups.get_mut(&id).unwrap();
        assert!(g.members.len() < MEMBER_CAP && g.members.contains_key(&sponsor));
        g.members.insert(
            a,
            Member {
                joined: at,
                met: at,
                mismatch_since: None,
            },
        );
        for m in g.members.values_mut() {
            m.met = at;
        }
        self.membership.insert(a, id);
        let purpose = g.purpose;
        self.emit(
            Event {
                at,
                circle: id,
                purpose,
                actor: a,
                other: Some(sponsor),
                kind: Kind::Joined,
                site: None,
                evidence: None,
            },
            seen,
        );
    }
    fn elect(&mut self, id: u32, a: ActorId, at: u64, seen: bool) {
        let g = self.groups.get_mut(&id).unwrap();
        assert!(g.members.contains_key(&a));
        if g.coordinator == Some(a) {
            return;
        }
        let other = g.coordinator;
        g.coordinator = Some(a);
        g.election_at = at;
        let purpose = g.purpose;
        self.emit(
            Event {
                at,
                circle: id,
                purpose,
                actor: a,
                other,
                kind: Kind::Elected,
                site: None,
                evidence: None,
            },
            seen,
        );
    }
    fn leave(&mut self, a: ActorId, at: u64, kind: Kind, other: Option<ActorId>) {
        let Some(id) = self.membership.remove(&a) else {
            return;
        };
        let g = self.groups.get_mut(&id).unwrap();
        g.members.remove(&a);
        if g.coordinator == Some(a) {
            g.coordinator = None;
        }
        let purpose = g.purpose;
        let empty = g.members.is_empty();
        self.retry.insert(a, at + REJOIN_YIELD);
        if empty {
            self.groups.remove(&id);
        }
        // The leaver owns the decision; remote peers do not learn it.
        self.emit(
            Event {
                at,
                circle: id,
                purpose,
                actor: a,
                other,
                kind,
                site: None,
                evidence: None,
            },
            false,
        );
        if let Some(l) = self.leads.remove(&a) {
            self.finish_lead(l, at, Kind::Lapsed);
        }
        if empty {
            self.emit(
                Event {
                    at,
                    circle: id,
                    purpose,
                    actor: a,
                    other: None,
                    kind: Kind::Dissolved,
                    site: None,
                    evidence: None,
                },
                false,
            );
        }
    }
    fn finish_lead(&mut self, l: Lead, at: u64, kind: Kind) {
        self.emit(
            Event {
                at,
                circle: l.circle,
                purpose: Purpose::Inquiry,
                actor: l.actor,
                other: Some(l.sponsor),
                kind,
                site: Some(l.site),
                evidence: Some(l.inquiry),
            },
            false,
        );
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        let mut w = Wire(b"SH:C57:CIRCLES:v1\0".to_vec());
        w.u16(VERSION);
        w.u32(self.next_id);
        for n in self.counts {
            w.u32(n);
        }
        w.count(self.groups.len());
        for (&id, g) in &self.groups {
            w.u32(id);
            w.actor(g.founder);
            w.u8(g.purpose as u8);
            w.u64(g.created);
            w.opt(g.coordinator, Wire::actor);
            w.u64(g.election_at);
            w.count(g.members.len());
            for (&a, m) in &g.members {
                w.actor(a);
                w.u64(m.joined);
                w.u64(m.met);
                w.opt(m.mismatch_since, Wire::u64);
            }
        }
        w.count(self.membership.len());
        for (&a, &id) in &self.membership {
            w.actor(a);
            w.u32(id);
        }
        w.count(self.retry.len());
        for (&a, &at) in &self.retry {
            w.actor(a);
            w.u64(at);
        }
        w.count(self.leads.len());
        for l in self.leads.values() {
            w.u32(l.circle);
            w.actor(l.actor);
            w.actor(l.sponsor);
            w.u8(l.site as u8);
            w.u32(l.inquiry);
            w.u64(l.accepted);
            w.u64(l.until);
        }
        w.count(self.projects.len());
        for (&id, p) in &self.projects {
            w.u32(id);
            w.u32(p.circle);
            let t = p.team;
            w.u32(t.id);
            w.u8(t.route as u8);
            w.actor(t.grower);
            w.actor(t.carrier);
            w.u64(t.opened);
            w.u64(t.deadline);
            w.u32(t.contributed);
            w.u32(t.shipment);
            w.u8(u8::from(t.opening_seen));
        }
        w.count(self.history.len());
        for e in &self.history {
            w.u64(e.at);
            w.u32(e.circle);
            w.u8(e.purpose as u8);
            w.actor(e.actor);
            w.opt(e.other, Wire::actor);
            w.u8(e.kind as u8);
            w.opt(e.site, |w, s| w.u8(s as u8));
            w.opt(e.evidence, Wire::u32);
        }
        w.count(self.observed.len());
        for e in &self.observed {
            w.u64(e.at);
            w.u8(e.purpose as u8);
            w.actor(e.actor);
            w.opt(e.other, Wire::actor);
            w.u8(e.kind as u8);
        }
        w.count(self.known.len());
        for d in self.known.values() {
            w.actor(d.actor);
            w.u64(d.at);
            w.opt(d.affiliation, |w, (p, f, c)| {
                w.u8(p as u8);
                w.actor(f);
                w.u8(u8::from(c));
            });
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
    fn count(&mut self, n: usize) {
        self.u32(n.try_into().expect("bounded circles"));
    }
    fn opt<T>(&mut self, n: Option<T>, f: impl FnOnce(&mut Self, T)) {
        self.u8(u8::from(n.is_some()));
        if let Some(n) = n {
            f(self, n);
        }
    }
}
impl World {
    pub fn circles(&self) -> Option<&Circles> {
        self.circles.as_ref()
    }
    pub fn known_circles(&self) -> impl Iterator<Item = &Disclosure> {
        self.circles().into_iter().flat_map(|c| c.known.values())
    }
    pub fn witnessed_circles(&self) -> impl DoubleEndedIterator<Item = &Observed> {
        self.circles().into_iter().flat_map(|c| c.observed.iter())
    }
    pub(super) fn circle_available(&self, a: ActorId) -> bool {
        let n = &self.npcs[&a];
        n.alive()
            && n.hp == n.definition.hp
            && n.fatigue < n.definition.rest_threshold
            && !self.injuries().unwrap().people[&a].injured()
            && !self.hobbled(a)
            && self.companion(a).is_none()
            && !self.sect_duty_active(a)
            && !matches!(
                n.activity.goal,
                Goal::Retreat
                    | Goal::Retaliate
                    | Goal::SeekCare
                    | Goal::GatherForCare
                    | Goal::SellForCare
            )
    }
    pub(super) fn circle_interest(&self, a: ActorId, p: Purpose) -> bool {
        let d = &self.development().unwrap().profiles()[&a];
        match p {
            Purpose::Inquiry => {
                self.npcs[&a].definition.shares_news
                    && (d.focus == Focus::Inquiry
                        || (d.focus == Focus::Vocation && d.inquiry_points >= 2))
            }
            Purpose::Partnership => d.focus == Focus::Partnership,
        }
    }
    fn circle_strength(&self, a: ActorId, p: Purpose) -> (u8, u32) {
        let d = &self.development().unwrap().profiles()[&a];
        match p {
            Purpose::Inquiry => (d.inquiry_points, d.checked),
            Purpose::Partnership => (d.partnership_points, d.successful_projects),
        }
    }
    pub(super) fn circle_familiar(&self, a: ActorId, b: ActorId) -> bool {
        self.social.relationship(a, b).trusts_news() && self.social.relationship(b, a).trusts_news()
    }
    fn circle_hostile(&self, a: ActorId, b: ActorId) -> bool {
        self.social.relationship(a, b).refuses_aid() || self.social.relationship(b, a).refuses_aid()
    }
    pub(super) fn circle_contact(&self, a: ActorId, b: ActorId) -> bool {
        let (x, y) = (self.npcs[&a].position, self.npcs[&b].position);
        self.npcs[&a].alive()
            && self.npcs[&b].alive()
            && navigation::distance(x, y) == 1
            && self.can_see(x, y)
    }
    pub(super) fn circle_seen(&self, actors: &[ActorId]) -> bool {
        self.player.alive()
            && actors
                .iter()
                .all(|a| self.player_sees(self.npcs[a].position))
    }
    fn circle_pair_interest(&self, a: ActorId, b: ActorId, p: Purpose) -> bool {
        if !self.circle_interest(a, p) || !self.circle_interest(b, p) || !self.circle_familiar(a, b)
        {
            return false;
        }
        match p {
            Purpose::Inquiry => {
                self.development().unwrap().profiles()[&a].focus == Focus::Inquiry
                    || self.development().unwrap().profiles()[&b].focus == Focus::Inquiry
            }
            Purpose::Partnership => {
                let partners = self.development().unwrap().partners();
                matches!((partners.get(&(a,b)),partners.get(&(b,a))),(Some(x),Some(y)) if x.successful>0&&y.successful>0&&x.retry_at==0&&y.retry_at==0&&x.last_project==y.last_project)
            }
        }
    }
    // All participants in a group decision must physically gather around
    // the negotiating sponsor. No remote votes or leadership election.
    fn circle_assembly(&self, sponsor: ActorId, actors: &[ActorId]) -> bool {
        actors
            .iter()
            .all(|&a| self.circle_available(a) && (a == sponsor || self.circle_contact(a, sponsor)))
            && actors.iter().all(|&a| {
                actors.iter().all(|&b| {
                    a == b
                        || (self.circle_familiar(a, b)
                            && self.can_see(self.npcs[&a].position, self.npcs[&b].position))
                })
            })
    }
    fn circle_best(&self, p: Purpose, actors: &[ActorId]) -> ActorId {
        *actors
            .iter()
            .max_by_key(|&&a| (self.circle_strength(a, p), std::cmp::Reverse(a)))
            .unwrap()
    }
    pub(super) fn advance_circles(&mut self) {
        let Some(c) = self.circles() else {
            return;
        };
        let members: Vec<_> = c.membership.keys().copied().collect();
        // Own loss of interest and own lack of contact are private exits.
        for a in members {
            let g = self.circles().unwrap().group_of(a).unwrap();
            let id = g.id;
            let m = g.members[&a];
            let p = g.purpose;
            let interested = self.circle_interest(a, p);
            let mismatch = if interested {
                None
            } else {
                m.mismatch_since.or(Some(self.tick))
            };
            let reason = if !self.npcs[&a].alive() {
                Some(Kind::Lost)
            } else if mismatch.is_some_and(|t| self.tick.saturating_sub(t) >= MISMATCH_GRACE) {
                Some(Kind::ChangedFocus)
            } else if self.tick.saturating_sub(m.met) >= INACTIVITY {
                Some(Kind::Separated)
            } else {
                None
            };
            let c = self.circles.as_mut().unwrap();
            if let Some(k) = reason {
                c.leave(a, self.tick, k, None);
            } else {
                c.groups
                    .get_mut(&id)
                    .unwrap()
                    .members
                    .get_mut(&a)
                    .unwrap()
                    .mismatch_since = mismatch;
            }
        }
        let local: BTreeMap<Position, ActorId> = self
            .npcs()
            .filter(|n| n.alive())
            .map(|n| (n.position, n.id()))
            .collect();
        let mut pairs = Vec::new();
        for (&pos, &a) in &local {
            for dir in DIRECTIONS {
                if let Some(b) = navigation::moved(pos, dir)
                    .and_then(|p| local.get(&p))
                    .copied()
                    && a < b
                    && self.circle_contact(a, b)
                {
                    pairs.push((a, b));
                }
            }
        }
        pairs.sort_unstable();
        let mut used = BTreeSet::new();
        for (a, b) in pairs {
            if used.contains(&a) || used.contains(&b) {
                continue;
            }
            let c = self.circles().unwrap();
            let (ga, gb) = (c.group_of(a).map(|g| g.id), c.group_of(b).map(|g| g.id));
            if let Some(id) = ga.filter(|&id| Some(id) == gb) {
                // Each party can independently refuse continuing association.
                if self.circle_hostile(a, b) {
                    let leave_a = self.social.relationship(a, b).refuses_aid();
                    let leave_b = self.social.relationship(b, a).refuses_aid();
                    let c = self.circles.as_mut().unwrap();
                    if leave_a {
                        c.leave(a, self.tick, Kind::Disagreed, Some(b));
                    }
                    if leave_b {
                        c.leave(b, self.tick, Kind::Disagreed, Some(a));
                    }
                    used.insert(a);
                    used.insert(b);
                    continue;
                }
                let g = &self.circles().unwrap().groups[&id];
                let p = g.purpose;
                let actors = g.members.keys().copied().collect::<Vec<_>>();
                let ready = (g.coordinator.is_none()
                    || self.tick.saturating_sub(g.election_at) >= ELECTION_INTERVAL)
                    && actors.iter().any(|&s| self.circle_assembly(s, &actors))
                    && actors.iter().all(|&x| self.circle_interest(x, p));
                let choice = self.circle_best(p, &actors);
                let change = ready
                    && g.coordinator.is_none_or(|old| {
                        self.circle_strength(choice, p).0
                            > self.circle_strength(old, p).0.saturating_add(1)
                    });
                let seen = self.circle_seen(&actors);
                let c = self.circles.as_mut().unwrap();
                let g = c.groups.get_mut(&id).unwrap();
                g.members.get_mut(&a).unwrap().met = self.tick;
                g.members.get_mut(&b).unwrap().met = self.tick;
                if change {
                    c.elect(id, choice, self.tick, seen);
                    used.extend(actors);
                }
                continue;
            }
            if !self.circle_available(a) || !self.circle_available(b) {
                continue;
            }
            if ga.is_none() && gb.is_none() {
                let c = self.circles().unwrap();
                if !c.ready(a, self.tick) || !c.ready(b, self.tick) {
                    continue;
                }
                if let Some(p) = [Purpose::Partnership, Purpose::Inquiry]
                    .into_iter()
                    .find(|&p| self.circle_pair_interest(a, b, p))
                {
                    let leader = self.circle_best(p, &[a, b]);
                    let seen = self.circle_seen(&[a, b]);
                    let evidence = (p == Purpose::Partnership)
                        .then(|| self.development().unwrap().partners()[&(a, b)].last_project);
                    self.circles.as_mut().unwrap().form(
                        a,
                        b,
                        p,
                        leader,
                        self.tick,
                        (seen, evidence),
                    );
                    used.insert(a);
                    used.insert(b);
                }
            } else if let Some((id, sponsor, applicant)) = match (ga, gb) {
                (Some(id), None) => Some((id, a, b)),
                (None, Some(id)) => Some((id, b, a)),
                _ => None,
            } {
                let c = self.circles().unwrap();
                let g = &c.groups[&id];
                let p = g.purpose;
                if g.members.len() >= MEMBER_CAP
                    || !c.ready(applicant, self.tick)
                    || !g.coordinator.is_none_or(|x| x == sponsor)
                {
                    continue;
                }
                let mut actors = g.members.keys().copied().collect::<Vec<_>>();
                actors.push(applicant);
                if !self.circle_pair_interest(sponsor, applicant, p)
                    || !self.circle_assembly(sponsor, &actors)
                    || !actors.iter().all(|&a| self.circle_interest(a, p))
                {
                    continue;
                }
                let vacancy = g.coordinator.is_none();
                let leader = self.circle_best(p, &actors);
                let seen = self.circle_seen(&actors);
                let c = self.circles.as_mut().unwrap();
                c.join(id, applicant, sponsor, self.tick, seen);
                if vacancy {
                    c.elect(id, leader, self.tick, seen);
                }
                used.extend(actors);
            }
        }
    }
    pub(super) fn circle_peers(&self, a: ActorId, b: ActorId) -> bool {
        let Some(c) = self.circles() else {
            return false;
        };
        c.group_of(a)
            .is_some_and(|g| g.purpose == Purpose::Inquiry && g.members.contains_key(&b))
            && self.circle_familiar(a, b)
            && self.circle_interest(a, Purpose::Inquiry)
            && self.circle_interest(b, Purpose::Inquiry)
    }
    pub(super) fn circle_account(&mut self, a: ActorId, b: ActorId, report: Account) {
        if !self.circle_peers(a, b)
            || !self.circle_contact(a, b)
            || !self.circle_available(b)
            || report.via != Some(a)
            || report.acquired_at != self.tick
            || self.tick < self.development().unwrap().profiles()[&b].inquiry_retry_at
            || report.confidence(self.tick) < MIN_LEAD_CONFIDENCE
            || !self.circle_interest(b, Purpose::Inquiry)
        {
            return;
        }
        let c = self.circles().unwrap();
        if c.leads.contains_key(&b) {
            return;
        }
        let Some(i) = self
            .narrative()
            .unwrap()
            .active_inquiries()
            .get(&(b, report.sample.site))
        else {
            return;
        };
        let id = c.group_of(b).unwrap().id;
        let l = Lead {
            circle: id,
            actor: b,
            sponsor: a,
            site: i.site,
            inquiry: i.id,
            accepted: self.tick,
            until: self.tick + LEAD_LIFETIME,
        };
        let c = self.circles.as_mut().unwrap();
        c.leads.insert(b, l);
        c.emit(
            Event {
                at: self.tick,
                circle: id,
                purpose: Purpose::Inquiry,
                actor: b,
                other: Some(a),
                kind: Kind::Lead,
                site: Some(l.site),
                evidence: Some(l.inquiry),
            },
            false,
        );
    }
    pub(super) fn circle_report_lead(&self, a: ActorId, s: Site) -> Option<u8> {
        let c = self.circles()?;
        let l = c.leads.get(&a)?;
        if l.site != s || self.tick >= l.until || !self.circle_interest(a, Purpose::Inquiry) {
            return None;
        }
        let d = &self.development()?.profiles()[&a];
        if self.tick < d.inquiry_retry_at {
            return None;
        }
        let v = self.rumors()?.view(a, s, self.tick);
        if v.status != Status::Reported {
            return None;
        }
        let r = v.low.or(v.adequate)?;
        let confidence = r.confidence(self.tick);
        (confidence >= MIN_LEAD_CONFIDENCE && self.social.relationship(a, r.via?).trusts_news())
            .then_some(confidence)
    }
    pub(super) fn circle_inquiry_end(&mut self, i: Inquiry, end: InquiryEnd, alive: bool) {
        let Some(c) = self.circles.as_mut() else {
            return;
        };
        let Some(l) = c
            .leads
            .get(&i.actor)
            .copied()
            .filter(|l| l.site == i.site && l.inquiry == i.id)
        else {
            return;
        };
        c.leads.remove(&i.actor);
        c.finish_lead(
            l,
            self.tick,
            if alive && self.tick < l.until && matches!(end, InquiryEnd::Checked(_)) {
                Kind::Checked
            } else {
                Kind::Lapsed
            },
        );
    }
    pub(super) fn prune_circle_leads(&mut self) {
        let Some(c) = self.circles() else {
            return;
        };
        let stale: Vec<_> = c
            .leads
            .values()
            .copied()
            .filter(|l| self.tick >= l.until)
            .collect();
        let c = self.circles.as_mut().unwrap();
        for l in stale {
            c.leads.remove(&l.actor);
            c.finish_lead(l, self.tick, Kind::Lapsed);
        }
    }
    pub(super) fn circle_project_open(&mut self, t: Team) {
        let Some(c) = self.circles() else {
            return;
        };
        let Some(g) = c
            .group_of(t.grower)
            .filter(|g| g.purpose == Purpose::Partnership && g.members.contains_key(&t.carrier))
        else {
            return;
        };
        let id = g.id;
        let c = self.circles.as_mut().unwrap();
        assert!(
            c.projects
                .insert(
                    t.id,
                    ProjectLink {
                        circle: id,
                        team: t
                    }
                )
                .is_none()
        );
        c.emit(
            Event {
                at: t.opened,
                circle: id,
                purpose: Purpose::Partnership,
                actor: t.grower,
                other: Some(t.carrier),
                kind: Kind::Project,
                site: None,
                evidence: Some(t.id),
            },
            false,
        );
    }
    pub(super) fn circle_project_end(&mut self, r: Resolution) {
        let Some(c) = self.circles.as_mut() else {
            return;
        };
        if let Some(p) = c.projects.remove(&r.team.id) {
            assert_eq!(p.team, r.team);
            c.emit(
                Event {
                    at: r.at,
                    circle: p.circle,
                    purpose: Purpose::Partnership,
                    actor: r.team.carrier,
                    other: Some(r.team.grower),
                    kind: if r.result == ResultKind::Fulfilled {
                        Kind::Delivered
                    } else {
                        Kind::Disrupted
                    },
                    site: None,
                    evidence: Some(r.team.id),
                },
                false,
            );
        }
    }
    pub(super) fn disclose_circle(&mut self, a: ActorId) {
        let Some(c) = self.circles() else {
            return;
        };
        let n = &self.npcs[&a];
        if !self.player.alive()
            || !n.alive()
            || !navigation::in_reach(self.player.position, n.position)
            || !self.can_see(self.player.position, n.position)
            || self.social.relationship(a, PLAYER).refuses_aid()
            || self.social.relationship(PLAYER, a).refuses_aid()
        {
            return;
        }
        let affiliation = c
            .group_of(a)
            .map(|g| (g.purpose, g.founder, g.coordinator == Some(a)));
        self.circles.as_mut().unwrap().known.insert(
            a,
            Disclosure {
                actor: a,
                at: self.tick,
                affiliation,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Intent, development::Profile};
    const A: ActorId = ActorId::new(1);
    const B: ActorId = ActorId::new(2);
    const C: ActorId = ActorId::new(3);
    fn row(x: u32) -> Position {
        Position { x, y: 0 }
    }
    fn place(w: &mut World, a: ActorId, x: u32) {
        let n = w.npcs.get_mut(&a).unwrap();
        n.position = row(x);
        n.activity.goal = Goal::Idle;
        n.fatigue = 0;
    }
    fn interest(w: &mut World, a: ActorId, points: u8) {
        *w.development.as_mut().unwrap().test_profile_mut(a) = Profile {
            focus: if points >= 3 {
                Focus::Inquiry
            } else {
                Focus::Vocation
            },
            inquiry_points: points,
            ..Profile::default()
        };
    }
    fn setup() -> World {
        let mut w = World::circles_campaign(3);
        w.tick = 10;
        w.player.position = row(0);
        place(&mut w, A, 2);
        place(&mut w, B, 1);
        place(&mut w, C, 9);
        interest(&mut w, A, 3);
        interest(&mut w, B, 2);
        interest(&mut w, C, 2);
        for (a, b) in [(A, B), (A, C), (B, C)] {
            w.social.meet(a, b);
        }
        w
    }
    #[test]
    fn local_formation_admission_leadership_and_directional_disagreement() {
        let mut w = setup();
        let supply = w.supply;
        let agency = w.character_agency().unwrap().clone();
        let social = w.social.clone();
        w.advance_circles();
        let c = w.circles().unwrap();
        let g = c.group_of(A).unwrap();
        assert_eq!(g.purpose, Purpose::Inquiry);
        assert_eq!(g.members.keys().copied().collect::<Vec<_>>(), vec![A, B]);
        assert_eq!(g.coordinator, Some(A));
        assert_eq!(c.count(Kind::Formed), 1);
        assert_eq!(c.observed().len(), 2);
        assert!(c.group_of(C).is_none());
        let id = g.id;
        w.tick += 1;
        place(&mut w, C, 3);
        place(&mut w, B, 8);
        w.advance_circles();
        assert!(
            w.circles().unwrap().group_of(C).is_none(),
            "no remote admission"
        );
        place(&mut w, B, 1);
        w.tick += 1;
        w.advance_circles();
        assert_eq!(w.circles().unwrap().groups()[&id].members.len(), MEMBER_CAP);
        assert_eq!(w.circles().unwrap().count(Kind::Joined), 1);
        interest(&mut w, C, 6);
        w.tick += ELECTION_INTERVAL;
        place(&mut w, B, 9);
        w.advance_circles();
        assert_eq!(
            w.circles().unwrap().groups()[&id].coordinator,
            Some(A),
            "no remote election"
        );
        place(&mut w, B, 1);
        w.tick += 1;
        w.advance_circles();
        assert_eq!(w.circles().unwrap().groups()[&id].coordinator, Some(C));
        assert_eq!(w.circles().unwrap().count(Kind::Elected), 2);
        assert_eq!(w.supply, supply);
        assert_eq!(w.character_agency().unwrap(), &agency);
        assert_eq!(w.social, social);
        let known = w.circles().unwrap().observed().clone();
        w.social.relations.get_mut(&(B, A)).unwrap().regard = -1;
        w.tick += 1;
        w.advance_circles();
        let c = w.circles().unwrap();
        assert!(c.group_of(B).is_none() && c.group_of(A).is_some() && c.group_of(C).is_some());
        assert_eq!(c.count(Kind::Disagreed), 1);
        assert_eq!(c.observed(), &known);
        assert!(!c.ready(B, w.tick));
    }
    #[test]
    fn private_loss_and_focus_exit_do_not_update_disclosures_or_elect_remotely() {
        let mut w = setup();
        w.advance_circles();
        w.tick += 1;
        place(&mut w, C, 3);
        w.advance_circles();
        w.player.position = row(0);
        w.disclose_circle(B);
        let before = *w.known_circles().next().unwrap();
        assert_eq!(before.affiliation, Some((Purpose::Inquiry, A, false)));
        let public = w.circles().unwrap().observed().clone();
        w.tick += 1;
        w.player.position = row(40);
        place(&mut w, A, 7);
        place(&mut w, B, 1);
        place(&mut w, C, 9);
        w.npcs.get_mut(&A).unwrap().hp = 0;
        w.advance_circles();
        let c = w.circles().unwrap();
        assert_eq!(c.count(Kind::Lost), 1);
        assert_eq!(c.group_of(B).unwrap().coordinator, None);
        assert_eq!(c.observed(), &public);
        assert_eq!(w.known_circles().next(), Some(&before));
        w.tick += 1;
        place(&mut w, C, 2);
        w.advance_circles();
        assert!(
            w.circles()
                .unwrap()
                .group_of(B)
                .unwrap()
                .coordinator
                .is_some()
        );
        assert_eq!(w.circles().unwrap().observed(), &public);
        interest(&mut w, B, 0);
        w.tick += 1;
        w.advance_circles();
        assert!(w.circles().unwrap().group_of(B).is_some());
        w.tick += MISMATCH_GRACE - 1;
        w.advance_circles();
        assert!(w.circles().unwrap().group_of(B).is_some());
        w.tick += 1;
        w.advance_circles();
        assert!(w.circles().unwrap().group_of(B).is_none());
        assert_eq!(w.circles().unwrap().count(Kind::ChangedFocus), 1);
        assert_eq!(w.known_circles().next(), Some(&before));
        w.player.position = row(0);
        w.disclose_circle(B);
        assert_eq!(w.known_circles().next().unwrap().affiliation, None);
        w.tick += INACTIVITY;
        w.advance_circles();
        let c = w.circles().unwrap();
        assert!(c.groups().is_empty());
        assert_eq!(c.count(Kind::Dissolved), 1);
    }
    #[test]
    fn natural_groups_and_collective_activity_replay() {
        let mut totals = [0u32; KIND_COUNT];
        for seed in [0, 3, 7, 42] {
            let mut w = World::circles_campaign(seed);
            let mut twin = w.clone();
            for _ in 0..2000 {
                assert_eq!(
                    w.step(w.tick(), Intent::Wait),
                    twin.step(twin.tick(), Intent::Wait)
                );
            }
            assert_eq!(w, twin);
            assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
            let c = w.circles().unwrap();
            println!(
                "seed {seed}: counts {:?}; active {} members {} leads {} known {} witnessed {}",
                c.counts,
                c.groups.len(),
                c.membership.len(),
                c.leads.len(),
                c.known.len(),
                c.observed.len()
            );
            assert!(c.known().is_empty());
            for (t, n) in totals.iter_mut().zip(c.counts) {
                *t += n;
            }
        }
        println!("total {totals:?}");
        assert!(totals[Kind::Formed as usize] > 0);
    }
}
