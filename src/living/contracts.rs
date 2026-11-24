//! Revision-eleven bounded institutional contract engine. The same contract
//! protocol serves two material needs: medicine in Qinghe, offerings at Reed
//! Hall. Observers discover a need locally, solicit face-to-face offers, and
//! physically return to award a provider. Cargo and coins never materialize.

use std::cmp::Reverse;

use super::navigation::in_reach;
use super::{ActorId, Goal, Npc, Position, Region, World};

const WATCH: ActorId = ActorId::new(2);
const MENTOR: ActorId = ActorId::new(33);
const LODGE: ActorId = ActorId::new(35);
const HALL: ActorId = ActorId::new(36);
const SHRINE_PROVIDERS: [ActorId; 2] = [HALL, LODGE];
const HALL_PROVIDERS: [ActorId; 1] = [HALL];
const FIRST_SHRINE: u64 = 48;
const FIRST_HALL: u64 = 64;
const COOLDOWN: u64 = 96;
pub(super) const DEADLINE: u64 = 480;
const HISTORY_CAP: usize = 160;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Need {
    ShrineMedicine,
    HallOfferings,
}

impl Need {
    pub(super) const ALL: [Self; 2] = [Self::ShrineMedicine, Self::HallOfferings];

    fn index(self) -> usize {
        match self {
            Self::ShrineMedicine => 0,
            Self::HallOfferings => 1,
        }
    }

    pub(super) fn requester(self) -> ActorId {
        match self {
            Self::ShrineMedicine => WATCH,
            Self::HallOfferings => MENTOR,
        }
    }

