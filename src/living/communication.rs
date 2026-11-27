//! C5.3: bounded, physically situated reporting of actual project outcomes.
//! No remote tracking, new goods, rumor fabrication or transitive trust.
use super::{
    ActorId, Goal, Npc, Position, Region, World,
    cooperation::Route,
    expansion,
    navigation::in_reach,
    obligations::{REPORT_LIFETIME, Resolution, ResultKind},
};
use std::collections::{BTreeMap, VecDeque};

pub const REVISION: u16 = 40;
pub const VERSION: u16 = 1;
pub const SEARCH_BUDGET: u16 = 64;
pub const RETRY_WAIT: u64 = 24;
pub const HISTORY_CAP: usize = 128;
pub const OBSERVED_CAP: usize = 48;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Message {
    pub project: u32,
    pub origin: ActorId,
    pub recipient: ActorId,
    pub holder: ActorId,
    pub created: u64,
    pub site: u8,
    pub dwell: u8,
    pub spent: u16,
    pub effort: u16,
    pub retry_at: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Kind {
    Handoff,
    Direct,
    Relayed,
    Register,
    Unreported,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Event {
    pub project: u32,
    pub at: u64,
    pub origin: ActorId,
    pub recipient: ActorId,
    pub holder: ActorId,
    pub kind: Kind,
    pub result: ResultKind,
    pub place: Position,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Communication {
    messages: BTreeMap<ActorId, Message>,
    history: VecDeque<Event>,
    observed: VecDeque<Event>,
    created: u32,
    delivered: u32,
    unreported: u32,
    handoffs: u32,
}
impl Communication {
    pub fn messages(&self) -> &BTreeMap<ActorId, Message> {
        &self.messages
    }
    pub fn history(&self) -> &VecDeque<Event> {
        &self.history
    }
    pub fn observed(&self) -> &VecDeque<Event> {
        &self.observed
    }
    pub fn counts(&self) -> (u32, u32, u32, u32) {
        (self.created, self.delivered, self.unreported, self.handoffs)
    }
    fn by_holder(&self, id: ActorId) -> Option<&Message> {
        self.messages.values().find(|m| m.holder == id)
    }
    fn event(&mut self, e: Event, seen: bool) {
        if self.history.len() == HISTORY_CAP {
            self.history.pop_front();
        }
        self.history.push_back(e);
        if seen {
            if self.observed.len() == OBSERVED_CAP {
                self.observed.pop_front();
            }
            self.observed.push_back(e);
        }
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = b"SH:C53:COMMUNICATION:v1\0".to_vec();
        out.extend(VERSION.to_le_bytes());
        for n in [
            self.created,
            self.delivered,
            self.unreported,
            self.handoffs,
            self.messages.len() as u32,
        ] {
            out.extend(n.to_le_bytes());
        }
        for m in self.messages.values() {
            out.extend(m.project.to_le_bytes());
            for id in [m.origin, m.recipient, m.holder] {
                out.extend(id.get().to_le_bytes());
            }
            out.extend(m.created.to_le_bytes());
            out.push(m.site);
            out.push(m.dwell);
            out.extend(m.spent.to_le_bytes());
            out.extend(m.effort.to_le_bytes());
            out.extend(m.retry_at.to_le_bytes());
        }
        for events in [&self.history, &self.observed] {
            out.extend((events.len() as u32).to_le_bytes());
            for e in events {
                out.extend(e.project.to_le_bytes());
                out.extend(e.at.to_le_bytes());
                for id in [e.origin, e.recipient, e.holder] {
                    out.extend(id.get().to_le_bytes());
                }
                out.push(e.kind as u8);
                out.push(e.result as u8);
                out.extend(e.place.x.to_le_bytes());
                out.extend(e.place.y.to_le_bytes());
            }
        }
        out
    }
}
impl World {
    pub fn communication(&self) -> Option<&Communication> {
        self.communication.as_ref()
    }
    pub fn witnessed_messages(&self) -> impl DoubleEndedIterator<Item = &Event> {
        self.communication()
            .into_iter()
            .flat_map(|c| c.observed.iter())
    }
    pub(super) fn open_message(&mut self, r: Resolution) {
        let Some(c) = self.communication.as_mut() else {
            return;
        };
        if r.result == ResultKind::CarrierLost {
            return;
        }
        let t = r.team;
        assert!(
            c.messages
                .insert(
                    t.carrier,
                    Message {
                        project: t.id,
                        origin: t.carrier,
                        recipient: t.grower,
                        holder: t.carrier,
                        created: r.at,
                        site: 0,
                        dwell: 0,
                        spent: 0,
                        effort: 0,
                        retry_at: 0
                    }
                )
                .is_none()
        );
        c.created += 1;
    }
    pub(super) fn finish_message(
        &mut self,
        r: Resolution,
        kind: Kind,
        holder: ActorId,
        place: Position,
        seen: bool,
    ) {
        let Some(c) = self.communication.as_mut() else {
            return;
        };
        let Some(m) = c.messages.remove(&r.team.carrier) else {
            return;
        };
        assert_eq!(m.project, r.team.id);
        let holder = if kind == Kind::Unreported {
            m.holder
        } else {
            holder
        };
        if kind == Kind::Unreported {
            c.unreported += 1;
        } else {
            c.delivered += 1;
        }
        let event = Event {
            project: m.project,
            at: self.tick,
            origin: m.origin,
            recipient: m.recipient,
            holder,
            kind,
            result: r.result,
            place,
        };
        c.event(event, seen);
        self.story_report(event, seen);
    }
    fn eligible_messenger(&self, n: &Npc) -> bool {
        n.alive()
            && n.hp == n.definition.hp
            && !self.injuries().unwrap().people[&n.id()].injured()
            && n.herbs == 0
            && self.companion(n.id()).is_none()
            && !self.sect_duty_active(n.id())
            && !self.hobbled(n.id())
    }
    fn message_for(&self, n: &Npc) -> Option<(&Message, Resolution)> {
        let m = self.communication()?.by_holder(n.id())?;
        let r = *self.obligations()?.pending().get(&m.origin)?;
        (r.team.id == m.project && self.tick.saturating_sub(r.at) < REPORT_LIFETIME)
            .then_some((m, r))
    }
    pub(super) fn message_plan(&self, n: &Npc) -> Option<(u16, Goal)> {
        let (m, r) = self.message_for(n)?;
        if !self.eligible_messenger(n)
            || n.fatigue >= n.definition.rest_threshold
            || self.tick < m.retry_at
            || self.social.relationship(n.id(), m.recipient).refuses_aid()
        {
            return None;
        }
        // Urgent disruption/near-expiry reports outrank routine work only.
        let urgent = r.result != ResultKind::Fulfilled || self.tick.saturating_sub(r.at) >= 360;
        Some((if urgent { 50 } else { 45 }, Goal::CarryMessage(m.project)))
    }
    pub(super) fn message_camp(&self, n: &Npc) -> bool {
        self.message_for(n).is_some()
            && self.eligible_messenger(n)
            && n.position != n.definition.home
            && n.fatigue >= n.definition.rest_threshold
    }
    fn message_target(&self, n: &Npc, m: &Message, r: Resolution) -> Position {
        let peer = &self.npcs[&m.recipient];
        if self.perceives(n.position, peer.position) {
            return peer.position;
        }
        // Public workplaces, not hidden live position or chosen goal.
        let field = match r.team.route {
            Route::Qinghe => Region::sites().garden,
            Route::Willowford => expansion::GROVE,
        };
        [r.team.route.hub(), field, peer.definition.home][usize::from(m.site)]
    }
    pub(super) fn work_message(&mut self, id: ActorId, project: u32) {
        let n = self.npcs[&id];
        let Some((m, r)) = self.message_for(&n) else {
            return;
        };
        let m = *m;
        if m.project != project || !self.eligible_messenger(&n) || self.tick < m.retry_at {
            return;
        }
        let target = self.message_target(&n, &m, r);
        let peer_visible = self.perceives(n.position, self.npcs[&m.recipient].position);
        let reached = in_reach(n.position, target) && self.can_see(n.position, target);
        if !reached {
            self.travel(id, target, 1);
        }
        let search = self
            .communication
            .as_mut()
            .unwrap()
            .messages
            .get_mut(&m.origin)
            .unwrap();
        search.spent += 1;
        search.effort += 1;
        if reached && !peer_visible {
            search.dwell += 1;
            if search.dwell >= 2 {
                search.dwell = 0;
                search.site = (search.site + 1) % 3;
            }
        } else {
            search.dwell = 0;
        }
        if search.spent >= SEARCH_BUDGET {
            search.spent = 0;
            search.dwell = 0;
            search.site = (search.site + 1) % 3;
            search.retry_at = self.tick + RETRY_WAIT;
        }
    }
    pub(super) fn has_relay(&self, origin: ActorId) -> bool {
        self.communication()
            .and_then(|c| c.messages.get(&origin))
            .is_some_and(|m| m.holder != origin)
    }
    pub(super) fn relay_delivery(&self, r: Resolution) -> Option<ActorId> {
        let m = self.communication()?.messages.get(&r.team.carrier)?;
        if m.holder == m.origin {
            return None;
        }
        let h = &self.npcs[&m.holder];
        let g = &self.npcs[&m.recipient];
        (h.alive()
            && g.alive()
            && in_reach(h.position, g.position)
            && self.can_see(h.position, g.position)
            && self.social.relationship(g.id(), h.id()).trusts_news()
            && !self.social.relationship(h.id(), g.id()).refuses_aid()
            && !matches!(h.activity.goal, Goal::Retreat | Goal::Retaliate)
            && !matches!(g.activity.goal, Goal::Retreat | Goal::Retaliate))
        .then_some(h.id())
    }
    /// A single voluntary handoff after a bounded failed search. Both origin and
    /// messenger must actually meet. The original can still report in person;
    /// acknowledgement removes the copy exactly once. No second handoff.
    pub(super) fn delegate_messages(&mut self) {
        let Some(c) = self.communication() else {
            return;
        };
        let eligible = c
            .messages
            .values()
            .filter(|m| m.origin == m.holder && m.effort >= SEARCH_BUDGET && self.tick < m.retry_at)
            .copied()
            .collect::<Vec<_>>();
        for m in eligible {
            let n = &self.npcs[&m.origin];
            if !n.alive() || matches!(n.activity.goal, Goal::Retreat | Goal::Retaliate) {
                continue;
            }
            let Some(r) = self
                .obligations()
                .unwrap()
                .pending()
                .get(&m.origin)
                .copied()
            else {
                continue;
            };
            let next = self
                .npcs
                .values()
                .find(|peer| {
                    let id = peer.id();
                    id != m.origin
                        && id != m.recipient
                        && peer.definition.shares_news
                        && self.eligible_messenger(peer)
                        && peer.fatigue < peer.definition.rest_threshold
                        && matches!(peer.activity.goal, Goal::Idle | Goal::Patrol)
                        && in_reach(n.position, peer.position)
                        && self.can_see(n.position, peer.position)
                        && self.social.relationship(m.origin, id).trusts_news()
                        && self.social.relationship(id, m.origin).trusts_news()
                        && self.social.relationship(id, m.recipient).trusts_news()
                        && self.communication().unwrap().by_holder(id).is_none()
                        && !self.communication().unwrap().messages.contains_key(&id)
                        && !self
                            .cooperation()
                            .unwrap()
                            .active()
                            .values()
                            .any(|t| t.grower == id || t.carrier == id)
                })
                .map(Npc::id);
            let Some(holder) = next else {
                continue;
            };
            let place = n.position;
            let seen = self.player_sees(place) && self.player_sees(self.npcs[&holder].position);
            let c = self.communication.as_mut().unwrap();
            let msg = c.messages.get_mut(&m.origin).unwrap();
            msg.holder = holder;
            msg.spent = 0;
            msg.site = 0;
            msg.dwell = 0;
            msg.retry_at = 0;
            c.handoffs += 1;
            let event = Event {
                project: m.project,
                at: self.tick,
                origin: m.origin,
                recipient: m.recipient,
                holder,
                kind: Kind::Handoff,
                result: r.result,
                place,
            };
            c.event(event, seen);
            self.story_report(event, seen);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{
        DIRECTIONS, Role,
        cooperation::{Outcome, Team},
        navigation,
    };

    fn actors(w: &World) -> (ActorId, ActorId, ActorId) {
        let a = w
            .npcs()
            .find(|n| n.definition.role == Role::Grower)
            .unwrap()
            .id();
        let b = w
            .npcs()
            .find(|n| n.definition.role == Role::Courier)
            .unwrap()
            .id();
        let h = w
            .npcs()
            .find(|n| {
                n.id() != a
                    && n.id() != b
                    && n.definition.shares_news
                    && n.definition.role == Role::Watch
            })
            .unwrap()
            .id();
        (a, b, h)
    }
    fn fixture(outcome: Outcome) -> (World, ActorId, ActorId, ActorId, Resolution) {
        let mut w = World::communication_campaign(3);
        let (a, b, h) = actors(&w);
        w.tick = 10;
        let hub = Route::Qinghe.hub();
        let adjacent = DIRECTIONS
            .into_iter()
            .filter_map(|d| navigation::moved(hub, d))
            .find(|&p| w.walkable(p))
            .unwrap();
        w.npcs.get_mut(&b).unwrap().position = hub;
        w.npcs.get_mut(&h).unwrap().position = adjacent;
        w.npcs.get_mut(&h).unwrap().activity.goal = Goal::Idle;
        w.npcs.get_mut(&a).unwrap().position = w.scenario.shrine;
        w.player.position = hub;
        let team = Team {
            id: 0,
            route: Route::Qinghe,
            grower: a,
            carrier: b,
            opened: 1,
            deadline: 241,
            contributed: 2,
            shipment: 2,
            opening_seen: false,
        };
        w.open_obligation(team);
        w.close_obligation(team, outcome, hub);
        let r = w.obligations().unwrap().pending()[&b];
        (w, a, b, h, r)
    }

    #[test]
    fn hidden_recipient_position_is_not_used_and_search_budget_yields() {
        let (mut w, a, b, _, r) = fixture(Outcome::CargoChanged);
        let m = w.communication().unwrap().messages()[&b];
        assert!(!w.perceives(w.npcs[&b].position, w.npcs[&a].position));
        let target = w.message_target(&w.npcs[&b], &m, r);
        let far = w.scenario.patch;
        w.npcs.get_mut(&a).unwrap().position = far;
        assert!(!w.perceives(w.npcs[&b].position, far));
        assert_eq!(target, w.message_target(&w.npcs[&b], &m, r));
        assert_eq!(target, Route::Qinghe.hub());
        {
            let s = w
                .communication
                .as_mut()
                .unwrap()
                .messages
                .get_mut(&b)
                .unwrap();
            s.spent = SEARCH_BUDGET - 1;
            s.effort = SEARCH_BUDGET - 1;
        }
        w.work_message(b, m.project);
        let s = w.communication().unwrap().messages()[&b];
        assert_eq!(
            (s.spent, s.effort, s.retry_at),
            (0, SEARCH_BUDGET, w.tick + RETRY_WAIT)
        );
        assert!(w.message_plan(&w.npcs[&b]).is_none());
        w.tick = s.retry_at;
        assert!(w.message_plan(&w.npcs[&b]).is_some());
    }

    #[test]
    fn trusted_single_handoff_is_physical_and_relay_acknowledges_original_once() {
        let (mut w, a, b, h, r) = fixture(Outcome::Delivered);
        let money = w.character_agency().unwrap().clone();
        let supply = w.supply;
        {
            let m = w
                .communication
                .as_mut()
                .unwrap()
                .messages
                .get_mut(&b)
                .unwrap();
            m.effort = SEARCH_BUDGET;
            m.retry_at = w.tick + RETRY_WAIT;
        }
        for (x, y) in [(b, h), (h, b), (h, a)] {
            w.social.meet(x, y);
        }
        let before = w.communication().unwrap().clone();
        // Moving the candidate far away never delegates by remote knowledge.
        let p = w.npcs[&h].position;
        w.npcs.get_mut(&h).unwrap().position = w.scenario.patch;
        w.delegate_messages();
        assert_eq!(w.communication().unwrap(), &before);
        w.npcs.get_mut(&h).unwrap().position = p;
        w.delegate_messages();
        assert_eq!(w.communication().unwrap().messages()[&b].holder, h);
        assert_eq!(w.communication().unwrap().counts(), (1, 0, 0, 1));
        assert_eq!(w.witnessed_messages().last().unwrap().kind, Kind::Handoff);
        let snapshot = w.clone();
        w.delegate_messages();
        assert_eq!(w, snapshot);
        assert!(w.message_plan(&w.npcs[&h]).is_some());
        assert!(w.message_plan(&w.npcs[&b]).is_none());
        assert!(w.relay_delivery(r).is_none());
        let target = w.npcs[&a].position;
        let adj = DIRECTIONS
            .into_iter()
            .filter_map(|d| navigation::moved(target, d))
            .find(|&p| w.walkable(p))
            .unwrap();
        w.npcs.get_mut(&h).unwrap().position = adj;
        // Recipient refusal prevents acknowledgement, despite physical meeting.
        w.social.relations.get_mut(&(a, h)).unwrap().regard = -1;
        w.advance_obligations();
        assert!(w.relay_delivery(r).is_none());
        assert_eq!(w.obligations().unwrap().counts(), (1, 1, 0, 0));
        w.social.relations.get_mut(&(a, h)).unwrap().regard = 0;
        w.player.position = target;
        let original = w.social.relationship(a, b).regard;
        let intermediary = w.social.relationship(a, h);
        w.advance_obligations();
        assert_eq!(w.obligations().unwrap().counts(), (1, 1, 1, 0));
        assert_eq!(w.communication().unwrap().counts(), (1, 1, 0, 1));
        assert_eq!(w.social.relationship(a, b).regard, (original + 1).min(8));
        assert_eq!(w.social.relationship(a, h), intermediary);
        assert_eq!(w.witnessed_obligations().count(), 0);
        let e = w.witnessed_messages().last().unwrap();
        assert_eq!(
            (e.kind, e.holder, e.origin, e.recipient),
            (Kind::Relayed, h, b, a)
        );
        assert_eq!(w.supply, supply);
        assert_eq!(w.character_agency().unwrap(), &money);
        let snapshot = w.clone();
        w.advance_obligations();
        assert_eq!(w, snapshot);
    }

    #[test]
    fn expired_unseen_report_never_penalizes_or_leaks() {
        let (mut w, a, b, _, r) = fixture(Outcome::CargoChanged);
        let relation = w.social.relationship(a, b);
        w.tick = r.at + REPORT_LIFETIME;
        w.advance_obligations();
        assert_eq!(w.communication().unwrap().counts(), (1, 0, 1, 0));
        assert_eq!(
            w.communication().unwrap().history().back().unwrap().kind,
            Kind::Unreported
        );
        assert!(w.communication().unwrap().messages().is_empty());
        assert_eq!(w.witnessed_messages().count(), 0);
        assert_eq!(w.social.relationship(a, b), relation);
    }
}
