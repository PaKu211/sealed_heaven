//! C5.5: bounded causal episodes from committed simulation events.
//! No director, scripted plot, resource mutation or privileged player knowledge.
use super::{
    ActorId, PLAYER, Position, World,
    communication::{Event as Message, Kind as MessageKind},
    cooperation::{Evidence, Route, Team, Witnessed},
    obligations::{Resolution, ResultKind},
    regional_news::Site,
    rumors::{Account, Sample, Status, View},
};
use std::collections::{BTreeMap, VecDeque};
pub const REVISION: u16 = 42;
pub const VERSION: u16 = 1;
pub const PROJECT_HISTORY_CAP: usize = 96;
pub const INQUIRY_HISTORY_CAP: usize = 128;
pub const KNOWN_CAP: usize = 48;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectEnd {
    pub at: u64,
    pub kind: MessageKind,
    pub holder: Option<ActorId>,
    pub place: Option<Position>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectArc {
    pub team: Team,
    pub outcome: Option<Resolution>,
    pub handoff: Option<Message>,
    pub end: Option<ProjectEnd>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Dispatch {
    pub at: u64,
    pub route: Route,
    pub shipment: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Delivery {
    pub at: u64,
    pub learned_at: u64,
    pub route: Route,
    pub shipment: u32,
    pub evidence: Evidence,
}
/// Evidence-only projection. A witnessed verbal report has its exchange
/// tick, not the hidden time/location of the material outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KnownProject {
    pub id: u32,
    pub grower: ActorId,
    pub carrier: ActorId,
    pub dispatch: Option<Dispatch>,
    pub delivery: Option<Delivery>,
    pub handoff: Option<Message>,
    pub report: Option<Message>,
    pub updated: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Conflict {
    pub at: u64,
    pub low: Sample,
    pub adequate: Sample,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InquiryEnd {
    Checked(Sample),
    Lapsed(u64),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Inquiry {
    pub id: u32,
    pub actor: ActorId,
    pub site: Site,
    pub opened: u64,
    pub first: Account,
    pub conflict: Option<Conflict>,
    pub end: Option<InquiryEnd>,
}
/// Engine-side durable career counters, not automatically public biography.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Career {
    pub projects: u32,
    pub delivered: u32,
    pub disrupted: u32,
    pub confirmed: u32,
    pub unreported: u32,
    pub inquiries: u32,
    pub checked: u32,
    pub lapsed: u32,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Narrative {
    active_projects: BTreeMap<u32, ProjectArc>,
    project_history: VecDeque<ProjectArc>,
    known_projects: BTreeMap<u32, KnownProject>,
    active_inquiries: BTreeMap<(ActorId, Site), Inquiry>,
    inquiry_history: VecDeque<Inquiry>,
    known_inquiries: VecDeque<Inquiry>,
    careers: BTreeMap<ActorId, Career>,
    next_inquiry: u32,
    projects: u32,
    closed_projects: u32,
    inquiries_checked: u32,
    inquiries_lapsed: u32,
}
fn bounded<T>(rows: &mut VecDeque<T>, cap: usize, value: T) {
    if rows.len() == cap {
        rows.pop_front();
    }
    rows.push_back(value);
}
impl Narrative {
    pub fn active_projects(&self) -> &BTreeMap<u32, ProjectArc> {
        &self.active_projects
    }
    pub fn project_history(&self) -> &VecDeque<ProjectArc> {
        &self.project_history
    }
    pub fn known_projects(&self) -> &BTreeMap<u32, KnownProject> {
        &self.known_projects
    }
    pub fn active_inquiries(&self) -> &BTreeMap<(ActorId, Site), Inquiry> {
        &self.active_inquiries
    }
    pub fn inquiry_history(&self) -> &VecDeque<Inquiry> {
        &self.inquiry_history
    }
    pub fn known_inquiries(&self) -> &VecDeque<Inquiry> {
        &self.known_inquiries
    }
    pub fn career(&self, id: ActorId) -> Career {
        self.careers.get(&id).copied().unwrap_or_default()
    }
    pub fn counts(&self) -> (u32, u32, u32, u32, u32) {
        (
            self.projects,
            self.closed_projects,
            self.next_inquiry,
            self.inquiries_checked,
            self.inquiries_lapsed,
        )
    }
    fn project_public(
        &mut self,
        id: u32,
        grower: ActorId,
        carrier: ActorId,
        at: u64,
    ) -> &mut KnownProject {
        if !self.known_projects.contains_key(&id) && self.known_projects.len() == KNOWN_CAP {
            let evict = *self
                .known_projects
                .values()
                .min_by_key(|p| (p.updated, p.id))
                .map(|p| &p.id)
                .unwrap();
            self.known_projects.remove(&evict);
        }
        let p = self.known_projects.entry(id).or_insert(KnownProject {
            id,
            grower,
            carrier,
            dispatch: None,
            delivery: None,
            handoff: None,
            report: None,
            updated: at,
        });
        debug_assert_eq!((p.grower, p.carrier), (grower, carrier));
        p
    }
    fn open(&mut self, t: Team) {
        assert!(
            self.active_projects
                .insert(
                    t.id,
                    ProjectArc {
                        team: t,
                        outcome: None,
                        handoff: None,
                        end: None
                    }
                )
                .is_none()
        );
        self.projects += 1;
        for a in [t.grower, t.carrier] {
            self.careers.entry(a).or_default().projects += 1;
        }
        if t.opening_seen {
            let p = self.project_public(t.id, t.grower, t.carrier, t.opened);
            p.dispatch = Some(Dispatch {
                at: t.opened,
                route: t.route,
                shipment: t.shipment,
            });
            p.updated = t.opened;
        }
    }
    fn resolve(&mut self, r: Resolution) {
        let p = self
            .active_projects
            .get_mut(&r.team.id)
            .expect("project opening committed");
        assert!(p.outcome.replace(r).is_none());
        assert_eq!(p.team, r.team);
        for a in [r.team.grower, r.team.carrier] {
            let c = self.careers.entry(a).or_default();
            if r.result == ResultKind::Fulfilled {
                c.delivered += 1;
            } else {
                c.disrupted += 1;
            }
        }
        if r.result == ResultKind::CarrierLost {
            self.end_project(
                r.team.id,
                ProjectEnd {
                    at: r.at,
                    kind: MessageKind::Unreported,
                    holder: None,
                    place: None,
                },
            );
        }
    }
    fn end_project(&mut self, id: u32, end: ProjectEnd) {
        let mut p = self.active_projects.remove(&id).expect("active project");
        assert!(p.outcome.is_some() && p.end.is_none());
        p.end = Some(end);
        self.closed_projects += 1;
        for a in [p.team.grower, p.team.carrier] {
            let c = self.careers.entry(a).or_default();
            if end.kind == MessageKind::Unreported {
                c.unreported += 1;
            } else {
                c.confirmed += 1;
            }
        }
        bounded(&mut self.project_history, PROJECT_HISTORY_CAP, p);
    }
    fn receipt(&mut self, r: Witnessed, learned: u64) {
        if !r.delivered {
            return;
        }
        let p = self.project_public(r.id, r.grower, r.carrier, learned);
        // Re-reading a manifest must not refresh a known story.
        if p.delivery.is_none() {
            p.delivery = Some(Delivery {
                at: r.at,
                learned_at: learned,
                route: r.route,
                shipment: r.shipment,
                evidence: r.evidence,
            });
            p.updated = learned;
        }
    }
    fn report(&mut self, m: Message, seen: bool) {
        let p = self
            .active_projects
            .get_mut(&m.project)
            .expect("message has committed project");
        assert_eq!(
            p.outcome.expect("material resolution before report").result,
            m.result
        );
        assert_eq!((p.team.carrier, p.team.grower), (m.origin, m.recipient));
        if m.kind == MessageKind::Handoff {
            assert!(p.handoff.replace(m).is_none());
        } else {
            let spoken = matches!(m.kind, MessageKind::Direct | MessageKind::Relayed);
            self.end_project(
                m.project,
                ProjectEnd {
                    at: m.at,
                    kind: m.kind,
                    holder: spoken.then_some(m.holder),
                    place: (m.kind != MessageKind::Unreported).then_some(m.place),
                },
            );
        }
        if seen
            && matches!(
                m.kind,
                MessageKind::Handoff | MessageKind::Direct | MessageKind::Relayed
            )
        {
            let p = self.project_public(m.project, m.recipient, m.origin, m.at);
            if m.kind == MessageKind::Handoff {
                p.handoff = Some(m);
            } else {
                p.report = Some(m);
            }
            p.updated = m.at;
        }
    }
    fn conflict(v: View, at: u64) -> Option<Conflict> {
        if v.status != Status::Contested {
            return None;
        }
        let band = |scarce: bool| {
            let heard = if scarce { v.low } else { v.adequate };
            let direct = v.local.filter(|s| s.scarce == scarce);
            direct
                .into_iter()
                .chain(heard.map(|a| a.sample))
                .max_by_key(|s| (s.observed_at, s.phase, s.origin))
        };
        Some(Conflict {
            at,
            low: band(true)?,
            adequate: band(false)?,
        })
    }
    fn account(&mut self, actor: ActorId, a: Account, v: View, at: u64) {
        if !matches!(v.status, Status::Reported | Status::Contested) {
            return;
        }
        let key = (actor, a.sample.site);
        let conflict = Self::conflict(v, at);
        if let Some(i) = self.active_inquiries.get_mut(&key) {
            if i.conflict.is_none() {
                i.conflict = conflict;
            }
            return;
        }
        self.next_inquiry += 1;
        let career = self.careers.entry(actor).or_default();
        let id = career.inquiries;
        career.inquiries += 1;
        self.active_inquiries.insert(
            key,
            Inquiry {
                id,
                actor,
                site: a.sample.site,
                opened: at,
                first: a,
                conflict,
                end: None,
            },
        );
    }
    fn end_inquiry(&mut self, actor: ActorId, site: Site, end: InquiryEnd) {
        let Some(mut i) = self.active_inquiries.remove(&(actor, site)) else {
            return;
        };
        i.end = Some(end);
        let career = self.careers.entry(actor).or_default();
        match end {
            InquiryEnd::Checked(_) => {
                career.checked += 1;
                self.inquiries_checked += 1;
            }
            InquiryEnd::Lapsed(_) => {
                career.lapsed += 1;
                self.inquiries_lapsed += 1;
            }
        }
        if actor == PLAYER {
            bounded(&mut self.known_inquiries, KNOWN_CAP, i);
        }
        bounded(&mut self.inquiry_history, INQUIRY_HISTORY_CAP, i);
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        let mut w = Wire(b"SH:C55:NARRATIVE:v1\0".to_vec());
        w.u16(VERSION);
        for n in [
            self.projects,
            self.closed_projects,
            self.next_inquiry,
            self.inquiries_checked,
            self.inquiries_lapsed,
        ] {
            w.u32(n);
        }
        w.count(self.active_projects.len());
        for p in self.active_projects.values() {
            w.arc(*p);
        }
        w.count(self.project_history.len());
        for &p in &self.project_history {
            w.arc(p);
        }
        w.count(self.known_projects.len());
        for p in self.known_projects.values() {
            w.known(*p);
        }
        w.count(self.active_inquiries.len());
        for i in self.active_inquiries.values() {
            w.inquiry(*i);
        }
        for rows in [&self.inquiry_history, &self.known_inquiries] {
            w.count(rows.len());
            for &i in rows {
                w.inquiry(i);
            }
        }
        w.count(self.careers.len());
        for (&a, c) in &self.careers {
            w.actor(a);
            for n in [
                c.projects,
                c.delivered,
                c.disrupted,
                c.confirmed,
                c.unreported,
                c.inquiries,
                c.checked,
                c.lapsed,
            ] {
                w.u32(n);
            }
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
        self.u32(x.try_into().expect("bounded narrative"));
    }
    fn actor(&mut self, a: ActorId) {
        self.u32(a.get());
    }
    fn pos(&mut self, p: Position) {
        self.u32(p.x);
        self.u32(p.y);
    }
    fn opt<T>(&mut self, x: Option<T>, f: impl FnOnce(&mut Self, T)) {
        self.u8(u8::from(x.is_some()));
        if let Some(x) = x {
            f(self, x);
        }
    }
    fn team(&mut self, t: Team) {
        self.u32(t.id);
        self.u8(t.route as u8);
        self.actor(t.grower);
        self.actor(t.carrier);
        self.u64(t.opened);
        self.u64(t.deadline);
        self.u32(t.contributed);
        self.u32(t.shipment);
        self.u8(u8::from(t.opening_seen));
    }
    fn message(&mut self, m: Message) {
        self.u32(m.project);
        self.u64(m.at);
        self.actor(m.origin);
        self.actor(m.recipient);
        self.actor(m.holder);
        self.u8(m.kind as u8);
        self.u8(m.result as u8);
        self.pos(m.place);
    }
    fn arc(&mut self, p: ProjectArc) {
        self.team(p.team);
        self.opt(p.outcome, |w, r| {
            w.team(r.team);
            w.u64(r.at);
            w.pos(r.place);
            w.u8(r.result as u8);
        });
        self.opt(p.handoff, Self::message);
        self.opt(p.end, |w, e| {
            w.u64(e.at);
            w.u8(e.kind as u8);
            w.opt(e.holder, Self::actor);
            w.opt(e.place, Self::pos);
        });
    }
    fn known(&mut self, p: KnownProject) {
        self.u32(p.id);
        self.actor(p.grower);
        self.actor(p.carrier);
        self.opt(p.dispatch, |w, d| {
            w.u64(d.at);
            w.u8(d.route as u8);
            w.u32(d.shipment);
        });
        self.opt(p.delivery, |w, d| {
            w.u64(d.at);
            w.u64(d.learned_at);
            w.u8(d.route as u8);
            w.u32(d.shipment);
            w.u8(d.evidence as u8);
        });
        self.opt(p.handoff, Self::message);
        self.opt(p.report, Self::message);
        self.u64(p.updated);
    }
    fn sample(&mut self, s: Sample) {
        self.u8(s.site as u8);
        self.u8(u8::from(s.scarce));
        self.actor(s.origin);
        self.u64(s.observed_at);
        self.u8(s.phase);
    }
    fn account(&mut self, a: Account) {
        self.sample(a.sample);
        self.opt(a.via, Self::actor);
        self.u8(a.hops);
        self.u64(a.acquired_at);
        self.u8(a.credibility);
    }
    fn inquiry(&mut self, i: Inquiry) {
        self.u32(i.id);
        self.actor(i.actor);
        self.u8(i.site as u8);
        self.u64(i.opened);
        self.account(i.first);
        self.opt(i.conflict, |w, c| {
            w.u64(c.at);
            w.sample(c.low);
            w.sample(c.adequate);
        });
        self.opt(i.end, |w, e| match e {
            InquiryEnd::Checked(s) => {
                w.u8(0);
                w.sample(s);
            }
            InquiryEnd::Lapsed(t) => {
                w.u8(1);
                w.u64(t);
            }
        });
    }
}
impl World {
    pub fn narrative(&self) -> Option<&Narrative> {
        self.narrative.as_ref()
    }
    pub fn known_story_projects(&self) -> impl Iterator<Item = &KnownProject> {
        self.narrative()
            .into_iter()
            .flat_map(|n| n.known_projects.values())
    }
    pub fn known_story_inquiries(&self) -> impl Iterator<Item = &Inquiry> {
        self.narrative().into_iter().flat_map(|n| {
            n.active_inquiries
                .values()
                .filter(|i| i.actor == PLAYER)
                .chain(n.known_inquiries.iter())
        })
    }
    pub(super) fn story_open(&mut self, t: Team) {
        if let Some(n) = self.narrative.as_mut() {
            n.open(t);
        }
    }
    pub(super) fn story_resolve(&mut self, r: Resolution) {
        if self.narrative.is_none() {
            return;
        }
        let receipt = self
            .known_cooperation()
            .find(|p| p.id == r.team.id && p.delivered && p.evidence == Evidence::Witnessed)
            .copied();
        let n = self.narrative.as_mut().unwrap();
        n.resolve(r);
        if let Some(p) = receipt {
            n.receipt(p, self.tick);
        }
        self.learn_own_resolution(r);
    }
    pub(super) fn story_register(&mut self, r: Witnessed) {
        if let Some(n) = self.narrative.as_mut() {
            n.receipt(r, self.tick);
        }
    }
    pub(super) fn story_report(&mut self, m: Message, seen: bool) {
        let seen = seen && self.player.alive();
        if let Some(n) = self.narrative.as_mut() {
            n.report(m, seen);
        }
        self.learn_reported_resolution(m);
    }
    pub(super) fn story_account(&mut self, actor: ActorId, a: Account) {
        if self.narrative.is_none() {
            return;
        }
        let v = self.rumors().unwrap().view(actor, a.sample.site, self.tick);
        self.narrative
            .as_mut()
            .unwrap()
            .account(actor, a, v, self.tick);
    }
    pub(super) fn story_check(&mut self, actor: ActorId, s: Sample) {
        if let Some(i) = self
            .narrative()
            .and_then(|n| n.active_inquiries().get(&(actor, s.site)))
            .copied()
        {
            self.learn_inquiry_end(i, InquiryEnd::Checked(s));
        }
        if let Some(n) = self.narrative.as_mut() {
            n.end_inquiry(actor, s.site, InquiryEnd::Checked(s));
        }
    }
    pub(super) fn expire_inquiries(&mut self) {
        let Some(n) = self.narrative() else {
            return;
        };
        let stale: Vec<_> = n
            .active_inquiries
            .keys()
            .copied()
            .filter(|&(a, s)| {
                let alive = if a == PLAYER {
                    self.player.alive()
                } else {
                    self.npcs.get(&a).is_some_and(|n| n.alive())
                };
                !alive
                    || !matches!(
                        self.rumors().unwrap().view(a, s, self.tick).status,
                        Status::Reported | Status::Contested
                    )
            })
            .collect();
        for (a, s) in stale {
            let i = self.narrative().unwrap().active_inquiries()[&(a, s)];
            self.learn_inquiry_end(i, InquiryEnd::Lapsed(self.tick));
            self.narrative
                .as_mut()
                .unwrap()
                .end_inquiry(a, s, InquiryEnd::Lapsed(self.tick));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Intent, Rules};
    const A: ActorId = ActorId::new(31);
    const B: ActorId = ActorId::new(32);
    const S: Site = Site::WillowfordMarket;
    fn team(id: u32, seen: bool) -> Team {
        Team {
            id,
            route: Route::Qinghe,
            grower: A,
            carrier: B,
            opened: 10,
            deadline: 250,
            contributed: 2,
            shipment: 2,
            opening_seen: seen,
        }
    }
    fn account(origin: ActorId, at: u64) -> Account {
        Account {
            sample: Sample {
                site: S,
                scarce: true,
                origin,
                observed_at: at,
                phase: 0,
            },
            via: Some(origin),
            hops: 1,
            acquired_at: at + 1,
            credibility: 67,
        }
    }
    fn view(a: Account) -> View {
        View {
            status: Status::Reported,
            local: None,
            low: Some(a),
            adequate: None,
        }
    }
    #[test]
    fn report_only_knowledge_never_backfills_hidden_facts_and_private_history_cannot_evict_it() {
        let mut n = Narrative::default();
        let t = team(0, false);
        n.open(t);
        let r = Resolution {
            team: t,
            at: 12,
            place: Position { x: 4, y: 4 },
            result: ResultKind::Fulfilled,
        };
        n.resolve(r);
        let handoff = Message {
            project: 0,
            at: 20,
            origin: B,
            recipient: A,
            holder: ActorId::new(2),
            kind: MessageKind::Handoff,
            result: r.result,
            place: Position { x: 5, y: 5 },
        };
        n.report(handoff, false);
        assert!(n.known_projects().is_empty());
        let relay = Message {
            kind: MessageKind::Relayed,
            at: 25,
            ..handoff
        };
        n.report(relay, true);
        let k = n.known_projects()[&0];
        assert!(k.dispatch.is_none() && k.delivery.is_none() && k.handoff.is_none());
        assert_eq!(k.report, Some(relay));
        assert_eq!(n.project_history()[0].handoff, Some(handoff));
        assert_eq!(
            n.project_history()[0].end.unwrap().holder,
            Some(relay.holder)
        );
        for id in 1..=200 {
            let t = team(id, false);
            n.open(t);
            n.resolve(Resolution {
                team: t,
                at: 12,
                place: r.place,
                result: ResultKind::CarrierLost,
            });
        }
        assert_eq!(n.project_history().len(), PROJECT_HISTORY_CAP);
        assert_eq!(n.known_projects().len(), 1);
        assert_eq!(n.known_projects()[&0], k);
        assert_eq!(n.career(A).projects, 201);
        assert_eq!(n.career(A).confirmed, 1);
        assert_eq!(n.career(A).unreported, 200);
        assert_eq!(n.counts().0, n.counts().1);
    }

    #[test]
    fn inquiry_lapse_and_public_history_have_separate_budgets_and_local_ids() {
        let mut n = Narrative::default();
        let a = account(A, 10);
        n.account(PLAYER, a, view(a), 11);
        n.end_inquiry(PLAYER, S, InquiryEnd::Lapsed(200));
        let known = n.known_inquiries()[0];
        assert_eq!(known.id, 0);
        assert_eq!(known.end, Some(InquiryEnd::Lapsed(200)));
        for i in 0..200 {
            let a = account(A, 201 + i * 3);
            let now = a.acquired_at;
            n.account(B, a, view(a), now);
            n.end_inquiry(B, S, InquiryEnd::Lapsed(now + 1));
        }
        assert_eq!(n.inquiry_history().len(), INQUIRY_HISTORY_CAP);
        assert_eq!(n.known_inquiries().len(), 1);
        assert_eq!(n.known_inquiries()[0], known);
        let a = account(B, 900);
        n.account(PLAYER, a, view(a), 901);
        let p = n.active_inquiries()[&(PLAYER, S)];
        assert_eq!(p.id, 1);
        assert_eq!(n.career(PLAYER).checked, 0);
        assert_eq!(n.career(PLAYER).lapsed, 1);
        for i in 0..60 {
            let at = 910 + i * 3;
            n.end_inquiry(PLAYER, S, InquiryEnd::Lapsed(at));
            let a = account(B, at);
            n.account(PLAYER, a, view(a), at + 1);
        }
        assert_eq!(n.known_inquiries().len(), KNOWN_CAP);
        assert_eq!(n.active_inquiries().len(), 1);
    }

    #[test]
    fn observational_layer_leaves_c54_simulation_identical_and_replays() {
        let mut total = (0, 0, 0, 0, 0);
        for seed in [0, 3, 7, 42] {
            let mut w = World::narrative_campaign(seed);
            let mut baseline = World::rumor_campaign(seed);
            for _ in 0..2000 {
                assert_eq!(
                    w.step(w.tick(), Intent::Wait),
                    baseline.step(baseline.tick(), Intent::Wait)
                );
                let n = w.narrative.take();
                w.rules = Rules::InformationNetworks;
                assert_eq!(w, baseline, "seed {seed} tick {}", w.tick());
                w.rules = Rules::CausalNarratives;
                w.narrative = n;
            }
            let restored = World::restore(&w.checkpoint()).unwrap();
            assert_eq!(w, restored);
            let n = w.narrative().unwrap();
            let counts = n.counts();
            println!("seed {seed}: {counts:?}");
            total.0 += counts.0;
            total.1 += counts.1;
            total.2 += counts.2;
            total.3 += counts.3;
            total.4 += counts.4;
            assert_eq!(counts.0, counts.1 + n.active_projects().len() as u32);
            assert_eq!(
                counts.2,
                counts.3 + counts.4 + n.active_inquiries().len() as u32
            );
            assert_eq!(counts.0, w.obligations().unwrap().counts().0);
            assert_eq!(
                counts.1,
                w.obligations().unwrap().counts().2 + w.obligations().unwrap().counts().3
            );
        }
        println!("total {total:?}");
        assert!(total.0 > 0 && total.1 > 0 && total.2 > 0 && total.3 > 0 && total.4 > 0);
    }
}
