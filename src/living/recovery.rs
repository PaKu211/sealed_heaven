//! Physical custody of abandoned freight. Only an observer who saw a corpse
//! can recover its cargo, and the carried herbs must reach the shrine.

use super::navigation::in_reach;
use super::{ActorId, KnowledgeSource, RecoveryEvent, World};

impl World {
    pub(super) fn recover_corpse_cargo(&mut self, watch: ActorId, body: ActorId) {
        if !self.is_physical_recovery()
            || self.companion(watch).is_some()
            || self.social.recovery_cargo.contains_key(&watch)
            || self
                .social
                .discoveries
                .get(&(watch, body))
                .is_none_or(|receipt| receipt.source != KnowledgeSource::Observed)
        {
            return;
        }
        let corpse = &self.npcs[&body];
        let site = corpse.position;
        if corpse.alive() || corpse.herbs == 0 {
            return;
        }
        let here = self.npcs[&watch].position;
        if !in_reach(here, site) {
            self.travel(watch, site, 1);
            return;
        }
        let herbs = corpse.herbs;
        let Some(total) = self.npcs[&watch].herbs.checked_add(herbs) else {
            return;
        };
        self.npcs.get_mut(&body).unwrap().herbs = 0;
        self.npcs.get_mut(&watch).unwrap().herbs = total;
        self.social.recovery_cargo.insert(watch, herbs);
        self.record_recovery(watch, body, herbs, false, site);
    }

    pub(super) fn deliver_recovered_cargo(&mut self, watch: ActorId) {
        if !self.is_physical_recovery() || self.companion(watch).is_some() {
            return;
        }
        let Some(&herbs) = self.social.recovery_cargo.get(&watch) else {
            return;
        };
        if self.npcs[&watch].herbs < herbs {
            return;
        }
        let shrine = self.scenario.shrine;
        if !in_reach(self.npcs[&watch].position, shrine) {
            self.travel(watch, shrine, 1);
            return;
        }
        let Some(total) = self.supply.shrine.checked_add(herbs) else {
            return;
        };
        let Some(deliveries) = self.supply.npc_deliveries.checked_add(herbs) else {
            return;
        };
        self.npcs.get_mut(&watch).unwrap().herbs -= herbs;
        self.social.recovery_cargo.remove(&watch);
        self.supply.shrine = total;
        self.supply.npc_deliveries = deliveries;
        // The recovery event retains the body identity, not a claim that the
        // watch knows who killed the carrier.
        let body = self
            .social
            .recoveries
            .iter()
            .rev()
            .find(|event| event.actor == watch && !event.delivered)
            .expect("recovered cargo has a pickup event")
            .body;
        self.record_recovery(watch, body, herbs, true, shrine);
    }

    fn record_recovery(
        &mut self,
        actor: ActorId,
        body: ActorId,
        herbs: u32,
        delivered: bool,
        site: super::Position,
    ) {
        let witnessed = self.player.alive()
            && self.perceives(self.player.position, site)
            && self.perceives(self.player.position, self.npcs[&actor].position);
        self.social.recoveries.push(RecoveryEvent {
            id: u32::try_from(self.social.recoveries.len()).expect("one pair of events per corpse"),
            at: self.tick,
            actor,
            body,
            herbs,
            delivered,
            witnessed,
        });
    }
}
