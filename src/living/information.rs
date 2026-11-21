//! Revision-thirteen institutional information. Audit findings become named
//! events with provenance. Living actors may carry those findings face-to-face;
//! distant actors do not learn them from global state. Credible accusations can
//! open bounded institutional disputes that alter later contract awards.

use std::collections::{BTreeMap, btree_map::Entry};

use super::contract_memory::AuditFinding;
use super::navigation::in_reach;
use super::{ActorId, Goal, Need, Position, World};

const REPORT_DELAY: u64 = 2;
const MAX_HOPS: u8 = 2;
/// Institutional findings remain relevant across contract timescales.
const NEWS_LIFETIME: u64 = 640;
const EVENT_CAP: usize = 96;
const PENDING_CAP: usize = 64;
const DISPUTE_CAP: usize = 32;
const HISTORY_CAP: usize = 160;
const KNOWLEDGE_PER_ACTOR: usize = 48;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EventId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InfoEvent {
    pub id: EventId,
    pub tick: u64,
    pub case_id: u32,
    pub need: Need,
    pub requester: ActorId,
    pub provider: ActorId,
    pub finding: AuditFinding,
    pub site: Position,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Provenance {
    Firsthand {
        observer: ActorId,
    },
    Reported {
        sender: ActorId,
        origin: ActorId,
        hops: u8,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InfoKnowledge {
    pub acquired: u64,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InfoReport {
    pub event: EventId,
    pub sender: ActorId,
    pub recipient: ActorId,
    pub origin: ActorId,
    pub hops: u8,
    pub sent: u64,
    pub due: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisputePhase {
    Open,
    Investigating,
    Confirmed,
    Refuted,
    Suspended,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Dispute {
    pub id: u32,
    pub opened: u64,
    pub institution: ActorId,
    pub subject: ActorId,
    pub event: EventId,
    pub need: Need,
    pub phase: DisputePhase,
    pub resolved_at: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InfoLogKind {
    Learned,
    DisputeOpened,
    DisputeUpdated,
    TestimonyQueued,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InfoLogEvent {
    pub at: u64,
    pub kind: InfoLogKind,
    pub actor: ActorId,
    pub other: Option<ActorId>,
    pub event: EventId,
    pub dispute: Option<u32>,
    pub phase: Option<DisputePhase>,
    pub witnessed: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Information {
    pub events: Vec<InfoEvent>,
    pub known: BTreeMap<ActorId, BTreeMap<EventId, InfoKnowledge>>,
    pub pending: BTreeMap<(ActorId, EventId), InfoReport>,
    pub disputes: Vec<Dispute>,
    pub history: Vec<InfoLogEvent>,
    pub next_dispute: u32,
}

impl Information {
    pub fn event(&self, id: EventId) -> Option<&InfoEvent> {
        self.events.get(id.0 as usize)
    }

    pub fn knowledge(&self, actor: ActorId, event: EventId) -> Option<InfoKnowledge> {
        self.known.get(&actor)?.get(&event).copied()
    }

    pub fn pending_reports(&self) -> impl ExactSizeIterator<Item = &InfoReport> {
        self.pending.values()
    }

    pub fn disputes(&self) -> &[Dispute] {
        &self.disputes
    }
}

impl World {
    pub fn information(&self) -> Option<&Information> {
        self.information.as_ref()
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn record_audit_information(
        &mut self,
        case_id: u32,
        need: Need,
        requester: ActorId,
        provider: ActorId,
        finding: AuditFinding,
        site: Position,
        witnessed: bool,
    ) {
        if self.information.is_none() {
            return;
        }
        let id = EventId(self.information.as_ref().unwrap().events.len() as u32);
        let event = InfoEvent {
            id,
            tick: self.tick,
            case_id,
            need,
            requester,
            provider,
            finding,
            site,
        };
        {
            let info = self.information.as_mut().unwrap();
            // Event identities are index-stable. When the ledger is full, refuse
            // new findings rather than renumbering or silently rewriting history.
            if info.events.len() >= EVENT_CAP {
                return;
            }
            info.events.push(event);
        }
        self.learn_information(
            requester,
            event,
            Provenance::Firsthand {
                observer: requester,
            },
            witnessed,
        );
    }

    pub(super) fn learn_information(
        &mut self,
        actor: ActorId,
        event: InfoEvent,
        provenance: Provenance,
        witnessed: bool,
    ) {
        {
            let Some(info) = self.information.as_mut() else {
                return;
            };
            info.pending.remove(&(actor, event.id));
            if info.knowledge(actor, event.id).is_some() {
                return;
            }
            let known = info.known.entry(actor).or_default();
            if known.len() == KNOWLEDGE_PER_ACTOR
                && let Some((&oldest, _)) = known.iter().next()
            {
                known.remove(&oldest);
            }
            known.insert(
                event.id,
                InfoKnowledge {
                    acquired: self.tick,
                    provenance,
                },
            );
            let other = match provenance {
                Provenance::Firsthand { observer } => Some(observer),
                Provenance::Reported { sender, .. } => Some(sender),
            };
            push_history(
                info,
                InfoLogEvent {
                    at: self.tick,
                    kind: InfoLogKind::Learned,
                    actor,
                    other,
                    event: event.id,
                    dispute: None,
                    phase: None,
                    witnessed,
                },
            );
        }
        self.apply_information_consequences(actor, event, provenance);
    }

    fn apply_information_consequences(
        &mut self,
        actor: ActorId,
        event: InfoEvent,
        provenance: Provenance,
    ) {
        if !is_institutional_requester(actor) {
            return;
        }
        match event.finding {
            AuditFinding::Unfulfilled => {
                self.open_or_reinforce_dispute(actor, event, provenance);
            }
            AuditFinding::Delivered => {
                self.refute_disputes_with_delivery(actor, event, provenance);
            }
        }
    }

    fn open_or_reinforce_dispute(
        &mut self,
        institution: ActorId,
        event: InfoEvent,
        provenance: Provenance,
    ) {
        let Some(info) = self.information.as_mut() else {
            return;
        };
        if let Some(existing) = info
            .disputes
            .iter_mut()
            .find(|dispute| dispute.institution == institution && dispute.event == event.id)
        {
            // Repeated delivery of the same finding never multiplies effects.
            let _ = existing;
            return;
        }
        if info.disputes.iter().any(|dispute| {
            dispute.institution == institution
                && dispute.subject == event.provider
                && dispute.need == event.need
                && matches!(
                    dispute.phase,
                    DisputePhase::Open
                        | DisputePhase::Investigating
                        | DisputePhase::Confirmed
                        | DisputePhase::Suspended
                )
        }) {
            return;
        }
        if info.disputes.len() == DISPUTE_CAP {
            if let Some(index) = info.disputes.iter().position(|dispute| {
                matches!(
                    dispute.phase,
                    DisputePhase::Confirmed | DisputePhase::Refuted | DisputePhase::Suspended
                )
            }) {
                info.disputes.remove(index);
            } else {
                return;
            }
        }
        let phase = match provenance {
            Provenance::Firsthand { .. } => DisputePhase::Suspended,
            Provenance::Reported { hops: 1, .. } => DisputePhase::Open,
            // Hop-2+ allegations remain as knowledge only. They do not open
            // institutional disputes until a stronger report arrives.
            Provenance::Reported { .. } => return,
        };
        info.next_dispute += 1;
        let id = info.next_dispute;
        let dispute = Dispute {
            id,
            opened: self.tick,
            institution,
            subject: event.provider,
            event: event.id,
            need: event.need,
            phase,
            resolved_at: matches!(phase, DisputePhase::Suspended).then_some(self.tick),
        };
        info.disputes.push(dispute);
        push_history(
            info,
            InfoLogEvent {
                at: self.tick,
                kind: InfoLogKind::DisputeOpened,
                actor: institution,
                other: Some(event.provider),
                event: event.id,
                dispute: Some(id),
                phase: Some(phase),
                witnessed: false,
            },
        );
    }

    fn refute_disputes_with_delivery(
        &mut self,
        institution: ActorId,
        event: InfoEvent,
        provenance: Provenance,
    ) {
        let credible = matches!(
            provenance,
            Provenance::Firsthand { .. } | Provenance::Reported { hops: 1, .. }
        );
        if !credible {
            return;
        }
        let Some(info) = self.information.as_mut() else {
            return;
        };
        let mut updates = Vec::new();
        for dispute in &mut info.disputes {
            if dispute.institution == institution
                && dispute.subject == event.provider
                && dispute.need == event.need
                && matches!(
                    dispute.phase,
                    DisputePhase::Open | DisputePhase::Investigating | DisputePhase::Suspended
                )
            {
                dispute.phase = DisputePhase::Refuted;
                dispute.resolved_at = Some(self.tick);
                updates.push((dispute.id, dispute.event));
            }
        }
        for (dispute_id, event_id) in updates {
            push_history(
                info,
                InfoLogEvent {
                    at: self.tick,
                    kind: InfoLogKind::DisputeUpdated,
                    actor: institution,
                    other: Some(event.provider),
                    event: event_id,
                    dispute: Some(dispute_id),
                    phase: Some(DisputePhase::Refuted),
                    witnessed: false,
                },
            );
        }
    }

    pub(super) fn dispute_bid_delta(&self, institution: ActorId, provider: ActorId) -> i16 {
        self.information()
            .map(|info| {
                info.disputes
                    .iter()
                    .filter(|dispute| {
                        dispute.institution == institution && dispute.subject == provider
                    })
                    .map(|dispute| match dispute.phase {
                        DisputePhase::Open => -2,
                        DisputePhase::Investigating => -3,
                        DisputePhase::Confirmed => -5,
                        DisputePhase::Suspended => -8,
                        DisputePhase::Refuted => 0,
                    })
                    .min()
                    .unwrap_or(0)
            })
            .unwrap_or(0)
    }

    pub(super) fn provider_suspended(&self, institution: ActorId, provider: ActorId) -> bool {
        self.information().is_some_and(|info| {
            info.disputes.iter().any(|dispute| {
                dispute.institution == institution
                    && dispute.subject == provider
                    && dispute.phase == DisputePhase::Suspended
            })
        })
    }

    pub(super) fn testimony_goal(&self, npc: &super::Npc) -> Option<Goal> {
        if self.information.is_none() || !npc.definition.shares_news || !npc.alive() {
            return None;
        }
        let preferred = npc.definition.report_to;
        let proposal = |peer: &super::Npc| {
            let recipient = peer.id();
            let event = self.reportable_information(npc.id(), recipient)?;
            (peer.alive()
                && self.can_approach(npc.position, self.meeting_place(npc.position, peer)))
            .then_some(Goal::DeliverTestimony { recipient, event })
        };
        // Prefer other institutional requesters so audit findings reach decision makers.
        let institutional = self
            .npcs
            .values()
            .filter(|peer| peer.id() != npc.id() && is_institutional_requester(peer.id()))
            .find_map(&proposal);
        if institutional.is_some() {
            return institutional;
        }
        let primary = preferred
            .and_then(|id| self.npcs.get(&id))
            .and_then(&proposal);
        if primary.is_some() || preferred.is_none() {
            return primary;
        }
        self.npcs
            .values()
            .filter(|peer| Some(peer.id()) != preferred && peer.alive())
            .find_map(proposal)
    }

    pub(super) fn dispute_goal(&self, institution: ActorId) -> Option<Goal> {
        if !self.npcs.get(&institution)?.alive() || self.companion(institution).is_some() {
            return None;
        }
        self.information()?
            .disputes
            .iter()
            .filter(|dispute| {
                dispute.institution == institution
                    && matches!(
                        dispute.phase,
                        DisputePhase::Open | DisputePhase::Investigating
                    )
            })
            .min_by_key(|dispute| (dispute.opened, dispute.id))
            .map(|dispute| Goal::InvestigateDispute(dispute.id))
    }

    fn reportable_information(&self, sender: ActorId, recipient: ActorId) -> Option<EventId> {
        let info = self.information()?;
        if sender == recipient {
            return None;
        }
        if !self.social.relationship(recipient, sender).trusts_news() {
            return None;
        }
        info.known.get(&sender).and_then(|known| {
            known.iter().rev().find_map(|(&event_id, receipt)| {
                let event = info.event(event_id)?;
                if self.tick.saturating_sub(event.tick) > NEWS_LIFETIME {
                    return None;
                }
                if info.knowledge(recipient, event_id).is_some()
                    || info.pending.contains_key(&(recipient, event_id))
                {
                    return None;
                }
                let hops = match receipt.provenance {
                    Provenance::Firsthand { .. } => 1,
                    Provenance::Reported { hops, .. } => hops.saturating_add(1),
                };
                if hops > MAX_HOPS {
                    return None;
                }
                Some(event_id)
            })
        })
    }

    pub(super) fn queue_info_reports(&mut self) {
        if self.information.is_none() {
            return;
        }
        let senders: Vec<_> = self
            .npcs
            .values()
            .filter(|npc| npc.alive() && npc.definition.shares_news)
            .map(|npc| npc.id())
            .collect();
        for sender_id in senders {
            let sender_pos = self.npcs[&sender_id].position;
            let preferred = self.npcs[&sender_id].definition.report_to;
            let recipient = self
                .npcs
                .values()
                .filter(|npc| npc.alive() && in_reach(sender_pos, npc.position))
                .filter_map(|npc| {
                    self.reportable_information(sender_id, npc.id())
                        .map(|event| (npc.id(), event))
                })
                .min_by_key(|&(id, _)| {
                    (!is_institutional_requester(id), Some(id) != preferred, id)
                });
            let Some((recipient, event_id)) = recipient else {
                continue;
            };
            let (origin, hops) = match self
                .information()
                .unwrap()
                .knowledge(sender_id, event_id)
                .expect("known information")
                .provenance
            {
                Provenance::Firsthand { observer } => (observer, 1),
                Provenance::Reported { origin, hops, .. } => (origin, hops + 1),
            };
            let recipient_pos = self.npcs[&recipient].position;
            let witnessed = self.player_sees(sender_pos) && self.player_sees(recipient_pos);
            let tick = self.tick;
            let info = self.information.as_mut().unwrap();
            if info.pending.len() >= PENDING_CAP {
                continue;
            }
            if let Entry::Vacant(entry) = info.pending.entry((recipient, event_id)) {
                entry.insert(InfoReport {
                    event: event_id,
                    sender: sender_id,
                    recipient,
                    origin,
                    hops,
                    sent: tick,
                    due: tick + REPORT_DELAY,
                });
                push_history(
                    info,
                    InfoLogEvent {
                        at: tick,
                        kind: InfoLogKind::TestimonyQueued,
                        actor: sender_id,
                        other: Some(recipient),
                        event: event_id,
                        dispute: None,
                        phase: None,
                        witnessed,
                    },
                );
            }
        }
    }

    pub(super) fn deliver_info_reports(&mut self) {
        let Some(info) = self.information.as_ref() else {
            return;
        };
        let ready: Vec<_> = info
            .pending
            .values()
            .filter(|report| report.due <= self.tick)
            .copied()
            .collect();
        for report in ready {
            self.information
                .as_mut()
                .unwrap()
                .pending
                .remove(&(report.recipient, report.event));
            if !self.npcs[&report.recipient].alive() || !self.npcs[&report.sender].alive() {
                continue;
            }
            // Delivery still requires the conversation to have been queued while
            // both were in reach; the delay models travel of the news, not teleport.
            let event = match self
                .information()
                .and_then(|info| info.event(report.event))
                .copied()
            {
                Some(event) => event,
                None => continue,
            };
            self.social.meet(report.sender, report.recipient);
            let witnessed = self.player_sees(self.npcs[&report.sender].position)
                && self.player_sees(self.npcs[&report.recipient].position);
            self.learn_information(
                report.recipient,
                event,
                Provenance::Reported {
                    sender: report.sender,
                    origin: report.origin,
                    hops: report.hops,
                },
                witnessed,
            );
        }
    }

    pub(super) fn investigate_dispute(&mut self, institution: ActorId, dispute_id: u32) {
        if self.npcs.get(&institution).is_none_or(|npc| !npc.alive())
            || self.companion(institution).is_some()
        {
            return;
        }
        let Some(dispute) = self.information().and_then(|info| {
            info.disputes
                .iter()
                .copied()
                .find(|dispute| dispute.id == dispute_id && dispute.institution == institution)
        }) else {
            return;
        };
        if !matches!(
            dispute.phase,
            DisputePhase::Open | DisputePhase::Investigating
        ) {
            return;
        }
        let Some(event) = self
            .information()
            .and_then(|info| info.event(dispute.event))
            .copied()
        else {
            return;
        };
        let here = self.npcs[&institution].position;
        if !in_reach(here, event.site) || !self.perceives(here, event.site) {
            if let Some(entry) = self
                .information
                .as_mut()
                .unwrap()
                .disputes
                .iter_mut()
                .find(|entry| entry.id == dispute_id)
            {
                entry.phase = DisputePhase::Investigating;
            }
            self.travel(institution, event.site, 1);
            return;
        }
        let knowledge = self
            .information()
            .and_then(|info| info.knowledge(institution, dispute.event));
        let phase = match knowledge.map(|entry| entry.provenance) {
            Some(Provenance::Firsthand { .. }) => DisputePhase::Suspended,
            Some(Provenance::Reported { hops: 1, .. }) => DisputePhase::Confirmed,
            Some(Provenance::Reported { .. }) => DisputePhase::Investigating,
            None => DisputePhase::Investigating,
        };
        let final_phase =
            if phase == DisputePhase::Confirmed && event.finding == AuditFinding::Unfulfilled {
                DisputePhase::Suspended
            } else {
                phase
            };
        let witnessed = self.player_sees(here) && self.player_sees(event.site);
        if let Some(entry) = self
            .information
            .as_mut()
            .unwrap()
            .disputes
            .iter_mut()
            .find(|entry| entry.id == dispute_id)
        {
            entry.phase = final_phase;
            if matches!(
                final_phase,
                DisputePhase::Confirmed | DisputePhase::Suspended | DisputePhase::Refuted
            ) {
                entry.resolved_at = Some(self.tick);
            }
        }
        push_history(
            self.information.as_mut().unwrap(),
            InfoLogEvent {
                at: self.tick,
                kind: InfoLogKind::DisputeUpdated,
                actor: institution,
                other: Some(dispute.subject),
                event: dispute.event,
                dispute: Some(dispute_id),
                phase: Some(final_phase),
                witnessed,
            },
        );
    }
}

fn is_institutional_requester(actor: ActorId) -> bool {
    Need::ALL.iter().any(|need| need.requester() == actor)
}

fn push_history(info: &mut Information, event: InfoLogEvent) {
    if info.history.len() == HISTORY_CAP {
        info.history.remove(0);
    }
    info.history.push(event);
}
