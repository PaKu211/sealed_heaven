use std::collections::{BTreeMap, btree_map::Entry};

use super::navigation::in_reach;
use super::{ActorId, PLAYER, Position, World};

const REPORT_DELAY: u64 = 2;
const MAX_HOPS: u8 = 2;
const NEWS_LIFETIME: u64 = 32;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct IncidentId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Incident {
    pub id: IncidentId,
    pub tick: u64,
    pub source: ActorId,
    pub target: ActorId,
    pub position: Position,
    pub damage: u16,
    pub fatal: bool,
    pub retaliation: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KnowledgeSource {
    Observed,
    Reported {
        sender: ActorId,
        witness: ActorId,
        hops: u8,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Knowledge {
    pub acquired: u64,
    pub source: KnowledgeSource,
    /// The actual change, after clamping. Committed with this recipient's receipt.
    pub regard_change: i8,
}

/// Seeing a body does not identify its killer. Someone found nearby is a
/// lead, not a witnessed assailant or an allegation that can be reported as fact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BodyDiscovery {
    pub observer: ActorId,
    pub body: ActorId,
    pub at: u64,
    pub nearby: Option<ActorId>,
    pub source: KnowledgeSource,
    /// Revision 17 keeps the original account after an observer checks the
    /// body themselves. Revision 16 never upgrades received accounts.
    pub prior_report: Option<(u64, KnowledgeSource)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BodyReport {
    pub sender: ActorId,
    pub recipient: ActorId,
    pub body: ActorId,
    pub due: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryEvent {
    pub id: u32,
    pub at: u64,
    pub actor: ActorId,
    pub body: ActorId,
    pub herbs: u32,
    pub delivered: bool,
    pub witnessed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Report {
    pub incident: IncidentId,
    pub sender: ActorId,
    pub recipient: ActorId,
    pub witness: ActorId,
    pub hops: u8,
    pub sent: u64,
    pub due: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Relationship {
    /// Positive values are trust; negative values are hostility. Range -8 to 8.
    pub regard: i8,
    pub familiarity: u8,
}

impl Relationship {
    pub fn refuses_aid(self) -> bool {
        self.regard < 0
    }

    pub(super) fn trusts_news(self) -> bool {
        self.familiarity > 0 && self.regard >= 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Social {
    pub(super) incidents: Vec<Incident>,
    pub(super) known: BTreeMap<ActorId, BTreeMap<IncidentId, Knowledge>>,
    pub(super) relations: BTreeMap<(ActorId, ActorId), Relationship>,
    pub(super) pending: BTreeMap<(ActorId, IncidentId), Report>,
    pub(super) discoveries: BTreeMap<(ActorId, ActorId), BodyDiscovery>,
    pub(super) body_reports: BTreeMap<(ActorId, ActorId), BodyReport>,
    pub(super) recovery_cargo: BTreeMap<ActorId, u32>,
    pub(super) recoveries: Vec<RecoveryEvent>,
}

impl Social {
    pub(super) fn new(contacts: &[(ActorId, ActorId)]) -> Self {
        let mut social = Self {
            incidents: Vec::new(),
            known: BTreeMap::new(),
            relations: BTreeMap::new(),
            pending: BTreeMap::new(),
            discoveries: BTreeMap::new(),
            body_reports: BTreeMap::new(),
            recovery_cargo: BTreeMap::new(),
            recoveries: Vec::new(),
        };
        for &(a, b) in contacts {
            social.meet(a, b);
            social.adjust(a, b, 1);
            social.adjust(b, a, 1);
        }
        social
    }

    pub fn incidents(&self) -> &[Incident] {
        &self.incidents
    }

    pub fn incident(&self, id: IncidentId) -> Option<&Incident> {
        self.incidents.get(id.0 as usize)
    }

    pub fn knowledge(&self, actor: ActorId, incident: IncidentId) -> Option<Knowledge> {
        self.known.get(&actor)?.get(&incident).copied()
    }

    pub fn relationship(&self, actor: ActorId, other: ActorId) -> Relationship {
        self.relations
            .get(&(actor, other))
            .copied()
            .unwrap_or_default()
    }

    pub fn pending_reports(&self) -> impl ExactSizeIterator<Item = &Report> {
        self.pending.values()
    }

    pub fn discoveries(&self) -> impl Iterator<Item = &BodyDiscovery> {
        self.discoveries.values()
    }

    pub fn pending_body_reports(&self) -> impl Iterator<Item = &BodyReport> {
        self.body_reports.values()
    }

    pub fn recoveries(&self) -> &[RecoveryEvent] {
        &self.recoveries
    }

    pub(super) fn meet(&mut self, a: ActorId, b: ActorId) {
        for pair in [(a, b), (b, a)] {
            let relation = self.relations.entry(pair).or_default();
            relation.familiarity = relation.familiarity.saturating_add(1).min(8);
        }
    }

    /// Establish a trusted news link used by institutional briefing networks.
    pub(super) fn link(&mut self, a: ActorId, b: ActorId) {
        self.meet(a, b);
        self.adjust(a, b, 1);
        self.adjust(b, a, 1);
    }

    pub(super) fn aided(&mut self, helper: ActorId, patient: ActorId) {
        self.meet(helper, patient);
        self.adjust(patient, helper, 1);
    }

    fn adjust(&mut self, actor: ActorId, other: ActorId, delta: i8) -> i8 {
        if delta == 0 {
            return 0;
        }
        let relation = self.relations.entry((actor, other)).or_default();
        let before = relation.regard;
        relation.regard = (before + delta).clamp(-8, 8);
        relation.regard - before
    }

    fn learn(&mut self, actor: ActorId, incident: Incident, tick: u64, source: KnowledgeSource) {
        // A report in flight can be superseded by direct evidence. Even then
        // there is only one receipt and one relation change per actor/incident.
        self.pending.remove(&(actor, incident.id));
        if self.knowledge(actor, incident.id).is_some() {
            return;
        }
        let penalty = if actor == incident.source {
            0
        } else {
            match source {
                KnowledgeSource::Observed if actor == incident.target => -4,
                KnowledgeSource::Observed if !incident.retaliation => -3,
                KnowledgeSource::Reported { hops: 1, .. } => -2,
                KnowledgeSource::Reported { .. } => -1,
                KnowledgeSource::Observed => 0,
            }
        };
        let regard_change = self.adjust(actor, incident.source, penalty);
        self.known.entry(actor).or_default().insert(
            incident.id,
            Knowledge {
                acquired: tick,
                source,
                regard_change,
            },
        );
    }

    pub(super) fn recent(
        &self,
        actor: ActorId,
        at: u64,
        lifetime: u64,
    ) -> impl Iterator<Item = (&Incident, &Knowledge)> {
        let incidents = &self.incidents;
        self.known
            .get(&actor)
            .into_iter()
            .flat_map(|known| known.iter().rev())
            .map(move |(id, receipt)| (&incidents[id.0 as usize], receipt))
            .take_while(move |(incident, _)| at - incident.tick <= lifetime)
    }

    pub(super) fn is_news(
        &self,
        sender: ActorId,
        recipient: ActorId,
        incident: &Incident,
        receipt: &Knowledge,
    ) -> bool {
        sender != recipient
            && recipient != incident.source
            && sender != incident.source
            && !incident.retaliation
            && !matches!(receipt.source, KnowledgeSource::Reported { hops, .. } if hops >= MAX_HOPS)
            && self.knowledge(recipient, incident.id).is_none()
            && !self.pending.contains_key(&(recipient, incident.id))
            && self.relationship(recipient, sender).trusts_news()
    }
}

impl World {
    pub(super) fn report_body(
        &mut self,
        to: ActorId,
        body: ActorId,
    ) -> Result<super::Outcome, super::Rejection> {
        use super::{Outcome, Rejection};
        if !self.player.alive() {
            return Err(Rejection::Dead);
        }
        let recipient = self
            .npcs
            .get(&to)
            .filter(|npc| npc.alive())
            .ok_or(Rejection::UnknownActor)?;
        self.require_reach(recipient.position)?;
        if !self.social.discoveries.contains_key(&(PLAYER, body)) {
            return Err(Rejection::NoEvidence);
        }
        if self.social.relationship(to, PLAYER).refuses_aid() {
            return Err(Rejection::Refused);
        }
        if self.social.discoveries.contains_key(&(to, body))
            || self.social.body_reports.contains_key(&(to, body))
        {
            return Err(Rejection::Refused);
        }
        self.social.body_reports.insert(
            (to, body),
            BodyReport {
                sender: PLAYER,
                recipient: to,
                body,
                due: self.tick + REPORT_DELAY,
            },
        );
        Ok(Outcome::BodyReported { to, body })
    }

    pub(super) fn observe_bodies(&mut self) {
        if !self.is_violence_logistics() {
            return;
        }
        let bodies: Vec<_> = self
            .npcs
            .values()
            .filter(|npc| !npc.alive())
            .map(|npc| (npc.id(), npc.position))
            .collect();
        let observers: Vec<_> = self
            .npcs
            .values()
            .filter(|npc| npc.alive())
            .map(|npc| (npc.id(), npc.position))
            .chain(
                self.player
                    .alive()
                    .then_some((PLAYER, self.player.position)),
            )
            .collect();
        for (body, position) in bodies {
            for (observer, location) in &observers {
                if *observer == body || !self.perceives(*location, position) {
                    continue;
                }
                let key = (*observer, body);
                if self.social.discoveries.get(&key).is_some_and(|receipt| {
                    receipt.source == KnowledgeSource::Observed || !self.is_player_reporting()
                }) {
                    continue;
                }
                // Even if a witness saw the killing, this independent body
                // observation cannot confer that knowledge on anyone else.
                let nearby = (*observer != PLAYER
                    && self.player.alive()
                    && self.perceives(*location, self.player.position)
                    && in_reach(self.player.position, position))
                .then_some(PLAYER);
                let prior_report = self
                    .social
                    .discoveries
                    .get(&key)
                    .map(|receipt| (receipt.at, receipt.source));
                self.social.body_reports.remove(&key);
                self.social.discoveries.insert(
                    key,
                    BodyDiscovery {
                        observer: *observer,
                        body,
                        at: self.tick,
                        nearby,
                        source: KnowledgeSource::Observed,
                        prior_report,
                    },
                );
            }
        }
    }

    pub(super) fn deliver_body_reports(&mut self) {
        if !self.is_violence_logistics() {
            return;
        }
        let due: Vec<_> = self
            .social
            .body_reports
            .iter()
            .filter(|(_, report)| report.due <= self.tick)
            .map(|(&key, &report)| (key, report))
            .collect();
        for (key, report) in due {
            self.social.body_reports.remove(&key);
            if !self.participant_alive(report.sender)
                || !self.participant_alive(report.recipient)
                || self.social.discoveries.contains_key(&key)
            {
                continue;
            }
            let Some(receipt) = self
                .social
                .discoveries
                .get(&(report.sender, report.body))
                .copied()
            else {
                continue;
            };
            let (witness, hops) = match receipt.source {
                KnowledgeSource::Observed => (report.sender, 1),
                KnowledgeSource::Reported { witness, hops, .. } => (witness, hops + 1),
            };
            self.social.discoveries.insert(
                key,
                BodyDiscovery {
                    observer: report.recipient,
                    body: report.body,
                    at: self.tick,
                    nearby: None,
                    source: KnowledgeSource::Reported {
                        sender: report.sender,
                        witness,
                        hops,
                    },
                    prior_report: None,
                },
            );
        }
    }

    pub(super) fn queue_body_reports(&mut self) {
        if !self.is_violence_logistics() {
            return;
        }
        let known: Vec<_> = self.social.discoveries.values().copied().collect();
        for receipt in known {
            let Some(sender) = self.npcs.get(&receipt.observer) else {
                continue;
            };
            if !sender.alive()
                || !sender.definition.shares_news
                || matches!(receipt.source, KnowledgeSource::Reported { hops: 2.., .. })
            {
                continue;
            }
            let recipient = self
                .npcs
                .values()
                .filter(|npc| npc.alive() && npc.id() != sender.id())
                .filter(|npc| in_reach(npc.position, sender.position))
                .filter(|npc| {
                    self.social
                        .relationship(npc.id(), sender.id())
                        .trusts_news()
                })
                .find(|npc| {
                    !self
                        .social
                        .discoveries
                        .contains_key(&(npc.id(), receipt.body))
                        && !self
                            .social
                            .body_reports
                            .contains_key(&(npc.id(), receipt.body))
                })
                .map(|npc| npc.id());
            if let Some(recipient) = recipient {
                self.social.body_reports.insert(
                    (recipient, receipt.body),
                    BodyReport {
                        sender: receipt.observer,
                        recipient,
                        body: receipt.body,
                        due: self.tick + REPORT_DELAY,
                    },
                );
            }
        }
    }

    /// Both combatants are identified at the strike location. Nearby people must
    /// see both; the victim always knows who landed the blow.
    pub(super) fn damage(
        &mut self,
        source: ActorId,
        target: ActorId,
        amount: u16,
        retaliation: bool,
    ) {
        let source_position = if source == PLAYER {
            self.player.position
        } else {
            self.npcs[&source].position
        };
        let (position, hp) = if target == PLAYER {
            (self.player.position, self.player.hp)
        } else {
            let npc = &self.npcs[&target];
            (npc.position, npc.hp)
        };
        let parry = if amount > 0 && hp > 0 {
            self.martial_block(target)
        } else {
            0
        };
        let blocked = if target == PLAYER
            && amount > 0
            && matches!(
                self.rules,
                super::Rules::SectCultivation
                    | super::Rules::MartialMentorship
                    | super::Rules::MeridianCultivation
                    | super::Rules::PersistentInjury
                    | super::Rules::AutonomousAgency
            )
            && self.contested().is_some_and(|state| state.wards > 0)
        {
            self.contested.as_mut().unwrap().wards -= 1;
            1
        } else {
            0
        };
        let damage = hp.min(amount.saturating_sub(blocked + parry));
        if damage == 0 {
            return;
        }

        let observed: Vec<_> = self
            .npcs
            .values()
            .filter(|npc| npc.alive())
            .filter(|npc| {
                npc.id() == source
                    || npc.id() == target
                    || (self.perceives(npc.position, position)
                        && self.perceives(npc.position, source_position))
            })
            .map(|npc| npc.id())
            .collect();

        let incident = Incident {
            id: IncidentId(self.social.incidents.len() as u32),
            tick: self.tick,
            source,
            target,
            position,
            damage,
            fatal: damage == hp,
            retaliation,
        };
        self.social.incidents.push(incident);
        for actor in observed {
            self.social
                .learn(actor, incident, self.tick, KnowledgeSource::Observed);
        }
        if target == PLAYER {
            self.player.hp -= damage;
            if damage == hp {
                self.drop_player_lots();
            }
        } else {
            self.npcs.get_mut(&target).expect("known combat target").hp -= damage;
            // The physical herb stays on a fallen carrier; only the return
            // assignment ends. A player may still take it from the body.
            if damage == hp && self.is_physical_recovery() {
                self.social.recovery_cargo.remove(&target);
            }
        }
        self.record_wound(target, damage, hp - damage, amount);
    }

    /// A body is robbed where it lies. Only people who can see both the player
    /// and the body learn who took the cargo. The herbs themselves are already gone.
    pub(super) fn witness_transfer(&mut self, target: ActorId, position: Position) {
        let incident = Incident {
            id: IncidentId(self.social.incidents.len() as u32),
            tick: self.tick,
            source: PLAYER,
            target,
            position,
            damage: 0,
            fatal: false,
            retaliation: false,
        };
        self.social.incidents.push(incident);
        let watchers: Vec<_> = self
            .npcs
            .values()
            .filter(|npc| npc.alive())
            .filter(|npc| {
                self.perceives(npc.position, position)
                    && self.perceives(npc.position, self.player.position)
            })
            .map(|npc| npc.id())
            .collect();
        self.social
            .learn(PLAYER, incident, self.tick, KnowledgeSource::Observed);
        for actor in watchers {
            self.social
                .learn(actor, incident, self.tick, KnowledgeSource::Observed);
        }
    }

    pub(super) fn reportable(&self, sender: ActorId, recipient: ActorId) -> Option<IncidentId> {
        self.social
            .recent(sender, self.tick, NEWS_LIFETIME)
            .find(|(incident, receipt)| self.social.is_news(sender, recipient, incident, receipt))
            .map(|(incident, _)| incident.id)
    }

    pub(super) fn queue_reports(&mut self) {
        // At most one conversation per sender per turn. Prefer the actor's
        // chosen destination, then use stable recipient and event ordering.
        for sender in self
            .npcs
            .values()
            .filter(|npc| npc.alive() && npc.definition.shares_news)
        {
            let preferred = sender.definition.report_to;
            let recipient = self
                .npcs
                .values()
                .filter(|npc| npc.alive() && in_reach(sender.position, npc.position))
                .filter_map(|npc| {
                    self.reportable(sender.id(), npc.id())
                        .map(|id| (npc.id(), id))
                })
                .min_by_key(|&(id, _)| (Some(id) != preferred, id));
            let Some((recipient, incident)) = recipient else {
                continue;
            };
            let (witness, hops) = match self
                .social
                .knowledge(sender.id(), incident)
                .expect("known report")
                .source
            {
                KnowledgeSource::Observed => (sender.id(), 1),
                KnowledgeSource::Reported { witness, hops, .. } => (witness, hops + 1),
            };
            if let Entry::Vacant(entry) = self.social.pending.entry((recipient, incident)) {
                entry.insert(Report {
                    incident,
                    sender: sender.id(),
                    recipient,
                    witness,
                    hops,
                    sent: self.tick,
                    due: self.tick + REPORT_DELAY,
                });
            }
        }
    }

    pub(super) fn deliver_reports(&mut self) {
        // Distinct receipts are independent. BTreeMap orders equal-tick
        // deliveries by recipient ID and then incident ID.
        let ready: Vec<_> = self
            .social
            .pending
            .values()
            .filter(|report| report.due <= self.tick)
            .copied()
            .collect();
        for report in ready {
            self.social
                .pending
                .remove(&(report.recipient, report.incident));
            if self.npcs[&report.recipient].alive() {
                let incident = self.social.incidents[report.incident.0 as usize];
                self.social.meet(report.sender, report.recipient);
                self.social.learn(
                    report.recipient,
                    incident,
                    self.tick,
                    KnowledgeSource::Reported {
                        sender: report.sender,
                        witness: report.witness,
                        hops: report.hops,
                    },
                );
            }
        }
    }
}
