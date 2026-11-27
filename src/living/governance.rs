//! C5.9: durable, locally written institutional boards.
use super::{
    ActorId, Need, Npc, World,
    contract_memory::{AuditFinding, ContractReceipt},
    navigation::in_reach,
};
use std::collections::VecDeque;
pub const REVISION: u16 = 46;
pub const VERSION: u16 = 1;
pub const REVIEW_INTERVAL: u64 = 16;
pub const REVIEW_GAP: u64 = 96;
pub const HISTORY_CAP: usize = 48;
pub const MIN_SHRINE_ORDER: u64 = 33;
pub const MIN_HALL_ORDER: u64 = 64;
pub const AUDIT_CAUTION: u64 = 48;
/// A personally experienced unsuccessful solicitation defers optional repeats.
pub const NO_AWARD_CAUTION: u64 = 192;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
#[repr(u8)]
pub enum Priority {
    #[default]
    Routine,
    Reserve,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Review {
    pub actor: ActorId,
    pub at: u64,
    pub stock: u32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Decision {
    pub priority: Priority,
    pub prior: Review,
    pub current: Review,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Order {
    pub actor: ActorId,
    pub at: u64,
    pub contract: u32,
    pub proactive: bool,
    pub priority: Priority,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Audit {
    pub actor: ActorId,
    pub requester: ActorId,
    pub provider: ActorId,
    pub at: u64,
    pub contract: u32,
    pub finding: AuditFinding,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum NegotiationKind {
    NoAward,
    RestoredLocally,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Negotiation {
    pub actor: ActorId,
    pub at: u64,
    pub contract: u32,
    pub kind: NegotiationKind,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FiledNegotiation {
    pub fact: Negotiation,
    pub filed_at: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Entry {
    Policy(Decision),
    Order(Order),
    Audit(Audit),
    Contribution { at: u64 },
    Negotiation(FiledNegotiation),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct Board {
    pub priority: Priority,
    pub review: Option<Review>,
    pub decision: Option<Decision>,
    pub changes: u32,
    pub orders: u32,
    pub proactive: u32,
    pub delivered: u32,
    pub unfulfilled: u32,
    pub player_contributions: u32,
    pub last_contribution: Option<u64>,
    pub last_order: Option<Order>,
    pub last_audit: Option<Audit>,
    pub latest_outcome: Option<Audit>,
    pub no_award: u32,
    pub restored_locally: u32,
    pub no_award_until: u64,
    pub last_negotiation: Option<FiledNegotiation>,
}
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct Ledger {
    pub board: Board,
    pub streak: i8,
    history: VecDeque<Entry>,
    pending: VecDeque<Negotiation>,
}
impl Ledger {
    pub fn history(&self) -> &VecDeque<Entry> {
        &self.history
    }
    fn append(&mut self, e: Entry) {
        if self.history.len() == HISTORY_CAP {
            self.history.pop_front();
        }
        self.history.push_back(e);
    }
    fn review(&mut self, r: Review, low: bool, adequate: bool) {
        let prior = self.board.review;
        if prior.is_some_and(|p| r.at.saturating_sub(p.at) < REVIEW_INTERVAL) {
            return;
        }
        if prior.is_none_or(|p| r.at.saturating_sub(p.at) > REVIEW_GAP) {
            self.streak = 0;
        }
        self.streak = if low {
            (self.streak.max(0) + 1).min(2)
        } else if adequate {
            (self.streak.min(0) - 1).max(-2)
        } else {
            0
        };
        self.board.review = Some(r);
        let priority = match self.streak {
            2 => Priority::Reserve,
            -2 => Priority::Routine,
            _ => self.board.priority,
        };
        if priority != self.board.priority {
            let decision = Decision {
                priority,
                prior: prior.expect("two local reviews"),
                current: r,
            };
            self.board.priority = priority;
            self.board.decision = Some(decision);
            self.board.changes += 1;
            self.append(Entry::Policy(decision));
        }
    }
    fn order(&mut self, actor: ActorId, at: u64, contract: u32, proactive: bool) {
        let order = Order {
            actor,
            at,
            contract,
            proactive,
            priority: self.board.priority,
        };
        self.board.orders += 1;
        if proactive {
            self.board.proactive += 1;
        }
        self.board.last_order = Some(order);
        self.append(Entry::Order(order));
    }
    fn audit(&mut self, a: Audit) {
        match a.finding {
            AuditFinding::Delivered => self.board.delivered += 1,
            AuditFinding::Unfulfilled => self.board.unfulfilled += 1,
        }
        self.board.last_audit = Some(a);
        if self
            .board
            .latest_outcome
            .is_none_or(|old| a.contract > old.contract)
        {
            self.board.latest_outcome = Some(a);
        }
        self.append(Entry::Audit(a));
    }
    pub fn pending(&self) -> &VecDeque<Negotiation> {
        &self.pending
    }
    fn queue(&mut self, n: Negotiation) {
        if self.pending.len() == HISTORY_CAP {
            self.pending.pop_front();
        }
        self.pending.push_back(n);
    }
    fn file_pending(&mut self, a: ActorId, now: u64) {
        let mut remain = VecDeque::new();
        while let Some(fact) = self.pending.pop_front() {
            if fact.actor != a {
                remain.push_back(fact);
                continue;
            }
            match fact.kind {
                NegotiationKind::NoAward => {
                    self.board.no_award += 1;
                    self.board.no_award_until =
                        self.board.no_award_until.max(fact.at + NO_AWARD_CAUTION);
                }
                NegotiationKind::RestoredLocally => self.board.restored_locally += 1,
            }
            let e = FiledNegotiation {
                fact,
                filed_at: now,
            };
            self.board.last_negotiation = Some(e);
            self.append(Entry::Negotiation(e));
        }
        self.pending = remain;
    }
    fn reserve_retry_ready(&self, now: u64) -> bool {
        now >= self.board.no_award_until
            && self.board.latest_outcome.is_none_or(|a| {
                a.finding == AuditFinding::Delivered || now.saturating_sub(a.at) >= AUDIT_CAUTION
            })
    }
    fn contribute(&mut self, at: u64) {
        self.board.player_contributions += 1;
        self.board.last_contribution = Some(at);
        self.append(Entry::Contribution { at });
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KnownBoard {
    pub read_at: u64,
    pub board: Board,
}
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct Governance {
    ledgers: [Ledger; 2],
    known: [Option<KnownBoard>; 2],
}
fn index(n: Need) -> usize {
    match n {
        Need::ShrineMedicine => 0,
        Need::HallOfferings => 1,
    }
}
impl Governance {
    pub fn ledger(&self, n: Need) -> &Ledger {
        &self.ledgers[index(n)]
    }
    pub fn known(&self, n: Need) -> Option<KnownBoard> {
        self.known[index(n)]
    }
    fn ledger_mut(&mut self, n: Need) -> &mut Ledger {
        &mut self.ledgers[index(n)]
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        let mut w = Wire(b"SH:C59:GOVERNANCE:v1\0".to_vec());
        w.u16(VERSION);
        for l in &self.ledgers {
            w.board(l.board);
            w.u8(l.streak as u8);
            w.u32(l.pending.len() as u32);
            for n in &l.pending {
                w.negotiation(*n);
            }
            w.u32(l.history.len() as u32);
            for e in &l.history {
                match *e {
                    Entry::Policy(d) => {
                        w.u8(0);
                        w.decision(d);
                    }
                    Entry::Order(o) => {
                        w.u8(1);
                        w.order(o);
                    }
                    Entry::Audit(a) => {
                        w.u8(2);
                        w.audit(a);
                    }
                    Entry::Contribution { at } => {
                        w.u8(3);
                        w.u64(at);
                    }
                    Entry::Negotiation(e) => {
                        w.u8(4);
                        w.filed(e);
                    }
                }
            }
        }
        for k in self.known {
            w.opt(k, |w, k| {
                w.u64(k.read_at);
                w.board(k.board);
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
    fn opt<T>(&mut self, n: Option<T>, f: impl FnOnce(&mut Self, T)) {
        self.u8(u8::from(n.is_some()));
        if let Some(n) = n {
            f(self, n);
        }
    }
    fn review(&mut self, r: Review) {
        self.actor(r.actor);
        self.u64(r.at);
        self.u32(r.stock);
    }
    fn decision(&mut self, d: Decision) {
        self.u8(d.priority as u8);
        self.review(d.prior);
        self.review(d.current);
    }
    fn order(&mut self, o: Order) {
        self.actor(o.actor);
        self.u64(o.at);
        self.u32(o.contract);
        self.u8(u8::from(o.proactive));
        self.u8(o.priority as u8);
    }
    fn audit(&mut self, a: Audit) {
        self.actor(a.actor);
        self.actor(a.requester);
        self.actor(a.provider);
        self.u64(a.at);
        self.u32(a.contract);
        self.u8(match a.finding {
            AuditFinding::Delivered => 0,
            AuditFinding::Unfulfilled => 1,
        });
    }
    fn negotiation(&mut self, n: Negotiation) {
        self.actor(n.actor);
        self.u64(n.at);
        self.u32(n.contract);
        self.u8(n.kind as u8);
    }
    fn filed(&mut self, n: FiledNegotiation) {
        self.negotiation(n.fact);
        self.u64(n.filed_at);
    }
    fn board(&mut self, b: Board) {
        self.u8(b.priority as u8);
        self.opt(b.review, Wire::review);
        self.opt(b.decision, Wire::decision);
        for x in [
            b.changes,
            b.orders,
            b.proactive,
            b.delivered,
            b.unfulfilled,
            b.player_contributions,
        ] {
            self.u32(x);
        }
        self.opt(b.last_contribution, Wire::u64);
        self.opt(b.last_order, Wire::order);
        self.opt(b.last_audit, Wire::audit);
        self.opt(b.latest_outcome, Wire::audit);
        self.u32(b.no_award);
        self.u32(b.restored_locally);
        self.u64(b.no_award_until);
        self.opt(b.last_negotiation, Wire::filed);
    }
}
impl World {
    pub fn governance(&self) -> Option<&Governance> {
        self.governance.as_ref()
    }
    pub fn known_board(&self, n: Need) -> Option<KnownBoard> {
        self.governance()?.known(n)
    }
    pub(super) fn contract_requester(&self, n: Need) -> Option<ActorId> {
        if self.governance.is_some() && n == Need::HallOfferings {
            return self.school()?.leader;
        }
        Some(n.requester())
    }
    pub(super) fn is_institutional_requester(&self, a: ActorId) -> bool {
        Need::ALL
            .iter()
            .any(|&n| self.contract_requester(n) == Some(a))
    }
    pub(super) fn institutional_testimony_target(&self, sender: &Npc, peer: &Npc) -> bool {
        Need::ALL.iter().any(|need| need.requester() == peer.id())
            || (self.governance.is_some()
                && in_reach(sender.position, peer.position)
                && self.can_see(sender.position, peer.position)
                && self.is_institutional_requester(peer.id()))
    }
    pub(super) fn can_inspect_contract_receipt(
        &self,
        inspector: ActorId,
        receipt: &ContractReceipt,
    ) -> bool {
        if inspector == receipt.requester {
            return true;
        }
        self.governance.is_some()
            && receipt.need == Need::HallOfferings
            && self.governance_review_allowed(receipt.need, inspector)
            && self.school().is_some_and(|school| {
                school
                    .history
                    .iter()
                    .any(|e| e.former == receipt.requester && e.successor.is_some())
            })
    }
    fn governance_stock(&self, n: Need) -> u32 {
        match n {
            Need::ShrineMedicine => self.supply.shrine,
            Need::HallOfferings => self.region().expect("governance region").hall_offerings,
        }
    }
    fn governance_present(&self, n: Need, a: ActorId) -> bool {
        self.npcs.get(&a).is_some_and(|npc| {
            npc.alive()
                && in_reach(npc.position, n.site(self))
                && self.can_see(npc.position, n.site(self))
        })
    }
    fn governance_review_allowed(&self, n: Need, a: ActorId) -> bool {
        self.governance_present(n, a)
            && self.contract_requester(n) == Some(a)
            && self.circle_available(a)
            && (n != Need::HallOfferings || !self.is_night())
    }
    pub(super) fn advance_governance(&mut self) {
        if self.governance.is_none() {
            return;
        }
        for n in Need::ALL {
            let Some(a) = self.contract_requester(n) else {
                continue;
            };
            if !self.governance_review_allowed(n, a) {
                continue;
            }
            let stock = self.governance_stock(n);
            let (low, adequate) = match n {
                Need::ShrineMedicine => (
                    stock <= 2 && stock < self.scenario.shrine_target,
                    stock >= self.scenario.shrine_target,
                ),
                Need::HallOfferings => (stock == 0, stock >= 2),
            };
            let r = Review {
                actor: a,
                at: self.tick,
                stock,
            };
            let ledger = self.governance.as_mut().unwrap().ledger_mut(n);
            ledger.file_pending(a, self.tick);
            ledger.review(r, low, adequate);
        }
    }
    pub(super) fn governance_need(&self, n: Need, a: ActorId) -> bool {
        let Some(g) = self.governance() else {
            return false;
        };
        if g.ledger(n).board.priority != Priority::Reserve
            || !self.governance_review_allowed(n, a)
            || !g.ledger(n).reserve_retry_ready(self.tick)
        {
            return false;
        }
        let stock = self.governance_stock(n);
        match n {
            Need::ShrineMedicine => {
                self.tick >= MIN_SHRINE_ORDER && stock < self.scenario.shrine_target
            }
            Need::HallOfferings => self.tick >= MIN_HALL_ORDER && stock == 0,
        }
    }
    /// An official carries their own witnessed decision to the office.
    /// No mechanical remote expiry, death or provider outcome enters this queue.
    pub(super) fn note_governance_negotiation(
        &mut self,
        n: Need,
        a: ActorId,
        contract: u32,
        kind: NegotiationKind,
    ) {
        if let Some(g) = self.governance.as_mut() {
            g.ledger_mut(n).queue(Negotiation {
                actor: a,
                at: self.tick,
                contract,
                kind,
            });
        }
    }
    pub(super) fn note_governance_order(
        &mut self,
        n: Need,
        a: ActorId,
        contract: u32,
        proactive: bool,
    ) {
        if let Some(g) = self.governance.as_mut() {
            g.ledger_mut(n).order(a, self.tick, contract, proactive);
        }
    }
    pub(super) fn note_governance_audit(
        &mut self,
        inspector: ActorId,
        r: ContractReceipt,
        finding: AuditFinding,
    ) {
        if let Some(g) = self.governance.as_mut() {
            g.ledger_mut(r.need).audit(Audit {
                actor: inspector,
                requester: r.requester,
                provider: r.provider,
                at: self.tick,
                contract: r.case_id,
                finding,
            });
        }
    }
    pub(super) fn note_governance_contribution(&mut self, n: Need) {
        if let Some(g) = self.governance.as_mut() {
            g.ledger_mut(n).contribute(self.tick);
        }
    }
    pub(super) fn observe_governance_boards(&mut self) {
        if self.governance.is_none() || !self.player.alive() {
            return;
        }
        for n in Need::ALL {
            if in_reach(self.player.position, n.site(self))
                && self.can_see(self.player.position, n.site(self))
            {
                let g = self.governance.as_mut().unwrap();
                g.known[index(n)] = Some(KnownBoard {
                    read_at: self.tick,
                    board: g.ledger(n).board,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