    fn candidates(self) -> &'static [ActorId] {
        match self {
            Self::ShrineMedicine => &SHRINE_PROVIDERS,
            Self::HallOfferings => &HALL_PROVIDERS,
        }
    }

    pub(super) fn site(self, world: &World) -> Position {
        match self {
            Self::ShrineMedicine => world.scenario.shrine,
            Self::HallOfferings => Region::sites().hall,
        }
    }

    fn triggered(self, world: &World) -> bool {
        match self {
            Self::ShrineMedicine => {
                (world.tick >= FIRST_SHRINE && world.supply.shrine <= 1)
                    || (world.is_violence_logistics()
                        && world.tick >= 33
                        && world.supply.shrine <= 2)
            }
            Self::HallOfferings => {
                world.tick >= FIRST_HALL
                    && world
                        .region()
                        .is_some_and(|region| region.hall_offerings == 0)
                    && world.rivalry().is_some_and(|rivalry| rivalry.pressure >= 4)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractPhase {
    Soliciting,
    Awarding,
    Procuring,
    Delivering,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Bid {
    pub provider: ActorId,
    pub quoted_coins: u32,
    pub local_trust: i8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contract {
    pub id: u32,
    pub need: Need,
    pub requester: ActorId,
    pub opened: u64,
    pub phase: ContractPhase,
    pub next_candidate: usize,
    pub bids: Vec<Bid>,
    pub provider: Option<ActorId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractEventKind {
    Observed,
    Offered,
    Declined,
    Awarded,
    Purchased,
    Redirected,
    Delivered,
    Failed,
    Withdrawn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContractEvent {
    pub at: u64,
    pub case_id: u32,
    pub need: Need,
    pub actor: ActorId,
    pub position: Position,
    pub kind: ContractEventKind,
    pub witnessed: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Contracts {
    pub active: Vec<Contract>,
    pub opened: u32,
    pub completed: u32,
    pub failed: u32,
    pub last_closed: [u64; 2],
    pub history: Vec<ContractEvent>,
}

impl Contracts {
    pub(super) fn new() -> Self {
        Self::default()
    }
}

impl World {
    pub fn contracts(&self) -> Option<&Contracts> {
        self.contracts.as_ref()
    }

    pub(super) fn advance_contracts(&mut self) {
        let Some(registry) = self.contracts() else {
            return;
        };
        // Validate commitments against actual bodies. A remote death ends
        // mechanical work, but its event is not broadcast as global news.
        let closures: Vec<_> = registry
            .active
            .iter()
            .filter_map(|case| {
                let responsible = if matches!(
                    case.phase,
                    ContractPhase::Soliciting | ContractPhase::Awarding
                ) {
                    case.requester
                } else {
                    case.provider?
                };
                let npc = self.npcs.get(&responsible)?;
                if !npc.alive() || self.tick.saturating_sub(case.opened) >= DEADLINE {
                    Some((
                        case.need,
                        ContractEventKind::Failed,
                        responsible,
                        npc.position,
                    ))
                } else if case.provider == Some(responsible)
                    && self.companion(responsible).is_some()
                {
                    Some((
                        case.need,
                        ContractEventKind::Withdrawn,
                        responsible,
                        npc.position,
                    ))
                } else {
                    None
                }
            })
            .collect();
        for (need, kind, actor, position) in closures {
            self.close_contract(need, kind, actor, position);
        }
        for need in Need::ALL {
            let registry = self.contracts().expect("institutional rules");
            if registry.active.iter().any(|case| case.need == need)
                || !need.triggered(self)
                || (registry.last_closed[need.index()] > 0
                    && self.tick.saturating_sub(registry.last_closed[need.index()]) < COOLDOWN)
            {
                continue;
            }
            let requester = need.requester();
            let Some(npc) = self.npcs.get(&requester) else {
                continue;
            };
            if !npc.alive()
                || self.companion(requester).is_some()
                || !self.perceives(npc.position, need.site(self))
            {
                continue;
            }
            let position = npc.position;
            let registry = self.contracts.as_mut().expect("institutional rules");
            registry.opened = registry.opened.saturating_add(1);
            let id = registry.opened;
            registry.active.push(Contract {
                id,
                need,
                requester,
                opened: self.tick,
                phase: ContractPhase::Soliciting,
                next_candidate: 0,
                bids: Vec::new(),
                provider: None,
            });
            self.record_contract(need, ContractEventKind::Observed, requester, position);
        }
    }

    pub(super) fn contract_goal(&self, npc: &Npc) -> Option<Goal> {
        self.contracts()?.active.iter().find_map(|case| {
            if case.requester == npc.id()
                && matches!(
                    case.phase,
                    ContractPhase::Soliciting | ContractPhase::Awarding
                )
            {
                return Some(Goal::NegotiateContract(case.need));
            }
            if case.provider != Some(npc.id()) {
                return None;
            }
            Some(match case.phase {
                ContractPhase::Procuring if npc.herbs == 0 => Goal::ProcureContract(case.need),
                ContractPhase::Procuring | ContractPhase::Delivering => {
                    Goal::FulfillContract(case.need)
                }
                _ => return None,
            })
        })
    }

    pub(super) fn work_contract(&mut self, actor: ActorId, goal: Goal) {
        let need = match goal {
            Goal::NegotiateContract(need)
            | Goal::ProcureContract(need)
            | Goal::FulfillContract(need) => need,
            _ => return,
        };
        let Some(case) = self
            .contracts()
            .and_then(|r| r.active.iter().find(|c| c.need == need))
            .cloned()
        else {
            return;
        };
        match goal {
            Goal::NegotiateContract(_) if case.requester == actor => self.negotiate_contract(&case),
            Goal::ProcureContract(_) if case.provider == Some(actor) => {
                self.procure_contract(&case)
            }
            Goal::FulfillContract(_) if case.provider == Some(actor) => {
                self.fulfill_contract(&case)
            }
            _ => {}
        }
    }

    fn contract_mut(&mut self, need: Need) -> &mut Contract {
        self.contracts
            .as_mut()
            .expect("institutional rules")
            .active
            .iter_mut()
            .find(|case| case.need == need)
            .expect("active contract")
    }

    fn negotiate_contract(&mut self, case: &Contract) {
        let here = self.npcs[&case.requester].position;
        if case.phase == ContractPhase::Awarding {
            let provider = case.provider.expect("selected bid");
            let peer = self.npcs[&provider];
            if !in_reach(here, peer.position) {
                let destination = if self.perceives(here, peer.position) {
                    peer.position
                } else {
                    peer.definition.home
                };
                self.travel(case.requester, destination, 1);
                return;
            }
            let provider_committed =
                self.contracts()
                    .expect("contracts")
                    .active
                    .iter()
                    .any(|other| {
                        other.need != case.need
                            && other.provider == Some(provider)
                            && matches!(
                                other.phase,
                                ContractPhase::Procuring | ContractPhase::Delivering
                            )
                    });
            if provider_committed || self.offer_from(provider, case.requester).is_none() {
                self.contract_mut(case.need)
                    .bids
                    .retain(|bid| bid.provider != provider);
                self.record_contract(case.need, ContractEventKind::Declined, provider, here);
                let remaining = self
                    .contracts()
                    .unwrap()
                    .active
                    .iter()
                    .find(|active| active.need == case.need)
                    .cloned()
                    .expect("active");
                self.choose_contract_bid(&remaining);
                return;
            }
            self.contract_mut(case.need).phase = ContractPhase::Procuring;
            self.record_contract(case.need, ContractEventKind::Awarded, case.requester, here);
            return;
        }
        if case.phase != ContractPhase::Soliciting {
            return;
        }
        let candidates = case.need.candidates();
        if case.next_candidate == candidates.len() {
            self.choose_contract_bid(case);
            return;
        }
        let candidate = candidates[case.next_candidate];
        let Some(peer) = self.npcs.get(&candidate) else {
            self.contract_mut(case.need).next_candidate += 1;
            return;
        };
        // The messenger travels to known workplaces; a distant dead NPC
        // cannot be diagnosed from a hidden health flag.
        if !in_reach(here, peer.position) {
            let destination = if self.perceives(here, peer.position) {
                peer.position
            } else {
                peer.definition.home
            };
            self.travel(case.requester, destination, 1);
            return;
        }
        let at = peer.position;
        let bid = self.offer_from(candidate, case.requester);
        let registry = self.contract_mut(case.need);
        if let Some(bid) = bid {
            registry.bids.push(bid);
        }
        registry.next_candidate += 1;
        self.record_contract(
            case.need,
            if bid.is_some() {
                ContractEventKind::Offered
            } else {
                ContractEventKind::Declined
            },
            candidate,
            at,
        );
        if case.next_candidate + 1 == candidates.len() {
            let current = self
                .contracts()
                .unwrap()
                .active
                .iter()
                .find(|c| c.need == case.need)
                .cloned()
                .unwrap();
            self.choose_contract_bid(&current);
        }
    }

    fn offer_from(&self, provider: ActorId, requester: ActorId) -> Option<Bid> {
        let peer = self.npcs.get(&provider)?;
        if !peer.alive()
            || self.companion(provider).is_some()
            || self.social.relationship(provider, requester).refuses_aid()
        {
            return None;
        }
        let cost = if peer.herbs > 0 {
            0
        } else {
            self.market_quote()?.buy
        };
        let treasury = if provider == HALL {
            self.politics()?.hall_coins
        } else if provider == LODGE {
            self.rivalry()?.lodge_coins
        } else {
            return None;
        };
        if (cost > 0 && self.region()?.market_herbs == 0) || treasury < cost {
            return None;
        }
        Some(Bid {
            provider,
            quoted_coins: cost,
            local_trust: self.social.relationship(requester, provider).regard,
        })
    }

    fn choose_contract_bid(&mut self, case: &Contract) {
        let occupied: Vec<_> = self
            .contracts()
            .unwrap()
            .active
            .iter()
            .filter(|active| {
                active.need != case.need
                    && matches!(
                        active.phase,
                        ContractPhase::Procuring | ContractPhase::Delivering
                    )
            })
            .filter_map(|active| active.provider)
            .collect();
        let selected = case
            .bids
            .iter()
            .filter(|bid| !occupied.contains(&bid.provider))
            .filter(|bid| !self.provider_suspended(case.requester, bid.provider))
            .max_by_key(|bid| {
                (
                    i16::from(bid.local_trust)
                        + i16::from(self.contract_regard(case.requester, bid.provider))
                        + self.dispute_bid_delta(case.requester, bid.provider),
                    Reverse(bid.quoted_coins),
                    bid.provider == HALL,
                    Reverse(bid.provider),
                )
            })
            .map(|bid| bid.provider);
        if let Some(provider) = selected {
            let contract = self.contract_mut(case.need);
            contract.provider = Some(provider);
            contract.phase = ContractPhase::Awarding;
        } else {
            let position = self.npcs[&case.requester].position;
            self.close_contract(
                case.need,
                ContractEventKind::Failed,
                case.requester,
                position,
            );
        }
    }

    fn procure_contract(&mut self, case: &Contract) {
        if case.phase != ContractPhase::Procuring {
            return;
        }
        let provider = case.provider.expect("accepted bid");
        let here = self.npcs[&provider].position;
        if self.npcs[&provider].herbs > 0 {
            self.begin_contract_delivery(case.need, ContractEventKind::Redirected, provider, here);
            return;
        }
        let market = Region::sites().market;
        if !in_reach(here, market) {
            self.travel(provider, market, 1);
            return;
        }
        if self.is_night() {
            return;
        }
        let price = self.market_quote().expect("regional market").buy;
        if self.region().is_none_or(|region| region.market_herbs == 0)
            || (provider == HALL && self.politics().is_none_or(|p| p.hall_coins < price))
            || (provider == LODGE && self.rivalry().is_none_or(|r| r.lodge_coins < price))
        {
            return;
        }
        let region = self.region.as_mut().expect("regional market");
        region.market_herbs -= 1;
        region.market_coins += price;
        if provider == HALL {
            let politics = self.politics.as_mut().expect("Hall");
            politics.hall_coins -= price;
            politics.hall_purchases += 1;
        } else {
            let rivalry = self.rivalry.as_mut().expect("Stonebank");
            rivalry.lodge_coins -= price;
            rivalry.market_purchases += 1;
        }
        self.npcs.get_mut(&provider).expect("provider").herbs += 1;
        self.begin_contract_delivery(case.need, ContractEventKind::Purchased, provider, here);
    }

    fn begin_contract_delivery(
        &mut self,
        need: Need,
        kind: ContractEventKind,
        actor: ActorId,
        here: Position,
    ) {
        self.contract_mut(need).phase = ContractPhase::Delivering;
        self.record_contract(need, kind, actor, here);
    }

    fn fulfill_contract(&mut self, case: &Contract) {
        let provider = case.provider.expect("awarded provider");
        let here = self.npcs[&provider].position;
        if case.phase == ContractPhase::Procuring && self.npcs[&provider].herbs > 0 {
            self.begin_contract_delivery(case.need, ContractEventKind::Redirected, provider, here);
        }
        if self.npcs[&provider].herbs == 0 {
            self.contract_mut(case.need).phase = ContractPhase::Procuring;
            return;
        }
        let destination = case.need.site(self);
        if here != destination {
            self.travel(provider, destination, 0);
            return;
        }
        self.npcs.get_mut(&provider).expect("provider").herbs -= 1;
        match case.need {
            Need::ShrineMedicine => {
                self.supply.shrine += 1;
                self.supply.npc_deliveries += 1;
            }
            Need::HallOfferings => {
                self.region.as_mut().expect("region").hall_offerings += 1;
                self.politics.as_mut().expect("politics").hall_supplies += 1;
            }
        }
        let rivalry = self.rivalry.as_mut().expect("rivalry");
        rivalry.pressure = if provider == LODGE {
            rivalry.pressure.saturating_add(1).min(8)
        } else {
            rivalry.pressure.saturating_sub(1)
        };
        let observer = self.npcs[&case.requester];
        if observer.alive() && self.perceives(observer.position, destination) {
            self.social.aided(provider, case.requester);
        }
        self.close_contract(case.need, ContractEventKind::Delivered, provider, here);
    }

    fn close_contract(
        &mut self,
        need: Need,
        kind: ContractEventKind,
        actor: ActorId,
        position: Position,
    ) {
        let receipt = self
            .contracts()
            .and_then(|registry| registry.active.iter().find(|case| case.need == need))
            .and_then(|case| {
                case.provider
                    .filter(|_| {
                        matches!(
                            case.phase,
                            ContractPhase::Procuring | ContractPhase::Delivering
                        )
                    })
                    .map(|provider| super::contract_memory::ContractReceipt {
                        case_id: case.id,
                        need,
                        requester: case.requester,
                        provider,
                        opened: case.opened,
                        closed: self.tick,
                        delivered: kind == ContractEventKind::Delivered,
                        inspected: false,
                    })
            });
        let registry = self.contracts.as_mut().expect("contracts");
        let case_id = registry
            .active
            .iter()
            .find(|case| case.need == need)
            .map_or(registry.opened, |case| case.id);
        registry.active.retain(|case| case.need != need);
        registry.last_closed[need.index()] = self.tick;
        if kind == ContractEventKind::Delivered {
            registry.completed = registry.completed.saturating_add(1);
        } else {
            registry.failed = registry.failed.saturating_add(1);
        }
        self.record_contract_with_id(case_id, need, kind, actor, position);
        if let Some(receipt) = receipt {
            self.retain_contract_receipt(receipt);
        }
    }

    fn record_contract(
        &mut self,
        need: Need,
        kind: ContractEventKind,
        actor: ActorId,
        position: Position,
    ) {
        let registry = self.contracts().expect("contracts");
        let case_id = registry
            .active
            .iter()
            .find(|case| case.need == need)
            .map_or(registry.opened, |case| case.id);
        self.record_contract_with_id(case_id, need, kind, actor, position);
    }

    fn record_contract_with_id(
        &mut self,
        case_id: u32,
        need: Need,
        kind: ContractEventKind,
        actor: ActorId,
        position: Position,
    ) {
        let witnessed = self.player_sees(position);
        let registry = self.contracts.as_mut().expect("contracts");
        if registry.history.len() == HISTORY_CAP {
            registry.history.remove(0);
        }
        registry.history.push(ContractEvent {
            at: self.tick,
            case_id,
            need,
            actor,
            position,
            kind,
            witnessed,
        });
    }
}
