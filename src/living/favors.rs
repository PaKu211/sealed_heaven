//! Situated personal requests. An actual conversation opens an optional
//! promise to a living actor, never an omniscient quest board. Fulfilment
//! transfers an existing resource; ordinary AI decides how it is used.
use super::{
    ActorId, Outcome, PLAYER, Rejection, World, character_agency::CLINIC_FEE, navigation::in_reach,
};
use std::collections::{BTreeMap, VecDeque};

pub const REVISION: u16 = 34;
pub const VERSION: u16 = 1;
pub const LIFETIME: u64 = 180;
pub const COOLDOWN: u64 = 96;
pub const RECEIPT_CAP: usize = 48;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Need {
    CareFunds,
    TravelKit,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Status {
    Offered,
    Accepted,
    Fulfilled,
    Declined,
    Withdrawn,
    Expired,
    Changed,
}
impl Status {
    pub fn active(self) -> bool {
        matches!(self, Self::Offered | Self::Accepted)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Favor {
    pub id: u32,
    pub actor: ActorId,
    pub need: Need,
    pub opened: u64,
    pub deadline: u64,
    pub status: Status,
    pub closed: Option<u64>,
    /// Actual clamped change to this actor's regard for the player.
    pub regard_change: i8,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Seen {
    pub favor: Favor,
    pub at: u64,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Record {
    pub current: Option<Favor>,
    pub seen: Option<Seen>,
    pub next_open: u64,
    pub fulfilled: u32,
    pub withdrawn: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Favors {
    people: BTreeMap<ActorId, Record>,
    next_id: u32,
    receipts: VecDeque<Seen>,
}
impl Default for Favors {
    fn default() -> Self {
        Self {
            people: BTreeMap::new(),
            next_id: 1,
            receipts: VecDeque::new(),
        }
    }
}
impl Favors {
    pub fn record(&self, actor: ActorId) -> Option<&Record> {
        self.people.get(&actor)
    }
    pub fn records(&self) -> impl Iterator<Item = (&ActorId, &Record)> {
        self.people.iter()
    }
    pub fn receipts(&self) -> &VecDeque<Seen> {
        &self.receipts
    }
    fn remember(&mut self, favor: Favor, at: u64) {
        let record = self
            .people
            .get_mut(&favor.actor)
            .expect("known actor record");
        if !favor.status.active()
            && record
                .seen
                .is_some_and(|s| s.favor.id == favor.id && s.favor.status.active())
        {
            if self.receipts.len() == RECEIPT_CAP {
                self.receipts.pop_front();
            }
            self.receipts.push_back(Seen { favor, at });
        }
        record.seen = Some(Seen { favor, at });
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        fn favor(out: &mut Vec<u8>, f: Option<Favor>) {
            out.push(u8::from(f.is_some()));
            if let Some(f) = f {
                out.extend(f.id.to_le_bytes());
                out.extend(f.actor.get().to_le_bytes());
                out.push(f.need as u8);
                out.extend(f.opened.to_le_bytes());
                out.extend(f.deadline.to_le_bytes());
                out.push(f.status as u8);
                out.push(f.regard_change as u8);
                out.push(u8::from(f.closed.is_some()));
                if let Some(t) = f.closed {
                    out.extend(t.to_le_bytes());
                }
            }
        }
        fn seen(out: &mut Vec<u8>, s: Option<Seen>) {
            out.push(u8::from(s.is_some()));
            if let Some(s) = s {
                favor(out, Some(s.favor));
                out.extend(s.at.to_le_bytes());
            }
        }
        let mut out = b"SH:C47:FAVORS:v1\0".to_vec();
        out.extend(VERSION.to_le_bytes());
        out.extend(self.next_id.to_le_bytes());
        out.extend((self.people.len() as u32).to_le_bytes());
        for (&actor, r) in &self.people {
            out.extend(actor.get().to_le_bytes());
            favor(&mut out, r.current);
            seen(&mut out, r.seen);
            out.extend(r.next_open.to_le_bytes());
            out.extend(r.fulfilled.to_le_bytes());
            out.extend(r.withdrawn.to_le_bytes());
        }
        out.extend((self.receipts.len() as u32).to_le_bytes());
        for &s in &self.receipts {
            seen(&mut out, Some(s));
        }
        out
    }
}
impl World {
    pub fn favors(&self) -> Option<&Favors> {
        self.favors.as_ref()
    }
    pub fn at_favor_actor(&self, actor: ActorId) -> bool {
        self.player.alive()
            && self.npc(actor).is_some_and(|n| {
                n.alive()
                    && in_reach(self.player.position, n.position)
                    && self.player_sees(n.position)
            })
    }
    pub fn known_favors(&self) -> Vec<Seen> {
        self.favors().map_or_else(Vec::new, |f| {
            f.records().filter_map(|(_, r)| r.seen).collect()
        })
    }
    pub fn favor_receipts(&self) -> impl Iterator<Item = Seen> + '_ {
        self.favors()
            .into_iter()
            .flat_map(|f| f.receipts().iter().copied())
    }
    fn need_active(&self, actor: ActorId, need: Need) -> bool {
        let Some(n) = self.npc(actor).filter(|n| n.alive()) else {
            return false;
        };
        let relation = self.social.relationship(actor, PLAYER);
        if relation.familiarity == 0 || relation.refuses_aid() {
            return false;
        }
        match need {
            Need::CareFunds => {
                let Some(agency) = self.character_agency() else {
                    return false;
                };
                let state = agency.people[&actor];
                self.injuries().is_some_and(|i| i.people[&actor].injured())
                    && state.coins < CLINIC_FEE
                    && (state.livelihood_herbs == 0 || n.herbs == 0)
            }
            Need::TravelKit => self.companion(actor).is_some() && n.packed_herbs == 0,
        }
    }
    fn current_favor(&self, actor: ActorId) -> Option<Favor> {
        self.favors()?.record(actor)?.current
    }
    /// Successful ordinary dialogue is the sole discovery/offer trigger.
    pub(super) fn discuss_favor(&mut self, actor: ActorId) {
        if self.favors.is_none() || !self.at_favor_actor(actor) {
            return;
        }
        if let Some(r) = self.current_favor(actor) {
            // A return visit acknowledges any stale result before replacing it.
            self.favors.as_mut().unwrap().remember(r, self.tick);
            if r.status.active() {
                return;
            }
        }
        let record = self.favors().unwrap().record(actor);
        if self.tick >= super::MAX_TICKS || record.is_some_and(|r| self.tick < r.next_open) {
            return;
        }
        let Some(need) = [Need::CareFunds, Need::TravelKit]
            .into_iter()
            .find(|n| self.need_active(actor, *n))
        else {
            return;
        };
        let f = self.favors.as_mut().unwrap();
        let id = f.next_id;
        f.next_id += 1;
        let favor = Favor {
            id,
            actor,
            need,
            opened: self.tick,
            deadline: (self.tick + LIFETIME).min(super::MAX_TICKS),
            status: Status::Offered,
            closed: None,
            regard_change: 0,
        };
        f.people.entry(actor).or_default().current = Some(favor);
        f.remember(favor, self.tick);
    }
    fn close_favor(&mut self, actor: ActorId, status: Status, regard_change: i8) {
        let record = self
            .favors
            .as_mut()
            .unwrap()
            .people
            .get_mut(&actor)
            .unwrap();
        let f = record.current.as_mut().unwrap();
        assert!(f.status.active() && !status.active());
        f.status = status;
        f.closed = Some(self.tick);
        f.regard_change = regard_change;
        record.next_open = self.tick + COOLDOWN;
        if status == Status::Fulfilled {
            record.fulfilled += 1;
        }
        if status == Status::Withdrawn {
            record.withdrawn += 1;
        }
    }
    fn observe_favor(&mut self, actor: ActorId) {
        if self.at_favor_actor(actor)
            && let Some(f) = self.current_favor(actor)
        {
            self.favors.as_mut().unwrap().remember(f, self.tick);
        }
    }
    /// No new favors are generated by passive ticking. Existing promises
    /// remain subject to actual circumstances, not forced NPC schedules.
    pub(super) fn advance_favors(&mut self) {
        let Some(registry) = self.favors() else {
            return;
        };
        let actors: Vec<_> = registry.records().map(|(&actor, _)| actor).collect();
        for actor in actors {
            if let Some(f) = self.current_favor(actor).filter(|f| f.status.active()) {
                let close = if !self.player.alive() {
                    Some(Status::Changed)
                } else if self.tick >= f.deadline {
                    Some(Status::Expired)
                } else if !self.need_active(actor, f.need) {
                    Some(Status::Changed)
                } else {
                    None
                };
                if let Some(status) = close {
                    self.close_favor(actor, status, 0);
                }
            }
            self.observe_favor(actor);
        }
    }
    fn known_favor(&self, id: u32) -> Result<Seen, Rejection> {
        self.favors()
            .and_then(|f| {
                f.records()
                    .filter_map(|(_, r)| r.seen)
                    .find(|s| s.favor.id == id)
            })
            .ok_or(Rejection::NoEvidence)
    }
    fn local_favor(&self, id: u32, status: Status) -> Result<Favor, Rejection> {
        let known = self.known_favor(id)?;
        if !self.at_favor_actor(known.favor.actor) {
            return Err(Rejection::OutOfReach);
        }
        self.current_favor(known.favor.actor)
            .filter(|f| f.id == id && f.status == status && self.need_active(f.actor, f.need))
            .ok_or(Rejection::Refused)
    }
    pub(super) fn choose_favor(&mut self, id: u32, accept: bool) -> Result<Outcome, Rejection> {
        let f = self.local_favor(id, Status::Offered)?;
        if accept {
            self.favors
                .as_mut()
                .unwrap()
                .people
                .get_mut(&f.actor)
                .unwrap()
                .current
                .as_mut()
                .unwrap()
                .status = Status::Accepted;
        } else {
            self.close_favor(f.actor, Status::Declined, 0);
        }
        self.observe_favor(f.actor);
        Ok(if accept {
            Outcome::FavorAccepted(id)
        } else {
            Outcome::FavorDeclined(id)
        })
    }
    /// Withdrawing remotely is a personal journal choice. No magical message
    /// reaches the NPC and no unseen disposition/regard is revealed.
    /// Only a face-to-face withdrawal from an active promise can disappoint.
    pub(super) fn withdraw_favor(&mut self, id: u32) -> Result<Outcome, Rejection> {
        let mut f = self.known_favor(id)?.favor;
        if f.status != Status::Accepted {
            return Err(Rejection::Refused);
        }
        let local = self.at_favor_actor(f.actor);
        let active = self
            .current_favor(f.actor)
            .is_some_and(|r| r.id == id && r.status == Status::Accepted);
        let delta = if local && active {
            self.social.promise_disappointment(f.actor, PLAYER)
        } else {
            0
        };
        if active {
            self.close_favor(f.actor, Status::Withdrawn, delta);
        }
        f.status = Status::Withdrawn;
        f.closed = Some(self.tick);
        f.regard_change = delta;
        self.favors.as_mut().unwrap().remember(f, self.tick);
        Ok(Outcome::FavorWithdrawn(id))
    }
    fn fulfill_favor(&mut self, f: Favor) {
        let before = self.social.relationship(f.actor, PLAYER).regard;
        self.social.aided(PLAYER, f.actor);
        let delta = self.social.relationship(f.actor, PLAYER).regard - before;
        self.close_favor(f.actor, Status::Fulfilled, delta);
        let current = self.current_favor(f.actor).unwrap();
        self.favors.as_mut().unwrap().remember(current, self.tick);
    }
    /// Ordinary provision is the physical transfer; this only records its
    /// already accepted purpose. No extra herb or duplicate aid credit.
    pub(super) fn provisioned_for_favor(&mut self, actor: ActorId) {
        let Some(f) = self
            .current_favor(actor)
            .filter(|f| f.status.active() && f.need == Need::TravelKit)
        else {
            return;
        };
        if !self.at_favor_actor(actor) {
            return;
        }
        if f.status == Status::Accepted {
            self.fulfill_favor(f);
        } else {
            self.close_favor(actor, Status::Changed, 0);
            self.observe_favor(actor);
        }
    }
    /// A successful face-to-face separation is direct evidence, even when
    /// the former companion moves away during their later NPC commit.
    pub(super) fn parted_favor(&mut self, actor: ActorId) {
        if self.at_favor_actor(actor)
            && self
                .current_favor(actor)
                .is_some_and(|f| f.status.active() && f.need == Need::TravelKit)
        {
            self.close_favor(actor, Status::Changed, 0);
            self.observe_favor(actor);
        }
    }
    /// One real clinic fee, not healing on demand. The NPC retains its
    /// funds and uses ordinary autonomous paid-care rules.
    pub(super) fn fund_favor(&mut self, id: u32) -> Result<Outcome, Rejection> {
        let f = self.local_favor(id, Status::Accepted)?;
        if f.need != Need::CareFunds {
            return Err(Rejection::Refused);
        }
        if self.region().unwrap().coins < CLINIC_FEE {
            return Err(Rejection::NotEnoughCoins);
        }
        self.region.as_mut().unwrap().coins -= CLINIC_FEE;
        self.character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&f.actor)
            .unwrap()
            .coins += CLINIC_FEE;
        self.fulfill_favor(f);
        self.remember_aid(PLAYER, f.actor, super::reciprocity::Kind::CareFunding);
        Ok(Outcome::FavorFunded {
            id,
            actor: f.actor,
            coins: CLINIC_FEE,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Direction, Intent, Position, StepError, Wounds, company::Journey};
    const MEDIC: ActorId = ActorId::new(1);
    const PATIENT: ActorId = ActorId::new(22);

    fn adjacent(w: &World, p: Position, exclude: Option<Position>) -> Position {
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
                && Some(q) != exclude
                && !w
                    .npcs()
                    .any(|n| n.alive() && n.id() != PATIENT && n.position == q)
        })
        .unwrap()
    }
    fn fixture(need: Need) -> World {
        let mut w = World::personal_favors_campaign(7);
        let clinic = w.scenario.clinic;
        w.npcs.get_mut(&MEDIC).unwrap().position = clinic;
        let patient = adjacent(&w, clinic, None);
        w.npcs.get_mut(&PATIENT).unwrap().position = patient;
        w.player.position = adjacent(&w, patient, Some(clinic));
        w.social.meet(PATIENT, PLAYER);
        w.region.as_mut().unwrap().market_coins -= 3;
        w.region.as_mut().unwrap().coins += 3;
        match need {
            Need::CareFunds => {
                let n = w.npcs.get_mut(&PATIENT).unwrap();
                n.hp = n.definition.hp - 1;
                w.injuries
                    .as_mut()
                    .unwrap()
                    .people
                    .insert(PATIENT, Wounds { arms: 1, legs: 0 });
                w.supply.shrine = 2;
            }
            Need::TravelKit => {
                w.company.as_mut().unwrap().journey = Some(Journey {
                    actor: PATIENT,
                    last_seen: w.player.position,
                    seen_at: w.tick,
                });
            }
        }
        assert!(w.at_favor_actor(PATIENT));
        assert!(w.need_active(PATIENT, need));
        w
    }
    fn offer(w: &mut World) -> u32 {
        w.discuss_favor(PATIENT);
        w.current_favor(PATIENT).unwrap().id
    }
    fn coins(w: &World) -> u32 {
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
    #[test]
    fn clinic_fee_transfers_once_then_ordinary_medic_consumes_real_stock() {
        let mut w = fixture(Need::CareFunds);
        assert!(w.known_favors().is_empty());
        w.advance_favors();
        assert!(w.known_favors().is_empty());
        let id = offer(&mut w);
        let total = coins(&w);
        let hp = w.npcs[&PATIENT].hp;
        let stock = w.supply.shrine;
        let consumed = w.supply.consumed;
        let regard = w.social.relationship(PATIENT, PLAYER).regard;
        let wallet = w.region().unwrap().coins;
        let before = w.clone();
        assert_eq!(w.fund_favor(id), Err(Rejection::Refused));
        assert_eq!(w, before);
        assert_eq!(w.choose_favor(id, true), Ok(Outcome::FavorAccepted(id)));
        assert_eq!(
            w.fund_favor(id),
            Ok(Outcome::FavorFunded {
                id,
                actor: PATIENT,
                coins: CLINIC_FEE
            })
        );
        assert_eq!(w.region().unwrap().coins, wallet - CLINIC_FEE);
        assert_eq!(
            w.character_agency().unwrap().people[&PATIENT].coins,
            CLINIC_FEE
        );
        assert_eq!(coins(&w), total);
        assert_eq!(w.npcs[&PATIENT].hp, hp);
        assert_eq!(w.supply.shrine, stock);
        assert!(w.injuries().unwrap().people[&PATIENT].injured());
        assert_eq!(w.social.relationship(PATIENT, PLAYER).regard, regard + 1);
        let f = w.current_favor(PATIENT).unwrap();
        assert_eq!(f.status, Status::Fulfilled);
        assert_eq!(f.regard_change, 1);
        assert_eq!(w.favors().unwrap().receipts().len(), 1);
        let before = w.clone();
        assert_eq!(w.fund_favor(id), Err(Rejection::Refused));
        assert_eq!(w, before);
        // Invoke exactly the existing autonomous care commit, not a substitute heal.
        w.work_wound_care(PATIENT);
        assert_eq!(w.character_agency().unwrap().people[&PATIENT].coins, 0);
        assert_eq!(
            w.character_agency().unwrap().people[&MEDIC].coins,
            CLINIC_FEE
        );
        assert!(!w.injuries().unwrap().people[&PATIENT].injured());
        assert_eq!(w.npcs[&PATIENT].hp, hp + 1);
        assert_eq!(w.supply.shrine, stock - 1);
        assert_eq!(w.supply.consumed, consumed + 1);
        assert_eq!(coins(&w), total);
    }
    #[test]
    fn insufficient_wallet_and_existing_barter_wealth_do_not_force_aid() {
        let mut w = fixture(Need::CareFunds);
        let id = offer(&mut w);
        w.choose_favor(id, true).unwrap();
        let wallet = w.region().unwrap().coins;
        let r = w.region.as_mut().unwrap();
        r.coins = 0;
        r.market_coins += wallet;
        let before = w.clone();
        assert_eq!(w.fund_favor(id), Err(Rejection::NotEnoughCoins));
        assert_eq!(w, before);
        w.npcs.get_mut(&PATIENT).unwrap().herbs = 1;
        w.character_agency
            .as_mut()
            .unwrap()
            .people
            .get_mut(&PATIENT)
            .unwrap()
            .livelihood_herbs = 1;
        w.advance_favors();
        assert_eq!(w.current_favor(PATIENT).unwrap().status, Status::Changed);
        assert_eq!(
            w.favors()
                .unwrap()
                .receipts()
                .back()
                .unwrap()
                .favor
                .regard_change,
            0
        );
        assert_eq!(w.favors().unwrap().record(PATIENT).unwrap().fulfilled, 0);
    }
    #[test]
    fn accepted_kit_uses_ordinary_provision_once_and_clamps_regard() {
        let mut w = fixture(Need::TravelKit);
        w.player.herbs = 2;
        for _ in 0..8 {
            w.social.aided(PLAYER, PATIENT);
        }
        let id = offer(&mut w);
        w.choose_favor(id, true).unwrap();
        assert_eq!(w.fund_favor(id), Err(Rejection::Refused));
        assert_eq!(w.provision(PATIENT), Ok(Outcome::Provisioned(PATIENT)));
        assert_eq!(w.player.herbs, 1);
        assert_eq!(w.npcs[&PATIENT].packed_herbs, 1);
        assert_eq!(w.current_favor(PATIENT).unwrap().status, Status::Fulfilled);
        assert_eq!(w.current_favor(PATIENT).unwrap().regard_change, 0);
        assert_eq!(w.social.relationship(PATIENT, PLAYER).regard, 8);
        w.provision(PATIENT).unwrap();
        assert_eq!(w.player.herbs, 0);
        assert_eq!(w.npcs[&PATIENT].packed_herbs, 2);
        assert_eq!(w.favors().unwrap().record(PATIENT).unwrap().fulfilled, 1);
        assert_eq!(w.favors().unwrap().receipts().len(), 1);
    }
    #[test]
    fn declining_and_unpromised_pack_are_free_choices_not_social_penalties() {
        let mut w = fixture(Need::TravelKit);
        w.player.herbs = 2;
        let id = offer(&mut w);
        let social = w.social.clone();
        assert_eq!(w.choose_favor(id, false), Ok(Outcome::FavorDeclined(id)));
        assert_eq!(w.social, social);
        w.provision(PATIENT).unwrap();
        assert_eq!(w.social, social);
        assert_eq!(w.player.herbs, 1);
        assert_eq!(w.npcs[&PATIENT].packed_herbs, 1);
        assert_eq!(w.current_favor(PATIENT).unwrap().status, Status::Declined);
        w.discuss_favor(PATIENT);
        assert_eq!(w.current_favor(PATIENT).unwrap().id, id);
        assert_eq!(w.favors().unwrap().record(PATIENT).unwrap().fulfilled, 0);
        let mut w = fixture(Need::TravelKit);
        w.player.herbs = 1;
        offer(&mut w);
        w.provision(PATIENT).unwrap();
        w.advance_favors();
        assert_eq!(w.current_favor(PATIENT).unwrap().status, Status::Changed);
    }
    #[test]
    fn direct_separation_records_known_closure_before_npc_moves() {
        let mut w = fixture(Need::TravelKit);
        let id = offer(&mut w);
        w.choose_favor(id, true).unwrap();
        assert_eq!(w.part(PATIENT), Ok(Outcome::Parted(PATIENT)));
        w.npcs.get_mut(&PATIENT).unwrap().position = w.scenario.patch;
        assert!(!w.at_favor_actor(PATIENT));
        w.advance_favors();
        assert_eq!(w.known_favors()[0].favor.status, Status::Changed);
        assert_eq!(w.favors().unwrap().receipts().len(), 1);
        assert_eq!(w.favors().unwrap().receipts()[0].favor.id, id);
        assert_eq!(w.favors().unwrap().receipts()[0].favor.regard_change, 0);
    }
    #[test]
    fn local_withdrawal_costs_only_existing_positive_regard_and_never_hostility() {
        for positive in [false, true] {
            let mut w = fixture(Need::TravelKit);
            if positive {
                w.social.aided(PLAYER, PATIENT);
                w.social.aided(PLAYER, PATIENT);
            }
            let initial = w.social.relationship(PATIENT, PLAYER).regard;
            let id = offer(&mut w);
            w.choose_favor(id, true).unwrap();
            assert_eq!(w.withdraw_favor(id), Ok(Outcome::FavorWithdrawn(id)));
            let delta = if positive { -1 } else { 0 };
            assert_eq!(
                w.social.relationship(PATIENT, PLAYER).regard,
                initial + delta
            );
            assert_eq!(w.current_favor(PATIENT).unwrap().regard_change, delta);
            let before = w.clone();
            assert_eq!(w.withdraw_favor(id), Err(Rejection::Refused));
            assert_eq!(w, before);
        }
    }
    #[test]
    fn remote_closure_keeps_stale_memory_and_withdrawal_never_reveals_it() {
        let mut w = fixture(Need::TravelKit);
        let id = offer(&mut w);
        w.choose_favor(id, true).unwrap();
        w.player.position = w.scenario.patch;
        assert!(!w.at_favor_actor(PATIENT));
        let known = w.known_favors();
        let mut changed = w.clone();
        changed.company.as_mut().unwrap().journey = None;
        changed.advance_favors();
        assert_eq!(
            changed.current_favor(PATIENT).unwrap().status,
            Status::Changed
        );
        assert_eq!(changed.known_favors(), known);
        assert_eq!(changed.choose_favor(id, true), Err(Rejection::OutOfReach));
        assert_eq!(changed.fund_favor(id), Err(Rejection::OutOfReach));
        let social = w.social.clone();
        assert_eq!(changed.withdraw_favor(id), w.withdraw_favor(id));
        assert_eq!(changed.known_favors(), w.known_favors());
        assert_eq!(
            changed.favors().unwrap().receipts(),
            w.favors().unwrap().receipts()
        );
        assert_eq!(w.social, social);
        assert_eq!(changed.social, social);
    }
    #[test]
    fn hostility_and_death_close_promises_and_never_create_offers() {
        let mut w = fixture(Need::TravelKit);
        // The fixture writes a bounded negative relationship directly; public
        // incidents use the existing social evidence system to reach it.
        w.social
            .relations
            .get_mut(&(PATIENT, PLAYER))
            .unwrap()
            .regard = -1;
        w.discuss_favor(PATIENT);
        assert!(w.known_favors().is_empty());
        w.social
            .relations
            .get_mut(&(PATIENT, PLAYER))
            .unwrap()
            .regard = 0;
        let id = offer(&mut w);
        w.choose_favor(id, true).unwrap();
        w.npcs.get_mut(&PATIENT).unwrap().hp = 0;
        w.advance_favors();
        assert_eq!(w.current_favor(PATIENT).unwrap().status, Status::Changed);
        assert_eq!(w.known_favors()[0].favor.status, Status::Accepted);
        assert_eq!(w.fund_favor(id), Err(Rejection::OutOfReach));
        assert!(w.favors().unwrap().receipts().is_empty());
    }
    #[test]
    fn receipt_bound_cooldown_and_final_horizon_are_exact() {
        let mut w = fixture(Need::TravelKit);
        for i in 0..RECEIPT_CAP + 5 {
            let id = offer(&mut w);
            assert_eq!(id, i as u32 + 1);
            w.choose_favor(id, false).unwrap();
            w.tick += COOLDOWN - 1;
            w.discuss_favor(PATIENT);
            assert_eq!(w.current_favor(PATIENT).unwrap().id, id);
            w.tick += 1;
        }
        assert_eq!(w.favors().unwrap().receipts().len(), RECEIPT_CAP);
        let mut w = fixture(Need::TravelKit);
        w.tick = super::super::MAX_TICKS - 2;
        let id = offer(&mut w);
        w.choose_favor(id, true).unwrap();
        let f = w.current_favor(PATIENT).unwrap();
        assert_eq!(f.deadline, super::super::MAX_TICKS);
        w.tick = f.deadline - 1;
        w.advance_favors();
        assert_eq!(w.current_favor(PATIENT).unwrap().status, Status::Accepted);
        w.tick = f.deadline;
        w.advance_favors();
        assert_eq!(w.current_favor(PATIENT).unwrap().status, Status::Expired);
        w.discuss_favor(PATIENT);
        assert_eq!(w.current_favor(PATIENT).unwrap().id, id);
    }
    #[test]
    fn historical_intent_admission_and_replay_bind_personal_state() {
        let mut old = World::emergent_requests_campaign(7);
        let bytes = old.checkpoint();
        for i in [
            Intent::AcceptFavor(1),
            Intent::DeclineFavor(1),
            Intent::WithdrawFavor(1),
            Intent::FundFavor(1),
        ] {
            assert_eq!(old.step(0, i), Err(StepError::UnrecordableIntent));
            assert_eq!(old.checkpoint(), bytes);
        }
        let mut w = World::personal_favors_campaign(7);
        assert_eq!(
            w.step(0, Intent::FundFavor(u32::MAX)),
            Ok(Outcome::Rejected(Rejection::NoEvidence))
        );
        assert_eq!(&w.checkpoint()[54..59], &[50, 255, 255, 255, 255]);
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
        let mut corrupt = w.clone();
        corrupt.favors.as_mut().unwrap().next_id += 1;
        assert_ne!(corrupt.state_digest(), w.state_digest());
        assert_eq!(
            World::restore(&corrupt.checkpoint()),
            Err(crate::living::LoadError::StateMismatch)
        );
    }
}
