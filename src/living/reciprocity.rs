//! Bounded bilateral gratitude, not a debt contract or a universal reputation.
//! Only committed assistance changes the ledger. Returns use actual resources.
use super::{
    ActorId, Goal, Outcome, PLAYER, Position, Rejection, World, character_agency::CLINIC_FEE,
    navigation::in_reach,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const REVISION: u16 = 35;
pub const VERSION: u16 = 1;
pub const BALANCE_CAP: i8 = 3;
pub const HISTORY_CAP: usize = 96;
pub const OBSERVED_CAP: usize = 48;
const RESERVE: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Asset {
    Coin,
    Herb,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Kind {
    Provision,
    CareFunding,
    ClinicCare,
    FieldCare,
    ReturnedCoin,
    ReturnedHerb,
    RepaidCoin,
    Released,
    NeighborCoin,
    NeighborHerb,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AidEvent {
    pub id: u32,
    pub at: u64,
    pub helper: ActorId,
    pub recipient: ActorId,
    pub kind: Kind,
    pub place: Position,
    /// Positive: recipient owes helper. Negative: helper owes recipient.
    pub balance: i8,
    pub change: i8,
}
/// A witness can see the act but cannot know another pair's private account.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservedAid {
    pub id: u32,
    pub at: u64,
    pub helper: ActorId,
    pub recipient: ActorId,
    pub kind: Kind,
    pub place: Position,
    pub balance: Option<i8>,
    pub change: Option<i8>,
}
impl From<AidEvent> for ObservedAid {
    fn from(e: AidEvent) -> Self {
        let own = e.helper == PLAYER || e.recipient == PLAYER;
        Self {
            id: e.id,
            at: e.at,
            helper: e.helper,
            recipient: e.recipient,
            kind: e.kind,
            place: e.place,
            balance: own.then_some(e.balance),
            change: own.then_some(e.change),
        }
    }
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Reciprocity {
    // Positive balance means the larger ID owes the smaller ID.
    balances: BTreeMap<(ActorId, ActorId), i8>,
    history: VecDeque<AidEvent>,
    observed: VecDeque<ObservedAid>,
    next_event: u32,
}
impl Reciprocity {
    fn ordered(a: ActorId, b: ActorId) -> ((ActorId, ActorId), i8) {
        if a < b { ((a, b), 1) } else { ((b, a), -1) }
    }
    pub fn net(&self, creditor: ActorId, debtor: ActorId) -> i8 {
        if creditor == debtor {
            return 0;
        }
        let (pair, sign) = Self::ordered(creditor, debtor);
        self.balances.get(&pair).copied().unwrap_or(0) * sign
    }
    pub fn balances(&self) -> impl Iterator<Item = (&(ActorId, ActorId), &i8)> {
        self.balances.iter()
    }
    pub fn history(&self) -> &VecDeque<AidEvent> {
        &self.history
    }
    pub fn observed(&self) -> &VecDeque<ObservedAid> {
        &self.observed
    }
    fn record(
        &mut self,
        at: u64,
        helper: ActorId,
        recipient: ActorId,
        kind: Kind,
        place: Position,
        seen: bool,
    ) {
        if helper == recipient {
            return;
        }
        let old = self.net(helper, recipient);
        let balance = if kind == Kind::Released {
            assert!(old > 0);
            0
        } else {
            (old + 1).min(BALANCE_CAP)
        };
        let (pair, sign) = Self::ordered(helper, recipient);
        if balance == 0 {
            self.balances.remove(&pair);
        } else {
            self.balances.insert(pair, balance * sign);
        }
        let event = AidEvent {
            id: self.next_event,
            at,
            helper,
            recipient,
            kind,
            place,
            balance,
            change: balance - old,
        };
        self.next_event += 1; // At most O(actor count) commits per bounded turn.
        if self.history.len() == HISTORY_CAP {
            self.history.pop_front();
        }
        self.history.push_back(event);
        if seen {
            if self.observed.len() == OBSERVED_CAP {
                self.observed.pop_front();
            }
            self.observed.push_back(event.into());
        }
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        fn events(out: &mut Vec<u8>, rows: &VecDeque<AidEvent>) {
            out.extend((rows.len() as u32).to_le_bytes());
            for e in rows {
                out.extend(e.id.to_le_bytes());
                out.extend(e.at.to_le_bytes());
                out.extend(e.helper.get().to_le_bytes());
                out.extend(e.recipient.get().to_le_bytes());
                out.push(e.kind as u8);
                out.extend(e.place.x.to_le_bytes());
                out.extend(e.place.y.to_le_bytes());
                out.push(e.balance as u8);
                out.push(e.change as u8);
            }
        }
        let mut out = b"SH:C48:RECIPROCITY:v1\0".to_vec();
        out.extend(VERSION.to_le_bytes());
        out.extend(self.next_event.to_le_bytes());
        out.extend((self.balances.len() as u32).to_le_bytes());
        for (&(a, b), &n) in &self.balances {
            out.extend(a.get().to_le_bytes());
            out.extend(b.get().to_le_bytes());
            out.push(n as u8);
        }
        events(&mut out, &self.history);
        out.extend((self.observed.len() as u32).to_le_bytes());
        for e in &self.observed {
            out.extend(e.id.to_le_bytes());
            out.extend(e.at.to_le_bytes());
            out.extend(e.helper.get().to_le_bytes());
            out.extend(e.recipient.get().to_le_bytes());
            out.push(e.kind as u8);
            out.extend(e.place.x.to_le_bytes());
            out.extend(e.place.y.to_le_bytes());
            match (e.balance, e.change) {
                (Some(b), Some(c)) => {
                    out.push(1);
                    out.push(b as u8);
                    out.push(c as u8);
                }
                (None, None) => out.push(0),
                _ => unreachable!("observations redact both account values together"),
            }
        }
        out
    }
}
impl World {
    pub fn reciprocity(&self) -> Option<&Reciprocity> {
        self.reciprocity.as_ref()
    }
    /// Own bilateral accounts are known through direct participation; no
    /// current remote NPC relationship, inventory or third-party balance.
    pub fn known_bonds(&self) -> Vec<(ActorId, i8)> {
        self.reciprocity().map_or_else(Vec::new, |r| {
            self.npcs()
                .filter_map(|n| {
                    let net = r.net(PLAYER, n.id());
                    (net != 0).then_some((n.id(), net))
                })
                .collect()
        })
    }
    pub fn known_aid(&self) -> impl DoubleEndedIterator<Item = &ObservedAid> {
        self.reciprocity()
            .into_iter()
            .flat_map(|r| r.observed().iter())
    }
    fn living_position(&self, id: ActorId) -> Option<Position> {
        if id == PLAYER {
            self.player.alive().then_some(self.player.position)
        } else {
            self.npc(id).filter(|n| n.alive()).map(|n| n.position)
        }
    }
    pub(super) fn remember_aid(&mut self, helper: ActorId, recipient: ActorId, kind: Kind) {
        if self.reciprocity.is_none() || helper == recipient {
            return;
        }
        let (Some(a), Some(b)) = (
            self.living_position(helper),
            self.living_position(recipient),
        ) else {
            return;
        };
        let seen = self.player.alive()
            && (helper == PLAYER
                || recipient == PLAYER
                || (self.player_sees(a) && self.player_sees(b)));
        self.reciprocity
            .as_mut()
            .unwrap()
            .record(self.tick, helper, recipient, kind, b, seen);
    }
    fn local_partner(&self, id: ActorId) -> Result<(), Rejection> {
        self.npc(id)
            .filter(|n| n.alive())
            .ok_or(Rejection::UnknownActor)?;
        if !self.at_favor_actor(id) {
            return Err(Rejection::OutOfReach);
        }
        Ok(())
    }
    fn safe_donor(&self, id: ActorId) -> bool {
        self.npc(id).is_some_and(|n| {
            n.alive()
                && n.hp == n.definition.hp
                && !self.injuries().unwrap().people[&id].injured()
                && n.fatigue < n.definition.rest_threshold
                && !matches!(
                    n.activity.goal,
                    Goal::Retreat
                        | Goal::Retaliate
                        | Goal::SeekCare
                        | Goal::SeekWoundCare
                        | Goal::GatherForCare
                        | Goal::SellForCare
                        | Goal::Treat(_)
                )
                && !self.sect_duty_active(id)
        })
    }
    fn takes_part(&self, a: ActorId, b: ActorId) -> bool {
        self.social.relationship(a, b).familiarity > 0
            && !self.social.relationship(a, b).refuses_aid()
            && !self.social.relationship(b, a).refuses_aid()
    }
    /// Ask, never seize. Keep one coin for the donor's own care and one
    /// packed herb for travel emergencies. Neither credit nor trust forces aid.
    pub(super) fn request_aid(&mut self, id: ActorId) -> Result<Outcome, Rejection> {
        self.local_partner(id)?;
        if self.reciprocity().unwrap().net(PLAYER, id) <= 0
            || !self.takes_part(id, PLAYER)
            || !self.safe_donor(id)
        {
            return Err(Rejection::Refused);
        }
        let person = self.character_agency().unwrap().people[&id];
        let asset = if person.coins > RESERVE {
            self.character_agency
                .as_mut()
                .unwrap()
                .people
                .get_mut(&id)
                .unwrap()
                .coins -= 1;
            self.region.as_mut().unwrap().coins += 1;
            self.remember_aid(id, PLAYER, Kind::ReturnedCoin);
            Asset::Coin
        } else if u32::from(self.npcs[&id].packed_herbs) > RESERVE {
            self.npcs.get_mut(&id).unwrap().packed_herbs -= 1;
            self.player.herbs += 1;
            self.remember_aid(id, PLAYER, Kind::ReturnedHerb);
            Asset::Herb
        } else {
            return Err(Rejection::Empty);
        };
        Ok(Outcome::ReciprocalAid { actor: id, asset })
    }
    /// A voluntary return of a coin, only against actual outstanding gratitude.
    pub(super) fn repay_aid(&mut self, id: ActorId) -> Result<Outcome, Rejection> {
        self.local_partner(id)?;
        if self.reciprocity().unwrap().net(id, PLAYER) <= 0 || !self.takes_part(PLAYER, id) {
            return Err(Rejection::Refused);
        }
        if self.region().unwrap().coins == 0 {
            return Err(Rejection::NotEnoughCoins);
        }
        self.region.as_mut().unwrap().coins -= 1;
        self.character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&id)
            .unwrap()
            .coins += 1;
        self.remember_aid(PLAYER, id, Kind::RepaidCoin);
        Ok(Outcome::AidRepaid(id))
    }
    /// Forgive a whole claim face-to-face; no material or trust reward.
    pub(super) fn waive_aid(&mut self, id: ActorId) -> Result<Outcome, Rejection> {
        self.local_partner(id)?;
        if self.reciprocity().unwrap().net(PLAYER, id) <= 0 {
            return Err(Rejection::NoEvidence);
        }
        self.remember_aid(PLAYER, id, Kind::Released);
        Ok(Outcome::AidWaived(id))
    }
    fn care_need(&self, id: ActorId) -> bool {
        let n = &self.npcs[&id];
        let p = self.character_agency().unwrap().people[&id];
        n.alive()
            && self.injuries().unwrap().people[&id].injured()
            && p.coins < CLINIC_FEE
            && (p.livelihood_herbs == 0 || n.herbs == 0)
    }
    fn neighbor_asset(&self, giver: ActorId, patient: ActorId) -> Option<Asset> {
        if giver == patient
            || !self.care_need(patient)
            || !self.safe_donor(giver)
            || !self.takes_part(giver, patient)
        {
            return None;
        }
        let a = self.npcs[&giver].position;
        let b = self.npcs[&patient].position;
        if !in_reach(a, b) || !self.perceives(a, b) {
            return None;
        }
        let relation = self.social.relationship(giver, patient);
        let owes = self.reciprocity().unwrap().net(patient, giver) > 0;
        if !owes && (relation.regard < 2 || relation.familiarity < 2) {
            return None;
        }
        let p = self.character_agency().unwrap().people[&giver];
        if p.coins > RESERVE {
            Some(Asset::Coin)
        } else if u32::from(p.livelihood_herbs) > RESERVE && self.npcs[&giver].herbs > RESERVE {
            Some(Asset::Herb)
        } else {
            None
        }
    }
    /// One optional local relief transfer per participant, after ordinary
    /// NPC commits. Plan from a single snapshot; revalidate before mutation.
    /// Never replaces medicine, movement, duty or the patient's livelihood AI.
    pub(super) fn advance_reciprocity(&mut self) {
        if self.reciprocity.is_none() {
            return;
        }
        let needy: Vec<_> = self
            .npcs()
            .filter(|n| self.care_need(n.id()))
            .map(|n| n.id())
            .collect();
        if needy.is_empty() {
            return;
        }
        self.reconcile_character_agency_inventory();
        let mut plan = Vec::new();
        for n in self.npcs() {
            let giver = n.id();
            for &patient in &needy {
                if self.neighbor_asset(giver, patient).is_some() {
                    let owes = self.reciprocity().unwrap().net(patient, giver) > 0;
                    let severe = self.injuries().unwrap().people[&patient].severe();
                    plan.push((!owes, !severe, giver, patient));
                }
            }
        }
        plan.sort_unstable();
        let mut used = BTreeSet::new();
        for (_, _, giver, patient) in plan {
            if used.contains(&giver) || used.contains(&patient) {
                continue;
            }
            let Some(asset) = self.neighbor_asset(giver, patient) else {
                continue;
            };
            let repaying = self.reciprocity().unwrap().net(patient, giver) > 0;
            let agency = self.character_agency.as_mut().unwrap();
            match asset {
                Asset::Coin => {
                    agency.people.get_mut(&giver).unwrap().coins -= 1;
                    agency.people.get_mut(&patient).unwrap().coins += 1;
                }
                Asset::Herb => {
                    agency.people.get_mut(&giver).unwrap().livelihood_herbs -= 1;
                    agency.people.get_mut(&patient).unwrap().livelihood_herbs += 1;
                    self.npcs.get_mut(&giver).unwrap().herbs -= 1;
                    self.npcs.get_mut(&patient).unwrap().herbs += 1;
                }
            }
            if !repaying {
                self.social.aided(giver, patient);
            }
            self.remember_aid(
                giver,
                patient,
                match asset {
                    Asset::Coin => Kind::NeighborCoin,
                    Asset::Herb => Kind::NeighborHerb,
                },
            );
            used.insert(giver);
            used.insert(patient);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{
        Direction, Intent, LoadError, PLAYER_HP, StepError, Wounds, company::Journey,
    };
    const MEDIC: ActorId = ActorId::new(1);
    const PARTNER: ActorId = ActorId::new(22);
    fn adjacent(w: &World, p: Position, skip: Option<Position>) -> Position {
        [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ]
        .into_iter()
        .filter_map(|d| super::super::navigation::moved(p, d))
        .find(|&q| {
            w.walkable(q)
                && Some(q) != skip
                && !w
                    .npcs()
                    .any(|n| n.alive() && n.id() != PARTNER && n.position == q)
        })
        .unwrap()
    }
    fn fixture() -> World {
        let mut w = World::reciprocal_bonds_campaign(7);
        let clinic = w.scenario.clinic;
        w.npcs.get_mut(&MEDIC).unwrap().position = clinic;
        let p = adjacent(&w, clinic, None);
        w.npcs.get_mut(&PARTNER).unwrap().position = p;
        w.player.position = adjacent(&w, p, Some(clinic));
        w.social.meet(PARTNER, PLAYER);
        w.npcs.get_mut(&PARTNER).unwrap().activity.goal = Goal::Idle;
        w.npcs.get_mut(&MEDIC).unwrap().activity.goal = Goal::Idle;
        w.company.as_mut().unwrap().journey = Some(Journey {
            actor: PARTNER,
            last_seen: w.player.position,
            seen_at: w.tick,
        });
        assert!(w.at_favor_actor(PARTNER));
        w
    }
    fn fund(w: &mut World, id: ActorId, amount: u32) {
        w.region.as_mut().unwrap().market_coins -= amount;
        w.character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&id)
            .unwrap()
            .coins += amount;
    }
    fn total_coins(w: &World) -> u32 {
        let r = w.region().unwrap();
        let a = w.character_agency().unwrap();
        r.coins
            + r.market_coins
            + w.frontier().unwrap().depot_coins
            + w.politics().unwrap().hall_coins
            + w.rivalry().unwrap().lodge_coins
            + a.clinic_coins
            + a.people.values().map(|p| p.coins).sum::<u32>()
            + w.requests().unwrap().escrow()
    }
    fn net(w: &World, creditor: ActorId, debtor: ActorId) -> i8 {
        w.reciprocity().unwrap().net(creditor, debtor)
    }
    #[test]
    fn bounded_bilateral_balance_and_separate_observed_history() {
        let mut r = Reciprocity::default();
        let at = Position { x: 1, y: 2 };
        for i in 0..5 {
            r.record(i, PLAYER, PARTNER, Kind::Provision, at, true);
        }
        assert_eq!(r.net(PLAYER, PARTNER), BALANCE_CAP);
        assert_eq!(r.net(PARTNER, PLAYER), -BALANCE_CAP);
        assert_eq!(r.history().back().unwrap().change, 0);
        r.record(5, PARTNER, PLAYER, Kind::ReturnedHerb, at, true);
        assert_eq!(r.net(PLAYER, PARTNER), 2);
        r.record(6, PLAYER, PARTNER, Kind::Released, at, true);
        assert_eq!(r.net(PLAYER, PARTNER), 0);
        assert_eq!(r.balances().count(), 0);
        assert_eq!(r.history().back().unwrap().change, -2);
        let seen = r.observed().clone();
        for i in 7..200 {
            r.record(i, MEDIC, PARTNER, Kind::ClinicCare, at, false);
        }
        assert_eq!(r.history().len(), HISTORY_CAP);
        assert_eq!(r.observed(), &seen);
        assert_eq!(r.net(MEDIC, PARTNER), BALANCE_CAP);
        assert_eq!(r.net(PLAYER, PARTNER), 0);
        let prior = r.clone();
        r.record(201, MEDIC, MEDIC, Kind::ClinicCare, at, true);
        assert_eq!(r, prior);
        for i in 202..300 {
            r.record(i, PLAYER, PARTNER, Kind::Provision, at, true);
        }
        assert_eq!(r.observed().len(), OBSERVED_CAP);
        assert!(r.observed().iter().all(|e| e.helper == PLAYER));
        assert!(
            r.history()
                .iter()
                .zip(r.history().iter().skip(1))
                .all(|(a, b)| a.id + 1 == b.id)
        );
    }
    #[test]
    fn ordinary_packing_return_reserves_and_waiver_do_not_mint_resources() {
        let mut w = fixture();
        w.player.herbs = 2;
        let relation = w.social.clone();
        let money = total_coins(&w);
        assert_eq!(w.request_aid(PARTNER), Err(Rejection::Refused));
        w.provision(PARTNER).unwrap();
        assert_eq!(net(&w, PLAYER, PARTNER), 1);
        let before = w.clone();
        assert_eq!(w.request_aid(PARTNER), Err(Rejection::Empty));
        assert_eq!(w, before);
        w.provision(PARTNER).unwrap();
        assert_eq!(net(&w, PLAYER, PARTNER), 2);
        assert_eq!(
            w.request_aid(PARTNER),
            Ok(Outcome::ReciprocalAid {
                actor: PARTNER,
                asset: Asset::Herb
            })
        );
        assert_eq!(w.player.herbs, 1);
        assert_eq!(w.npcs[&PARTNER].packed_herbs, 1);
        assert_eq!(net(&w, PLAYER, PARTNER), 1);
        assert_eq!(w.social, relation);
        assert_eq!(w.waive_aid(PARTNER), Ok(Outcome::AidWaived(PARTNER)));
        assert_eq!(net(&w, PLAYER, PARTNER), 0);
        assert_eq!(w.social, relation);
        assert_eq!(total_coins(&w), money);
        let before = w.clone();
        assert_eq!(w.waive_aid(PARTNER), Err(Rejection::NoEvidence));
        assert_eq!(w.request_aid(PARTNER), Err(Rejection::Refused));
        assert_eq!(w, before);
        assert_eq!(w.known_aid().count(), 4);
    }
    #[test]
    fn actual_field_care_cancels_gratitude_and_keeps_ordinary_care_effect() {
        let mut w = fixture();
        w.player.herbs = 1;
        w.provision(PARTNER).unwrap();
        w.player.hp = PLAYER_HP - 2;
        let consumed = w.company().unwrap().herbs_used;
        w.field_care(PARTNER);
        assert_eq!(w.player.hp, PLAYER_HP);
        assert_eq!(w.npcs[&PARTNER].packed_herbs, 0);
        assert_eq!(w.company().unwrap().herbs_used, consumed + 1);
        assert_eq!(net(&w, PLAYER, PARTNER), 0);
        assert_eq!(w.known_aid().last().unwrap().kind, Kind::FieldCare);
        assert_eq!(w.social.relationship(PLAYER, PARTNER).regard, 1);
        let before = w.clone();
        w.field_care(PARTNER);
        assert_eq!(w, before);
    }
    #[test]
    fn real_clinic_care_and_voluntary_player_coin_return() {
        let mut w = fixture();
        w.player.position = adjacent(&w, w.scenario.clinic, Some(w.npcs[&PARTNER].position));
        w.player.hp = PLAYER_HP - 2;
        w.supply.shrine = 2;
        let money = total_coins(&w);
        let before = w.supply.consumed;
        assert_eq!(
            w.talk(MEDIC),
            Ok(Outcome::Treated {
                by: MEDIC,
                healed: 2
            })
        );
        assert_eq!(w.supply.consumed, before + 1);
        assert_eq!(net(&w, MEDIC, PLAYER), 1);
        let purse = w.region().unwrap().coins;
        let prior = w.clone();
        if purse == 0 {
            assert_eq!(w.repay_aid(MEDIC), Err(Rejection::NotEnoughCoins));
            assert_eq!(w, prior);
        }
        w.region.as_mut().unwrap().market_coins -= 1;
        w.region.as_mut().unwrap().coins += 1;
        let social = w.social.clone();
        assert_eq!(w.repay_aid(MEDIC), Ok(Outcome::AidRepaid(MEDIC)));
        assert_eq!(net(&w, MEDIC, PLAYER), 0);
        assert_eq!(w.region().unwrap().coins, purse);
        assert_eq!(w.character_agency().unwrap().people[&MEDIC].coins, 1);
        assert_eq!(total_coins(&w), money);
        assert_eq!(w.social, social);
    }
    #[test]
    fn real_coin_return_respects_own_care_reserve_and_hostility() {
        let mut w = fixture();
        w.player.herbs = 1;
        w.provision(PARTNER).unwrap();
        fund(&mut w, PARTNER, 2);
        let total = total_coins(&w);
        w.npcs.get_mut(&PARTNER).unwrap().hp -= 1;
        let before = w.clone();
        assert_eq!(w.request_aid(PARTNER), Err(Rejection::Refused));
        assert_eq!(w, before);
        w.npcs.get_mut(&PARTNER).unwrap().hp += 1;
        w.social
            .relations
            .get_mut(&(PARTNER, PLAYER))
            .unwrap()
            .regard = -1;
        let before = w.clone();
        assert_eq!(w.request_aid(PARTNER), Err(Rejection::Refused));
        assert_eq!(w, before);
        w.social
            .relations
            .get_mut(&(PARTNER, PLAYER))
            .unwrap()
            .regard = 0;
        let purse = w.region().unwrap().coins;
        let social = w.social.clone();
        assert_eq!(
            w.request_aid(PARTNER),
            Ok(Outcome::ReciprocalAid {
                actor: PARTNER,
                asset: Asset::Coin
            })
        );
        assert_eq!(w.region().unwrap().coins, purse + 1);
        assert_eq!(w.character_agency().unwrap().people[&PARTNER].coins, 1);
        assert_eq!(net(&w, PLAYER, PARTNER), 0);
        assert_eq!(w.social, social);
        assert_eq!(total_coins(&w), total);
    }
    #[test]
    fn autonomous_npc_repayment_pays_real_clinic_fee_without_an_extra_debt() {
        let mut w = fixture();
        let n = w.npcs.get_mut(&MEDIC).unwrap();
        n.hp -= 1;
        w.injuries
            .as_mut()
            .unwrap()
            .people
            .insert(MEDIC, Wounds { arms: 1, legs: 0 });
        w.supply.shrine = 2;
        w.social.meet(PARTNER, MEDIC);
        w.remember_aid(MEDIC, PARTNER, Kind::ClinicCare);
        fund(&mut w, PARTNER, 2);
        let total = total_coins(&w);
        let social = w.social.clone();
        let before = w.supply.shrine;
        w.advance_reciprocity();
        assert_eq!(w.character_agency().unwrap().people[&PARTNER].coins, 1);
        assert_eq!(w.character_agency().unwrap().people[&MEDIC].coins, 1);
        assert_eq!(net(&w, MEDIC, PARTNER), 0);
        assert_eq!(w.social, social);
        assert_eq!(total_coins(&w), total);
        assert!(w.injuries().unwrap().people[&MEDIC].injured());
        assert_eq!(w.supply.shrine, before);
        let snap = w.clone();
        w.advance_reciprocity();
        assert_eq!(w, snap);
        w.work_wound_care(MEDIC);
        assert!(!w.injuries().unwrap().people[&MEDIC].injured());
        assert_eq!(w.supply.shrine, before - 1);
        assert_eq!(total_coins(&w), total);
        assert_eq!(net(&w, MEDIC, PARTNER), 0);
    }
    #[test]
    fn autonomous_friend_gives_only_personal_surplus_herbs_and_private_event_stays_private() {
        let mut w = fixture();
        let n = w.npcs.get_mut(&MEDIC).unwrap();
        n.hp -= 1;
        w.injuries
            .as_mut()
            .unwrap()
            .people
            .insert(MEDIC, Wounds { arms: 1, legs: 1 });
        w.npcs.get_mut(&PARTNER).unwrap().herbs = 2;
        w.character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&PARTNER)
            .unwrap()
            .livelihood_herbs = 2;
        w.social.link(PARTNER, MEDIC);
        w.social.link(PARTNER, MEDIC);
        w.player.position = w.scenario.patch;
        assert!(!w.player_sees(w.npcs[&MEDIC].position));
        let before = w.social.relationship(MEDIC, PARTNER).regard;
        w.advance_reciprocity();
        assert_eq!(w.npcs[&PARTNER].herbs, 1);
        assert_eq!(w.npcs[&MEDIC].herbs, 1);
        assert_eq!(
            w.character_agency().unwrap().people[&PARTNER].livelihood_herbs,
            1
        );
        assert_eq!(
            w.character_agency().unwrap().people[&MEDIC].livelihood_herbs,
            1
        );
        assert_eq!(net(&w, PARTNER, MEDIC), 1);
        assert_eq!(
            w.social.relationship(MEDIC, PARTNER).regard,
            (before + 1).min(8)
        );
        assert_eq!(
            w.reciprocity().unwrap().history().back().unwrap().kind,
            Kind::NeighborHerb
        );
        assert!(w.known_bonds().is_empty() && w.known_aid().next().is_none());
        w.player.position = adjacent(&w, w.npcs[&PARTNER].position, Some(w.npcs[&MEDIC].position));
        assert!(w.known_bonds().is_empty() && w.known_aid().next().is_none());
        let snap = w.clone();
        w.advance_reciprocity();
        assert_eq!(w, snap);
    }
    #[test]
    fn competing_donors_prioritize_actual_reciprocity_and_never_spend_twice() {
        let mut w = fixture();
        let other = ActorId::new(2);
        let pos = adjacent(&w, w.scenario.clinic, Some(w.npcs[&PARTNER].position));
        w.npcs.get_mut(&other).unwrap().position = pos;
        w.npcs.get_mut(&other).unwrap().activity.goal = Goal::Idle;
        w.npcs.get_mut(&MEDIC).unwrap().hp -= 1;
        w.injuries
            .as_mut()
            .unwrap()
            .people
            .insert(MEDIC, Wounds { arms: 1, legs: 0 });
        w.social.link(other, MEDIC);
        w.social.link(other, MEDIC);
        w.social.meet(PARTNER, MEDIC);
        w.remember_aid(MEDIC, PARTNER, Kind::ClinicCare);
        fund(&mut w, other, 2);
        fund(&mut w, PARTNER, 2);
        assert_eq!(w.neighbor_asset(other, MEDIC), Some(Asset::Coin));
        assert_eq!(w.neighbor_asset(PARTNER, MEDIC), Some(Asset::Coin));
        let money = total_coins(&w);
        let mut twin = w.clone();
        w.advance_reciprocity();
        twin.advance_reciprocity();
        assert_eq!(w, twin);
        let agency = w.character_agency().unwrap();
        assert_eq!(agency.people[&other].coins, 2);
        assert_eq!(agency.people[&PARTNER].coins, 1);
        assert_eq!(agency.people[&MEDIC].coins, 1);
        assert_eq!(net(&w, MEDIC, PARTNER), 0);
        assert_eq!(w.reciprocity().unwrap().history().len(), 2);
        assert_eq!(
            w.reciprocity().unwrap().history().back().unwrap().helper,
            PARTNER
        );
        assert_eq!(total_coins(&w), money);
    }
    #[test]
    fn public_freight_is_not_personal_spare_medicine() {
        let mut w = fixture();
        w.npcs.get_mut(&MEDIC).unwrap().hp -= 1;
        w.injuries
            .as_mut()
            .unwrap()
            .people
            .insert(MEDIC, Wounds { arms: 1, legs: 0 });
        w.npcs.get_mut(&PARTNER).unwrap().herbs = 2;
        w.social.meet(PARTNER, MEDIC);
        w.remember_aid(MEDIC, PARTNER, Kind::ClinicCare);
        let before = w.clone();
        w.advance_reciprocity();
        assert_eq!(w, before);
        w.character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&PARTNER)
            .unwrap()
            .livelihood_herbs = 2;
        w.advance_reciprocity();
        assert_eq!(w.npcs[&PARTNER].herbs, 1);
        assert_eq!(w.npcs[&MEDIC].herbs, 1);
        assert_eq!(net(&w, MEDIC, PARTNER), 0);
    }
    #[test]
    fn witnessed_third_party_aid_reveals_act_not_private_balance() {
        let mut w = fixture();
        assert!(w.player_sees(w.npcs[&MEDIC].position));
        assert!(w.player_sees(w.npcs[&PARTNER].position));
        w.remember_aid(MEDIC, PARTNER, Kind::ClinicCare);
        let known = w.known_aid().last().unwrap();
        assert_eq!(known.kind, Kind::ClinicCare);
        assert_eq!((known.balance, known.change), (None, None));
        assert_eq!(net(&w, MEDIC, PARTNER), 1);
        assert!(w.known_bonds().is_empty());
        w.player.herbs = 1;
        w.provision(PARTNER).unwrap();
        let known = w.known_aid().last().unwrap();
        assert_eq!((known.balance, known.change), (Some(1), Some(1)));
    }
    #[test]
    fn local_and_temporal_admission_and_state_digest_are_bound() {
        let mut w = fixture();
        w.player.herbs = 1;
        w.provision(PARTNER).unwrap();
        w.player.position = w.scenario.patch;
        let before = w.clone();
        assert_eq!(w.request_aid(PARTNER), Err(Rejection::OutOfReach));
        assert_eq!(w.waive_aid(PARTNER), Err(Rejection::OutOfReach));
        assert_eq!(w, before);
        let mut old = World::personal_favors_campaign(7);
        let bytes = old.checkpoint();
        for i in [
            Intent::RequestAid(PARTNER),
            Intent::RepayAid(PARTNER),
            Intent::WaiveAid(PARTNER),
        ] {
            assert_eq!(old.step(0, i), Err(StepError::UnrecordableIntent));
            assert_eq!(old.checkpoint(), bytes);
        }
        let mut actual = World::reciprocal_bonds_campaign(7);
        assert_eq!(
            actual.step(0, Intent::RequestAid(ActorId::new(u32::MAX))),
            Ok(Outcome::Rejected(Rejection::UnknownActor))
        );
        assert_eq!(&actual.checkpoint()[54..59], &[51, 255, 255, 255, 255]);
        assert_eq!(World::restore(&actual.checkpoint()).unwrap(), actual);
        let mut corrupt = actual.clone();
        corrupt.reciprocity.as_mut().unwrap().next_event += 1;
        assert_ne!(actual.state_digest(), corrupt.state_digest());
        assert_eq!(
            World::restore(&corrupt.checkpoint()),
            Err(LoadError::StateMismatch)
        );
    }
}
