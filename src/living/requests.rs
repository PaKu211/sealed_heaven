//! Player-facing situated service requests. These supplement, never replace,
//! NPC contracts and physical freight. Only ordinary successful transfers
//! count as work. The market bonus is escrowed in its own ledger.
use super::content::{MARKET_RESERVE, MARKET_TARGET};
use super::{Outcome, Position, Region, Rejection, World, navigation::in_reach};
use std::collections::VecDeque;

pub const REVISION: u16 = 33;
pub const VERSION: u16 = 1;
pub const LIFETIME: u64 = 240;
pub const COOLDOWN: u64 = 96;
pub const RECEIPT_CAP: usize = 32;
const MARKET_BONUS: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Site {
    Shrine,
    Market,
}
impl Site {
    pub const ALL: [Self; 2] = [Self::Shrine, Self::Market];
    fn position(self, w: &World) -> Position {
        match self {
            Self::Shrine => w.scenario.shrine,
            Self::Market => Region::sites().market,
        }
    }
    fn stock_target(self, w: &World) -> (u32, u32) {
        match self {
            Self::Shrine => (w.supply.shrine, w.scenario.shrine_target),
            Self::Market => (w.region().unwrap().market_herbs, MARKET_TARGET),
        }
    }
    fn triggered(self, w: &World) -> bool {
        let (stock, target) = self.stock_target(w);
        stock < target
            && match self {
                Self::Shrine => stock <= 1,
                Self::Market => {
                    stock <= MARKET_RESERVE && w.region().unwrap().market_coins >= MARKET_BONUS
                }
            }
    }
    fn bonus(self) -> u32 {
        match self {
            Self::Shrine => 0,
            Self::Market => MARKET_BONUS,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Status {
    Offered,
    Accepted,
    Completed,
    Declined,
    Abandoned,
    Expired,
    Superseded,
}
impl Status {
    pub fn active(self) -> bool {
        matches!(self, Self::Offered | Self::Accepted)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Request {
    pub id: u32,
    pub site: Site,
    pub opened: u64,
    pub deadline: u64,
    pub required: u32,
    pub delivered: u32,
    pub status: Status,
    pub bonus: u32,
    pub escrow: u32,
    pub closed: Option<u64>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Seen {
    pub request: Request,
    pub at: u64,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Board {
    pub current: Option<Request>,
    pub seen: Option<Seen>,
    pub next_open: u64,
    pub completed: u32,
    pub paid: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Requests {
    boards: [Board; 2],
    next_id: u32,
    receipts: VecDeque<Seen>,
}
impl Default for Requests {
    fn default() -> Self {
        Self {
            boards: std::array::from_fn(|_| Board::default()),
            next_id: 1,
            receipts: VecDeque::new(),
        }
    }
}
impl Requests {
    pub fn board(&self, site: Site) -> &Board {
        &self.boards[site as usize]
    }
    pub fn receipts(&self) -> &VecDeque<Seen> {
        &self.receipts
    }
    pub fn escrow(&self) -> u32 {
        self.boards
            .iter()
            .filter_map(|b| b.current)
            .map(|r| r.escrow)
            .sum()
    }
    fn remember(&mut self, site: Site, request: Request, at: u64) {
        let b = &mut self.boards[site as usize];
        // Only a known active notice transitioning to terminal is a receipt.
        // Repeated inspection cannot duplicate it. A remote cancellation is
        // personal evidence, not access to the board's unseen actual outcome.
        if !request.status.active()
            && b.seen
                .is_some_and(|s| s.request.id == request.id && s.request.status.active())
        {
            if self.receipts.len() == RECEIPT_CAP {
                self.receipts.pop_front();
            }
            self.receipts.push_back(Seen { request, at });
        }
        b.seen = Some(Seen { request, at });
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        fn request(out: &mut Vec<u8>, r: Option<Request>) {
            out.push(u8::from(r.is_some()));
            if let Some(r) = r {
                out.extend(r.id.to_le_bytes());
                out.push(r.site as u8);
                out.extend(r.opened.to_le_bytes());
                out.extend(r.deadline.to_le_bytes());
                for n in [r.required, r.delivered, r.bonus, r.escrow] {
                    out.extend(n.to_le_bytes());
                }
                out.push(r.status as u8);
                out.push(u8::from(r.closed.is_some()));
                if let Some(t) = r.closed {
                    out.extend(t.to_le_bytes());
                }
            }
        }
        fn seen(out: &mut Vec<u8>, s: Option<Seen>) {
            out.push(u8::from(s.is_some()));
            if let Some(s) = s {
                request(out, Some(s.request));
                out.extend(s.at.to_le_bytes());
            }
        }
        let mut out = b"SH:C46:REQUESTS:v1\0".to_vec();
        out.extend(VERSION.to_le_bytes());
        out.extend(self.next_id.to_le_bytes());
        for b in &self.boards {
            request(&mut out, b.current);
            seen(&mut out, b.seen);
            out.extend(b.next_open.to_le_bytes());
            out.extend(b.completed.to_le_bytes());
            out.extend(b.paid.to_le_bytes());
        }
        out.extend((self.receipts.len() as u32).to_le_bytes());
        for &s in &self.receipts {
            seen(&mut out, Some(s));
        }
        out
    }
}
impl World {
    /// Developer/domain inspector. Normal presentation uses remembered notices.
    pub fn requests(&self) -> Option<&Requests> {
        self.requests.as_ref()
    }
    pub fn at_request_site(&self, site: Site) -> bool {
        let p = site.position(self);
        self.player_sees(p) && in_reach(self.player.position, p)
    }
    pub fn known_requests(&self) -> Vec<Seen> {
        self.requests().map_or_else(Vec::new, |q| {
            q.boards.iter().filter_map(|b| b.seen).collect()
        })
    }
    pub fn request_receipts(&self) -> impl Iterator<Item = Seen> + '_ {
        self.requests()
            .into_iter()
            .flat_map(|q| q.receipts().iter().copied())
    }
    fn close_request(&mut self, site: Site, status: Status) {
        debug_assert!(!status.active());
        let b = &mut self.requests.as_mut().unwrap().boards[site as usize];
        let r = b.current.as_mut().expect("active request");
        assert!(r.status.active());
        if r.escrow > 0 {
            let region = self.region.as_mut().unwrap();
            if status == Status::Completed {
                region.coins += r.escrow;
                b.paid += r.escrow;
            } else {
                region.market_coins += r.escrow;
            }
            r.escrow = 0;
        }
        r.status = status;
        r.closed = Some(self.tick);
        if status == Status::Completed {
            b.completed += 1;
        }
        b.next_open = self.tick + COOLDOWN;
    }
    /// Process authoritative needs before player action and after NPC commits.
    /// A closed accepted request is retained until the player learns its
    /// disposition (or withdraws); unrelated NPC work is never blocked.
    pub(super) fn advance_requests(&mut self) {
        if self.requests.is_none() {
            return;
        }
        for site in Site::ALL {
            let current = self.requests().unwrap().board(site).current;
            if let Some(r) = current.filter(|r| r.status.active()) {
                let (stock, target) = site.stock_target(self);
                let closure = if !self.player.alive() && r.status == Status::Accepted {
                    Some(Status::Abandoned)
                } else if self.tick >= r.deadline {
                    Some(Status::Expired)
                } else if stock >= target {
                    Some(Status::Superseded)
                } else {
                    None
                };
                if let Some(status) = closure {
                    self.close_request(site, status);
                }
            }
            let b = self.requests().unwrap().board(site);
            let available = b.current.is_none_or(|r| !r.status.active());
            let unacknowledged = b.seen.is_some_and(|s| s.request.status == Status::Accepted);
            if available
                && !unacknowledged
                && self.tick >= b.next_open
                && self.player.alive()
                && self.tick < super::MAX_TICKS
                && site.triggered(self)
            {
                let (stock, target) = site.stock_target(self);
                let q = self.requests.as_mut().unwrap();
                let id = q.next_id;
                q.next_id += 1;
                q.boards[site as usize].current = Some(Request {
                    id,
                    site,
                    opened: self.tick,
                    deadline: (self.tick + LIFETIME).min(super::MAX_TICKS),
                    required: (target - stock).min(2),
                    delivered: 0,
                    status: Status::Offered,
                    bonus: site.bonus(),
                    escrow: 0,
                    closed: None,
                });
            }
        }
        self.observe_requests();
    }
    fn observe_requests(&mut self) {
        if self.requests.is_none() {
            return;
        }
        for site in Site::ALL {
            if self.at_request_site(site)
                && let Some(r) = self.requests().unwrap().board(site).current
            {
                let q = self.requests.as_mut().unwrap();
                q.remember(site, r, self.tick);
            }
        }
    }
    fn known_request(&self, id: u32) -> Result<Seen, Rejection> {
        self.requests()
            .and_then(|q| {
                q.boards
                    .iter()
                    .filter_map(|b| b.seen)
                    .find(|s| s.request.id == id)
            })
            .ok_or(Rejection::NoEvidence)
    }
    pub(super) fn choose_request(&mut self, id: u32, accept: bool) -> Result<Outcome, Rejection> {
        let seen = self.known_request(id)?;
        let site = seen.request.site;
        if !self.at_request_site(site) {
            return Err(Rejection::OutOfReach);
        }
        let r = self
            .requests()
            .unwrap()
            .board(site)
            .current
            .filter(|r| r.id == id && r.status == Status::Offered)
            .ok_or(Rejection::Refused)?;
        if !accept {
            self.close_request(site, Status::Declined);
            self.observe_requests();
            return Ok(Outcome::RequestDeclined(id));
        }
        let budget = r.bonus
            + if site == Site::Market {
                r.required * self.market_quote().unwrap().sell
            } else {
                0
            };
        let region = self.region.as_mut().unwrap();
        if region.market_coins < budget {
            return Err(Rejection::NotEnoughCoins);
        }
        region.market_coins -= r.bonus;
        let current = self.requests.as_mut().unwrap().boards[site as usize]
            .current
            .as_mut()
            .unwrap();
        current.status = Status::Accepted;
        current.escrow = r.bonus;
        self.observe_requests();
        Ok(Outcome::RequestAccepted(id))
    }
    /// A player can withdraw from a remembered commitment anywhere. If the
    /// world has already closed it remotely, the response is still identical:
    /// personal withdrawal does not reveal hidden live state.
    pub(super) fn abandon_request(&mut self, id: u32) -> Result<Outcome, Rejection> {
        let seen = self.known_request(id)?;
        let mut r = seen.request;
        if r.status != Status::Accepted {
            return Err(Rejection::Refused);
        }
        if self
            .requests()
            .unwrap()
            .board(r.site)
            .current
            .is_some_and(|c| c.id == id && c.status == Status::Accepted)
        {
            self.close_request(r.site, Status::Abandoned);
        }
        r.status = Status::Abandoned;
        r.escrow = 0;
        r.closed = Some(self.tick);
        self.requests
            .as_mut()
            .unwrap()
            .remember(r.site, r, self.tick);
        Ok(Outcome::RequestAbandoned(id))
    }
    /// Called exactly once following an actual local transfer; no quest
    /// action mints goods. Ordinary transactions remain possible unaccepted.
    pub(super) fn request_contribution(&mut self, site: Site) {
        let Some(r) = self
            .requests
            .as_mut()
            .and_then(|q| q.boards[site as usize].current.as_mut())
        else {
            return;
        };
        if r.status != Status::Accepted {
            return;
        }
        r.delivered += 1;
        let quota_met = r.delivered == r.required;
        let (stock, target) = site.stock_target(self);
        if quota_met || stock >= target {
            self.close_request(site, Status::Completed);
        }
        self.observe_requests();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Intent, StepError};
    fn funded_fixture(site: Site) -> (World, u32) {
        let mut w = World::emergent_requests_campaign(7);
        w.player.position = site.position(&w);
        match site {
            Site::Market => w.region.as_mut().unwrap().market_herbs = 1,
            Site::Shrine => w.supply.shrine = 0,
        }
        w.requests = Some(Requests::default());
        w.advance_requests();
        let id = w.requests().unwrap().board(site).current.unwrap().id;
        assert!(w.known_requests().iter().any(|s| s.request.id == id));
        (w, id)
    }
    fn current(w: &World, site: Site) -> Request {
        w.requests().unwrap().board(site).current.unwrap()
    }
    fn treasury(w: &World) -> u32 {
        let r = w.region().unwrap();
        r.coins + r.market_coins + w.requests().unwrap().escrow()
    }
    #[test]
    fn market_commitment_escrows_refunds_and_pays_once() {
        let (mut w, id) = funded_fixture(Site::Market);
        let initial = treasury(&w);
        let before = w.region().unwrap().market_coins;
        assert_eq!(w.choose_request(id, true), Ok(Outcome::RequestAccepted(id)));
        assert_eq!(w.requests().unwrap().escrow(), 1);
        assert_eq!(w.region().unwrap().market_coins, before - 1);
        let accepted = w.clone();
        assert_eq!(w.choose_request(id, true), Err(Rejection::Refused));
        assert_eq!(w, accepted);
        // Execute the same real trade handler as the player's normal SellHerb.
        w.player.herbs = 2;
        let h = w.region().unwrap().market_herbs;
        let wallet = w.region().unwrap().coins;
        let first = w.market_quote().unwrap().sell;
        w.trade_herb(false).unwrap();
        assert_eq!(current(&w, Site::Market).delivered, 1);
        let second = w.market_quote().unwrap().sell;
        w.trade_herb(false).unwrap();
        assert_eq!(current(&w, Site::Market).status, Status::Completed);
        assert_eq!(w.player.herbs, 0);
        assert_eq!(w.region().unwrap().market_herbs, h + 2);
        assert_eq!(w.region().unwrap().coins, wallet + first + second + 1);
        assert_eq!(treasury(&w), initial);
        assert_eq!(w.requests().unwrap().escrow(), 0);
        assert_eq!(w.requests().unwrap().board(Site::Market).paid, 1);
        assert_eq!(w.requests().unwrap().receipts().len(), 1);
        let before = w.clone();
        assert_eq!(w.abandon_request(id), Err(Rejection::Refused));
        w.request_contribution(Site::Market);
        assert_eq!(w, before);
    }
    #[test]
    fn no_cargo_or_funds_no_progress_and_decline_has_no_charge() {
        let (mut w, id) = funded_fixture(Site::Market);
        w.region.as_mut().unwrap().market_coins = 1;
        let before = w.clone();
        assert_eq!(w.choose_request(id, true), Err(Rejection::NotEnoughCoins));
        assert_eq!(w, before);
        assert_eq!(
            w.choose_request(id, false),
            Ok(Outcome::RequestDeclined(id))
        );
        assert_eq!(w.requests().unwrap().escrow(), 0);
        assert_eq!(w.region().unwrap().market_coins, 1);
        let (mut w, id) = funded_fixture(Site::Market);
        w.choose_request(id, true).unwrap();
        w.player.herbs = 0;
        assert_eq!(w.trade_herb(false), Err(Rejection::NoCargo));
        assert_eq!(current(&w, Site::Market).delivered, 0);
        let total = treasury(&w);
        let position = w.scenario.entrance;
        w.player.position = position;
        assert!(!w.at_request_site(Site::Market));
        assert_eq!(w.abandon_request(id), Ok(Outcome::RequestAbandoned(id)));
        assert_eq!(w.requests().unwrap().escrow(), 0);
        assert_eq!(treasury(&w), total);
        assert_eq!(
            w.requests()
                .unwrap()
                .receipts()
                .back()
                .unwrap()
                .request
                .status,
            Status::Abandoned
        );
    }
    #[test]
    fn unseen_external_closure_is_not_an_oracle_and_withdrawal_is_personal() {
        let (mut w, id) = funded_fixture(Site::Market);
        w.choose_request(id, true).unwrap();
        w.player.position = w.scenario.entrance;
        assert!(!w.at_request_site(Site::Market));
        let mut remote = w.clone();
        let seen = w.known_requests();
        let total = treasury(&w);
        remote.region.as_mut().unwrap().market_herbs = MARKET_TARGET;
        remote.advance_requests();
        assert_eq!(current(&remote, Site::Market).status, Status::Superseded);
        assert_eq!(remote.known_requests(), seen);
        assert!(remote.requests().unwrap().receipts().is_empty());
        assert_eq!(treasury(&remote), total);
        assert_eq!(remote.choose_request(id, true), Err(Rejection::OutOfReach));
        assert_eq!(remote.abandon_request(id), w.abandon_request(id));
        assert_eq!(remote.known_requests(), w.known_requests());
        assert_eq!(
            remote.requests().unwrap().receipts(),
            w.requests().unwrap().receipts()
        );
        assert_eq!(treasury(&remote), treasury(&w));
    }
    #[test]
    fn partial_work_can_meet_last_remaining_need_and_external_completion_refunds() {
        let (mut w, id) = funded_fixture(Site::Market);
        w.choose_request(id, true).unwrap();
        w.region.as_mut().unwrap().market_herbs = MARKET_TARGET - 1;
        w.player.herbs = 1;
        w.trade_herb(false).unwrap();
        assert_eq!(current(&w, Site::Market).status, Status::Completed);
        assert_eq!(current(&w, Site::Market).delivered, 1);
        let (mut w, id) = funded_fixture(Site::Market);
        w.choose_request(id, true).unwrap();
        let total = treasury(&w);
        w.region.as_mut().unwrap().market_herbs = MARKET_TARGET;
        w.advance_requests();
        assert_eq!(current(&w, Site::Market).status, Status::Superseded);
        assert_eq!(w.requests().unwrap().escrow(), 0);
        assert_eq!(w.requests().unwrap().board(Site::Market).paid, 0);
        assert_eq!(treasury(&w), total);
        assert_eq!(
            w.requests()
                .unwrap()
                .receipts()
                .back()
                .unwrap()
                .request
                .status,
            Status::Superseded
        );
    }
    #[test]
    fn deadline_is_exclusive_and_final_horizon_refunds() {
        let (mut w, id) = funded_fixture(Site::Market);
        w.choose_request(id, true).unwrap();
        let deadline = current(&w, Site::Market).deadline;
        let total = treasury(&w);
        w.tick = deadline - 1;
        w.advance_requests();
        assert_eq!(current(&w, Site::Market).status, Status::Accepted);
        w.tick = deadline;
        w.advance_requests();
        assert_eq!(current(&w, Site::Market).status, Status::Expired);
        assert_eq!(treasury(&w), total);
        assert_eq!(w.requests().unwrap().escrow(), 0);
        let (mut w, _) = funded_fixture(Site::Market);
        w.requests = Some(Requests::default());
        w.tick = super::super::MAX_TICKS - 2;
        w.advance_requests();
        let r = current(&w, Site::Market);
        assert_eq!(r.deadline, super::super::MAX_TICKS);
        w.choose_request(r.id, true).unwrap();
        w.tick = super::super::MAX_TICKS;
        w.advance_requests();
        assert_eq!(current(&w, Site::Market).status, Status::Expired);
        assert_eq!(w.requests().unwrap().escrow(), 0);
    }
    #[test]
    fn receipt_cap_and_monotonic_ids_and_cooldown() {
        let (mut w, mut id) = funded_fixture(Site::Shrine);
        for i in 0..(RECEIPT_CAP + 7) {
            assert_eq!(
                w.choose_request(id, false),
                Ok(Outcome::RequestDeclined(id))
            );
            assert_eq!(
                w.requests().unwrap().receipts().len(),
                (i + 1).min(RECEIPT_CAP)
            );
            let last = id;
            let due = w.requests().unwrap().board(Site::Shrine).next_open;
            w.tick = due - 1;
            w.advance_requests();
            assert_eq!(current(&w, Site::Shrine).id, last);
            w.tick = due;
            w.advance_requests();
            id = current(&w, Site::Shrine).id;
            assert!(id > last);
        }
        assert!(
            w.requests()
                .unwrap()
                .receipts()
                .iter()
                .all(|r| r.request.status == Status::Declined)
        );
    }
    #[test]
    fn revision_admission_and_hash_binding() {
        let mut old = World::road_history_campaign(7);
        let bytes = old.checkpoint();
        for intent in [
            Intent::AcceptRequest(1),
            Intent::DeclineRequest(1),
            Intent::AbandonRequest(u32::MAX),
        ] {
            assert_eq!(old.step(0, intent), Err(StepError::UnrecordableIntent));
            assert_eq!(old.checkpoint(), bytes);
        }
        let w = World::emergent_requests_campaign(7);
        let mut bad = w.clone();
        bad.requests.as_mut().unwrap().next_id += 1;
        assert_ne!(w.state_digest(), bad.state_digest());
        assert_eq!(
            World::restore(&bad.checkpoint()),
            Err(crate::living::LoadError::StateMismatch)
        );
        let mut w = w;
        assert_eq!(
            w.step(0, Intent::AcceptRequest(u32::MAX)),
            Ok(Outcome::Rejected(Rejection::NoEvidence))
        );
        assert_eq!(&w.checkpoint()[54..59], &[44, 255, 255, 255, 255]);
        assert_eq!(World::restore(&w.checkpoint()).unwrap(), w);
    }
}
