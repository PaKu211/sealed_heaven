//! Revision-twelve institutional accountability. A completed contract leaves
//! evidence at its delivery site; an unfulfilled contract becomes inspectable
//! only after its original deadline. Requesters must visit that site before
//! learning an outcome. This is *personal* institutional memory, never a
//! globally visible reputation score or a remote report of someone's death.

use std::collections::BTreeMap;

use super::contracts::DEADLINE;
use super::navigation::in_reach;
use super::{ActorId, Goal, Need, Position, World};

const RECEIPT_CAP: usize = 96;
const HISTORY_CAP: usize = 160;
const MAX_REGARD: i8 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContractReceipt {
    pub case_id: u32,
    pub need: Need,
    pub requester: ActorId,
    pub provider: ActorId,
    pub opened: u64,
    pub closed: u64,
    pub delivered: bool,
    pub inspected: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditFinding {
    Delivered,
    Unfulfilled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuditEvent {
    pub at: u64,
    pub case_id: u32,
    pub need: Need,
    pub requester: ActorId,
    pub provider: ActorId,
    pub site: Position,
    pub finding: AuditFinding,
    pub witnessed: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ContractMemory {
    pub receipts: Vec<ContractReceipt>,
    /// Only the requester who inspected a delivery site learns this score.
    pub regard: BTreeMap<(ActorId, ActorId), i8>,
    pub history: Vec<AuditEvent>,
}

impl World {
    pub fn contract_memory(&self) -> Option<&ContractMemory> {
        self.contract_memory.as_ref()
    }

    pub(super) fn contract_regard(&self, requester: ActorId, provider: ActorId) -> i8 {
        self.contract_memory()
            .and_then(|memory| memory.regard.get(&(requester, provider)).copied())
            .unwrap_or(0)
    }

    /// A receipt is not knowledge. For failed work it represents only the
    /// original obligation and the absence of a signed delivery, not knowledge
    /// of whether its courier died, deserted or was attacked elsewhere.
    pub(super) fn retain_contract_receipt(&mut self, receipt: ContractReceipt) {
        let Some(memory) = self.contract_memory.as_mut() else {
            return;
        };
        if memory.receipts.len() == RECEIPT_CAP {
            let oldest_inspected = memory
                .receipts
                .iter()
                .position(|entry| entry.inspected)
                .unwrap_or(0);
            memory.receipts.remove(oldest_inspected);
        }
        memory.receipts.push(receipt);
    }

    pub(super) fn audit_goal(&self, requester: ActorId) -> Option<Goal> {
        if !self.npcs.get(&requester)?.alive() || self.companion(requester).is_some() {
            return None;
        }
        self.contract_memory()?
            .receipts
            .iter()
            .filter(|receipt| receipt.requester == requester && !receipt.inspected)
            .filter(|receipt| {
                receipt.delivered || self.tick >= receipt.opened.saturating_add(DEADLINE)
            })
            .min_by_key(|receipt| (receipt.closed, receipt.case_id))
            .map(|receipt| Goal::AuditContract(receipt.need))
    }

    pub(super) fn inspect_contract(&mut self, requester: ActorId, need: Need) {
        if self.npcs.get(&requester).is_none_or(|npc| !npc.alive())
            || self.companion(requester).is_some()
        {
            return;
        }
        let Some(receipt) = self.contract_memory().and_then(|memory| {
            memory.receipts.iter().copied().find(|receipt| {
                receipt.requester == requester
                    && receipt.need == need
                    && !receipt.inspected
                    && (receipt.delivered || self.tick >= receipt.opened.saturating_add(DEADLINE))
            })
        }) else {
            return;
        };
        let site = receipt.need.site(self);
        let here = self.npcs[&requester].position;
        if !in_reach(here, site) || !self.perceives(here, site) {
            // A known site is a route, not a magical view of its current state.
            self.travel(requester, site, 1);
            return;
        }
        let finding = if receipt.delivered {
            AuditFinding::Delivered
        } else {
            AuditFinding::Unfulfilled
        };
        let witnessed = self.player_sees(here) && self.player_sees(site);
        let memory = self.contract_memory.as_mut().expect("revision twelve");
        // A receipt is removed from the queue exactly once; the rating is
        // changed only at the location and only by the actual requester.
        if let Some(entry) = memory
            .receipts
            .iter_mut()
            .find(|entry| entry.case_id == receipt.case_id && entry.need == receipt.need)
        {
            entry.inspected = true;
        }
        let regard = memory
            .regard
            .entry((requester, receipt.provider))
            .or_insert(0);
        let delta = if receipt.delivered { 2 } else { -3 };
        *regard = regard.saturating_add(delta).clamp(-MAX_REGARD, MAX_REGARD);
        if memory.history.len() == HISTORY_CAP {
            memory.history.remove(0);
        }
        memory.history.push(AuditEvent {
            at: self.tick,
            case_id: receipt.case_id,
            need: receipt.need,
            requester,
            provider: receipt.provider,
            site,
            finding,
            witnessed,
        });
        self.record_audit_information(
            receipt.case_id,
            receipt.need,
            requester,
            receipt.provider,
            finding,
            site,
            witnessed,
        );
    }
}
