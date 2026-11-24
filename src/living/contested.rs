//! Scarce, individually identified goods. A lot has exactly one physical
//! holder; agents can act only on a site they have actually observed.

use std::collections::BTreeMap;

use super::navigation::in_reach;
use super::{ActorId, Goal, Outcome, PLAYER, Position, Region, Rejection, Role, World};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Good {
    SpiritOre,
    JadeScript,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Holder {
    Ground(Position),
    Carried(ActorId),
    Deposited(ActorId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Lot {
    pub id: u32,
    pub good: Good,
    pub holder: Holder,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LotEventKind {
    Claimed,
    Sold,
    Deposited,
    Dropped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LotEvent {
    pub id: u32,
    pub at: u64,
    pub lot: u32,
    pub actor: ActorId,
    pub kind: LotEventKind,
    pub position: Position,
    pub witnessed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contested {
    pub lots: BTreeMap<u32, Lot>,
    /// Observed last location, not a live global tracker.
    pub known: BTreeMap<(ActorId, u32), Position>,
    pub history: Vec<LotEvent>,
    pub prestige: BTreeMap<ActorId, u32>,
}

impl Contested {
    pub(super) fn new() -> Self {
        let lots = [
            (0, Good::SpiritOre, Position { x: 8, y: 4 }),
            (1, Good::SpiritOre, Position { x: 26, y: 4 }),
            (2, Good::JadeScript, Position { x: 29, y: 6 }),
        ]
        .into_iter()
        .map(|(id, good, position)| {
            (
                id,
                Lot {
                    id,
                    good,
                    holder: Holder::Ground(position),
                },
            )
        })
        .collect();
        Self {
            lots,
            known: BTreeMap::new(),
            history: Vec::new(),
            prestige: BTreeMap::new(),
        }
    }
}

impl World {
    pub fn contested(&self) -> Option<&Contested> {
        self.contested.as_ref()
    }

    pub(super) fn observe_lots(&mut self) {
        let Some(contested) = self.contested() else {
            return;
        };
        let sites: Vec<_> = contested
            .lots
            .values()
            .filter_map(|lot| match lot.holder {
                Holder::Ground(pos) => Some((lot.id, pos)),
                Holder::Carried(actor)
                    if actor != PLAYER && self.npcs.get(&actor).is_some_and(|npc| !npc.alive()) =>
                {
                    Some((lot.id, self.npcs[&actor].position))
                }
                _ => None,
            })
            .collect();
        let actors: Vec<_> = self
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
        for (actor, from) in actors {
            for &(id, site) in &sites {
                if self.perceives(from, site) {
                    self.contested
                        .as_mut()
                        .unwrap()
                        .known
                        .insert((actor, id), site);
                }
            }
        }
    }

    pub(super) fn drop_player_lots(&mut self) {
        let site = self.player.position;
        let ids: Vec<_> = self
            .contested()
            .into_iter()
            .flat_map(|state| state.lots.values())
            .filter(|lot| lot.holder == Holder::Carried(PLAYER))
            .map(|lot| lot.id)
            .collect();
        for id in ids {
            self.contested
                .as_mut()
                .unwrap()
                .lots
                .get_mut(&id)
                .unwrap()
                .holder = Holder::Ground(site);
            self.record_lot(id, PLAYER, LotEventKind::Dropped, site);
        }
    }

    pub(super) fn claim_lot(&mut self, id: u32) -> Result<Outcome, Rejection> {
        let lot = self
            .contested()
            .and_then(|state| state.lots.get(&id).copied())
            .ok_or(Rejection::Empty)?;
        let site = match lot.holder {
            Holder::Ground(site) => site,
            Holder::Carried(actor)
                if actor != PLAYER && self.npcs.get(&actor).is_some_and(|npc| !npc.alive()) =>
            {
                self.npcs[&actor].position
            }
            _ => return Err(Rejection::Refused),
        };
        if !self.player_sees(site) {
            return Err(Rejection::NoEvidence);
        }
        self.require_reach(site)?;
        self.contested
            .as_mut()
            .unwrap()
            .lots
            .get_mut(&id)
            .unwrap()
            .holder = Holder::Carried(PLAYER);
        self.record_lot(id, PLAYER, LotEventKind::Claimed, site);
        Ok(Outcome::LotClaimed(id))
    }

    pub(super) fn offer_lot(&mut self, to: ActorId, id: u32) -> Result<Outcome, Rejection> {
        let recipient = self
            .npcs
            .get(&to)
            .filter(|npc| npc.alive())
            .ok_or(Rejection::UnknownActor)?;
        let role = recipient.definition.role;
        let site = recipient.position;
        self.require_reach(site)?;
        if self
            .contested()
            .and_then(|state| state.lots.get(&id))
            .is_none_or(|lot| lot.holder != Holder::Carried(PLAYER))
        {
            return Err(Rejection::NoCargo);
        }
        if self.social.relationship(to, PLAYER).refuses_aid() {
            return Err(Rejection::Refused);
        }
        let price = self.contested().unwrap().lots[&id].good.value();
        let Some(coins) = self.region.as_ref().unwrap().coins.checked_add(price) else {
            return Err(Rejection::PackFull);
        };
        match role {
            Role::RivalEnvoy
                if self
                    .rivalry
                    .as_ref()
                    .is_some_and(|state| state.lodge_coins >= price) =>
            {
                self.rivalry.as_mut().unwrap().lodge_coins -= price
            }
            Role::HallSteward
                if self
                    .politics
                    .as_ref()
                    .is_some_and(|state| state.hall_coins >= price) =>
            {
                self.politics.as_mut().unwrap().hall_coins -= price
            }
            _ => return Err(Rejection::Refused),
        }
        self.region.as_mut().unwrap().coins = coins;
        self.contested
            .as_mut()
            .unwrap()
            .lots
            .get_mut(&id)
            .unwrap()
            .holder = Holder::Carried(to);
        self.social.aided(PLAYER, to);
        self.record_lot(id, PLAYER, LotEventKind::Sold, site);
        Ok(Outcome::LotSold {
            to,
            id,
            coins: price,
        })
    }

    pub(super) fn lot_goal(&self, actor: ActorId) -> Option<Goal> {
        let npc = self.npcs.get(&actor)?;
        if !npc.alive()
            || self.companion(actor).is_some()
            || !matches!(npc.definition.role, Role::HallSteward | Role::RivalEnvoy)
        {
            return None;
        }
        let state = self.contested()?;
        if let Some(lot) = state
            .lots
            .values()
            .find(|lot| lot.holder == Holder::Carried(actor))
        {
            return Some(Goal::DeliverLot(lot.id));
        }
        state
            .known
            .iter()
            .filter(|((observer, _), _)| *observer == actor)
            .filter_map(|((_, id), site)| {
                let lot = state.lots.get(id)?;
                // Do not inspect its current holder from afar. The last
                // observed position is merely a lead until the NPC arrives.
                Some((lot.id, *site))
            })
            .min_by_key(|(id, site)| {
                (
                    npc.position.x.abs_diff(site.x) + npc.position.y.abs_diff(site.y),
                    *id,
                )
            })
            .map(|(id, _)| Goal::ClaimLot(id))
    }

    pub(super) fn work_lot(&mut self, actor: ActorId, goal: Goal) {
        match goal {
            Goal::ClaimLot(id) => {
                let Some(lot) = self
                    .contested()
                    .and_then(|state| state.lots.get(&id).copied())
                else {
                    return;
                };
                let Some(&site) = self.contested().unwrap().known.get(&(actor, id)) else {
                    return;
                };
                if !self.perceives(self.npcs[&actor].position, site) {
                    self.travel(actor, site, 1);
                    return;
                }
                let available = match lot.holder {
                    Holder::Ground(here) => here == site,
                    Holder::Carried(holder) if holder != PLAYER => self
                        .npcs
                        .get(&holder)
                        .is_some_and(|npc| !npc.alive() && npc.position == site),
                    _ => false,
                };
                if !available {
                    self.contested.as_mut().unwrap().known.remove(&(actor, id));
                    return;
                }
                if !in_reach(self.npcs[&actor].position, site) {
                    self.travel(actor, site, 1);
                    return;
                }
                self.contested
                    .as_mut()
                    .unwrap()
                    .lots
                    .get_mut(&id)
                    .unwrap()
                    .holder = Holder::Carried(actor);
                self.record_lot(id, actor, LotEventKind::Claimed, site);
            }
            Goal::DeliverLot(id) => {
                let Some(lot) = self
                    .contested()
                    .and_then(|state| state.lots.get(&id).copied())
                else {
                    return;
                };
                if lot.holder != Holder::Carried(actor) {
                    return;
                }
                let site = match self.npcs[&actor].definition.role {
                    Role::HallSteward => Region::sites().hall,
                    Role::RivalEnvoy => self.npcs[&actor].definition.home,
                    _ => return,
                };
                if !in_reach(self.npcs[&actor].position, site) {
                    self.travel(actor, site, 1);
                    return;
                }
                self.contested
                    .as_mut()
                    .unwrap()
                    .lots
                    .get_mut(&id)
                    .unwrap()
                    .holder = Holder::Deposited(actor);
                let strength = lot.good.value();
                *self
                    .contested
                    .as_mut()
                    .unwrap()
                    .prestige
                    .entry(actor)
                    .or_default() += strength;
                if let Some(rivalry) = &mut self.rivalry {
                    rivalry.pressure = if self.npcs[&actor].definition.role == Role::RivalEnvoy {
                        rivalry.pressure.saturating_add(strength as u8).min(8)
                    } else {
                        rivalry.pressure.saturating_sub(strength as u8)
                    };
                }
                self.record_lot(id, actor, LotEventKind::Deposited, site);
            }
            _ => {}
        }
    }

    fn record_lot(&mut self, id: u32, actor: ActorId, kind: LotEventKind, site: Position) {
        let witnessed = self.player.alive() && self.perceives(self.player.position, site);
        let state = self.contested.as_mut().unwrap();
        state.history.push(LotEvent {
            id: u32::try_from(state.history.len()).expect("bounded physical lots"),
            at: self.tick,
            lot: id,
            actor,
            kind,
            position: site,
            witnessed,
        });
    }
}

impl Good {
    const fn value(self) -> u32 {
        match self {
            Self::SpiritOre => 2,
            Self::JadeScript => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Direction, Intent};

    #[test]
    fn public_race_for_relics_allows_two_patron_choices_or_no_intervention() {
        let mut world = World::contested_resources(0);
        let mut idle = world.clone();
        for _ in 0..8 {
            let result = world
                .step(world.tick(), Intent::Move(Direction::East))
                .unwrap();
            assert_eq!(result, Outcome::Moved);
            idle.step(idle.tick(), Intent::Wait).unwrap();
        }
        assert_eq!(
            world.step(world.tick(), Intent::ClaimLot(0)).unwrap(),
            Outcome::LotClaimed(0)
        );
        let mut replay = World::restore(&world.checkpoint()).unwrap();
        for _ in 0..20 {
            world
                .step(world.tick(), Intent::Move(Direction::East))
                .unwrap();
            replay
                .step(replay.tick(), Intent::Move(Direction::East))
                .unwrap();
            idle.step(idle.tick(), Intent::Wait).unwrap();
            assert_eq!(world.state_digest(), replay.state_digest());
            assert_eq!(world.contested().unwrap().lots.len(), 3);
            assert_eq!(idle.contested().unwrap().lots.len(), 3);
        }
        assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
        let mut lodge = world.clone();
        world
            .step(world.tick(), Intent::Move(Direction::South))
            .unwrap();
        assert_eq!(
            world
                .step(
                    world.tick(),
                    Intent::OfferLot {
                        to: ActorId::new(36),
                        id: 0
                    }
                )
                .unwrap(),
            Outcome::LotSold {
                to: ActorId::new(36),
                id: 0,
                coins: 2
            }
        );
        for action in [
            Intent::Move(Direction::South),
            Intent::Move(Direction::West),
            Intent::Move(Direction::West),
        ] {
            lodge.step(lodge.tick(), action).unwrap();
        }
        assert_eq!(
            lodge
                .step(
                    lodge.tick(),
                    Intent::OfferLot {
                        to: ActorId::new(35),
                        id: 0
                    }
                )
                .unwrap(),
            Outcome::LotSold {
                to: ActorId::new(35),
                id: 0,
                coins: 2
            }
        );
        for _ in 0..35 {
            world.step(world.tick(), Intent::Wait).unwrap();
            lodge.step(lodge.tick(), Intent::Wait).unwrap();
            idle.step(idle.tick(), Intent::Wait).unwrap();
        }
        for (branch, owner) in [(&world, ActorId::new(36)), (&lodge, ActorId::new(35))] {
            let contested = branch.contested().unwrap();
            assert_eq!(contested.lots[&0].holder, Holder::Deposited(owner));
            assert_eq!(
                contested
                    .history
                    .iter()
                    .filter(|e| e.lot == 0 && e.kind == LotEventKind::Deposited)
                    .count(),
                1
            );
            assert_eq!(World::restore(&branch.checkpoint()).unwrap(), *branch);
            assert!(
                contested
                    .history
                    .iter()
                    .enumerate()
                    .all(|(n, event)| event.id == n as u32)
            );
        }
        assert_ne!(
            world.contested().unwrap().prestige,
            lodge.contested().unwrap().prestige
        );
        assert_eq!(
            idle.contested().unwrap().lots[&0].holder,
            Holder::Ground(Position { x: 8, y: 4 })
        );
        assert_eq!(
            idle.contested().unwrap().lots[&1].holder,
            Holder::Deposited(ActorId::new(35))
        );
        assert_eq!(
            idle.contested().unwrap().lots[&2].holder,
            Holder::Deposited(ActorId::new(36))
        );
        assert_eq!(World::restore(&idle.checkpoint()).unwrap(), idle);
    }

    #[test]
    fn two_observers_compete_for_one_lot_without_duplicate_custody() {
        let mut world = World::contested_resources(0);
        let hall = ActorId::new(36);
        let lodge = ActorId::new(35);
        let site = Position { x: 26, y: 4 };
        // Isolated contested-site fixture: the public route above is the
        // acceptance test. Neither actor receives omniscient lot knowledge.
        world.npcs.get_mut(&hall).unwrap().position = Position { x: 27, y: 4 };
        world.npcs.get_mut(&lodge).unwrap().position = Position { x: 25, y: 4 };
        world.observe_lots();
        assert_eq!(
            world.contested().unwrap().known.get(&(hall, 1)),
            Some(&site)
        );
        assert_eq!(
            world.contested().unwrap().known.get(&(lodge, 1)),
            Some(&site)
        );
        world.work_lot(lodge, Goal::ClaimLot(1));
        assert_eq!(
            world.contested().unwrap().lots[&1].holder,
            Holder::Carried(lodge)
        );
        world.work_lot(hall, Goal::ClaimLot(1));
        assert_eq!(
            world.contested().unwrap().lots[&1].holder,
            Holder::Carried(lodge)
        );
        assert!(!world.contested().unwrap().known.contains_key(&(hall, 1)));
        assert_eq!(
            world
                .contested()
                .unwrap()
                .history
                .iter()
                .filter(|event| event.lot == 1)
                .count(),
            1
        );
    }

    #[test]
    fn sale_transfers_exact_coins_once_and_cannot_sell_the_same_lot_twice() {
        let mut world = World::contested_resources(0);
        let hall = ActorId::new(36);
        world.player.position = Position { x: 8, y: 4 };
        assert_eq!(world.claim_lot(0), Ok(Outcome::LotClaimed(0)));
        world.player.position = Position { x: 29, y: 4 };
        let hall_coins = world.politics().unwrap().hall_coins;
        let purse = world.region().unwrap().coins;
        assert_eq!(
            world.offer_lot(hall, 0),
            Ok(Outcome::LotSold {
                to: hall,
                id: 0,
                coins: 2
            })
        );
        assert_eq!(world.politics().unwrap().hall_coins, hall_coins - 2);
        assert_eq!(world.region().unwrap().coins, purse + 2);
        assert_eq!(world.offer_lot(hall, 0), Err(Rejection::NoCargo));
        assert_eq!(world.politics().unwrap().hall_coins, hall_coins - 2);
        assert_eq!(world.region().unwrap().coins, purse + 2);
        assert_eq!(world.contested().unwrap().lots.len(), 3);
    }

    #[test]
    fn death_drops_player_goods_and_dead_carrier_goods_remain_recoverable() {
        let mut world = World::contested_resources(0);
        let lodge = ActorId::new(35);
        let site = Position { x: 8, y: 4 };
        world.player.position = site;
        world.npcs.get_mut(&lodge).unwrap().position = site;
        assert_eq!(world.claim_lot(0), Ok(Outcome::LotClaimed(0)));
        world.damage(lodge, PLAYER, world.player.hp, false);
        assert_eq!(
            world.contested().unwrap().lots[&0].holder,
            Holder::Ground(site)
        );
        assert_eq!(
            world.contested().unwrap().history.last().unwrap().kind,
            LotEventKind::Dropped
        );
        world.npcs.get_mut(&lodge).unwrap().position = Position { x: 26, y: 6 };
        world.observe_lots();
        assert!(!world.contested().unwrap().known.contains_key(&(lodge, 0)));

        let mut carried = World::contested_resources(0);
        carried
            .contested
            .as_mut()
            .unwrap()
            .lots
            .get_mut(&0)
            .unwrap()
            .holder = Holder::Carried(lodge);
        carried.npcs.get_mut(&lodge).unwrap().position = site;
        carried.player.position = site;
        carried.damage(PLAYER, lodge, carried.npcs[&lodge].hp, false);
        assert_eq!(
            carried.contested().unwrap().lots[&0].holder,
            Holder::Carried(lodge)
        );
        assert_eq!(carried.claim_lot(0), Ok(Outcome::LotClaimed(0)));
        assert_eq!(
            carried.contested().unwrap().lots[&0].holder,
            Holder::Carried(PLAYER)
        );
        assert_eq!(carried.claim_lot(0), Err(Rejection::Refused));
    }

    #[test]
    fn invisible_lots_and_unfunded_offers_cannot_transfer_goods() {
        let mut world = World::contested_resources(0);
        assert_eq!(
            world.step(world.tick(), Intent::ClaimLot(2)).unwrap(),
            Outcome::Rejected(Rejection::NoEvidence)
        );
        assert_eq!(
            world.contested().unwrap().lots[&2].holder,
            Holder::Ground(Position { x: 29, y: 6 })
        );
        world.player.position = Position { x: 8, y: 4 };
        assert_eq!(world.claim_lot(0), Ok(Outcome::LotClaimed(0)));
        world.player.position = Position { x: 29, y: 4 };
        world.politics.as_mut().unwrap().hall_coins = 0;
        let purse = world.region().unwrap().coins;
        let history = world.contested().unwrap().history.len();
        assert_eq!(
            world.offer_lot(ActorId::new(36), 0),
            Err(Rejection::Refused)
        );
        assert_eq!(
            world.contested().unwrap().lots[&0].holder,
            Holder::Carried(PLAYER)
        );
        assert_eq!(world.contested().unwrap().history.len(), history);
        assert_eq!(world.region().unwrap().coins, purse);
    }

    #[test]
    fn previous_checkpoint_revision_never_spawns_relics() {
        for mut legacy in [World::player_reporting(0), World::physical_recovery(0)] {
            for _ in 0..35 {
                legacy.step(legacy.tick(), Intent::Wait).unwrap();
            }
            let revision = u16::from_le_bytes(legacy.checkpoint()[12..14].try_into().unwrap());
            assert!(revision == 17 || revision == 18);
            let restored = World::restore(&legacy.checkpoint()).unwrap();
            assert_eq!(restored, legacy);
            assert!(restored.contested().is_none());
        }
    }

    #[test]
    fn natural_same_lot_race_is_decided_by_physical_arrival() {
        let mut world = World::contested_resources(0);
        let hall = ActorId::new(36);
        let lodge = ActorId::new(35);
        for _ in 0..2 {
            world.step(world.tick(), Intent::Wait).unwrap();
        }
        assert_eq!(
            world.contested().unwrap().known.get(&(hall, 1)),
            Some(&Position { x: 26, y: 4 })
        );
        assert_eq!(
            world.contested().unwrap().known.get(&(lodge, 1)),
            Some(&Position { x: 26, y: 4 })
        );
        world.step(world.tick(), Intent::Wait).unwrap();
        assert_eq!(
            world.contested().unwrap().lots[&1].holder,
            Holder::Carried(lodge)
        );
        assert_eq!(
            world
                .contested()
                .unwrap()
                .history
                .iter()
                .filter(|event| event.lot == 1 && event.kind == LotEventKind::Claimed)
                .count(),
            1
        );
    }
}
