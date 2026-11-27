//! Revision-25 Reed Hall hierarchy, shared service, and physical allocations.

use std::collections::BTreeMap;

use super::navigation::in_reach;
use super::{
    ActorId, Duty, Goal, HallRank, Outcome, PLAYER, Region, Rejection, Role, Scenario, World,
};

const HISTORY_CAP: usize = 256;
const DISCIPLE_MERIT: u8 = 1;
const SENIOR_MERIT: u8 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SectDuty {
    Unassigned,
    Carrying,
    Delivered,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SectMember {
    pub rank: HallRank,
    pub merit: u8,
    pub duty: SectDuty,
    pub allocation_claimed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HierarchyEventKind {
    DutyAccepted,
    DutyDelivered,
    MeritEarned,
    Promoted,
    AllocationClaimed,
    OfficeAssumed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HierarchyEvent {
    pub id: u32,
    pub at: u64,
    pub actor: ActorId,
    pub kind: HierarchyEventKind,
    pub rank: HallRank,
    pub merit: u8,
    pub witnessed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SectHierarchy {
    pub player: Option<SectMember>,
    pub members: BTreeMap<ActorId, SectMember>,
    pub history: Vec<HierarchyEvent>,
    pub next_event: u32,
}

impl SectHierarchy {
    pub(super) fn new(scenario: &Scenario) -> Self {
        let members = scenario
            .npcs
            .iter()
            .filter_map(|npc| {
                let rank = match npc.role {
                    Role::Mentor => HallRank::Elder,
                    Role::HallDeputy => HallRank::Apprentice,
                    Role::HallSteward => HallRank::Disciple,
                    _ => return None,
                };
                Some((
                    npc.id,
                    SectMember {
                        rank,
                        merit: 0,
                        duty: SectDuty::Unassigned,
                        allocation_claimed: false,
                    },
                ))
            })
            .collect();
        Self {
            player: None,
            members,
            history: Vec::new(),
            next_event: 0,
        }
    }
}

impl World {
    pub fn sect_hierarchy(&self) -> Option<&SectHierarchy> {
        self.sect_hierarchy.as_ref()
    }

    pub fn sect_member(&self, actor: ActorId) -> Option<SectMember> {
        if actor == PLAYER {
            self.sect_hierarchy()?.player
        } else {
            self.sect_hierarchy()?.members.get(&actor).copied()
        }
    }

    pub(super) fn hall_tuition_waived(&self) -> bool {
        self.sect_hierarchy().is_some()
            && matches!(
                self.hall_rank(),
                Some(HallRank::Disciple | HallRank::SeniorDisciple | HallRank::Elder)
            )
    }

    pub(super) fn may_draw_hall_qi(&self, actor: ActorId) -> bool {
        if self.sect_hierarchy().is_none() {
            return true;
        }
        self.sect_member(actor).is_some_and(|member| {
            matches!(
                member.rank,
                HallRank::Disciple | HallRank::SeniorDisciple | HallRank::Elder
            )
        })
    }

    pub(super) fn enroll_hierarchy_player(&mut self) {
        let Some(hierarchy) = self.sect_hierarchy.as_mut() else {
            return;
        };
        hierarchy.player = Some(SectMember {
            rank: HallRank::Apprentice,
            merit: 0,
            duty: SectDuty::Unassigned,
            allocation_claimed: false,
        });
    }

    pub(super) fn remove_hierarchy_player(&mut self) {
        if let Some(hierarchy) = self.sect_hierarchy.as_mut() {
            hierarchy.player = None;
        }
    }

    pub(super) fn accept_repeat_player_duty(&mut self) -> Result<Outcome, Rejection> {
        let member = self
            .sect_member(PLAYER)
            .ok_or(Rejection::MembershipRequired)?;
        if member.duty != SectDuty::Unassigned {
            return Err(Rejection::BoundToDuty);
        }
        let region = self.region.as_mut().expect("hierarchy inherits region");
        if region.hall_offerings == 0 {
            return Err(Rejection::Empty);
        }
        region.hall_offerings -= 1;
        self.sect_hierarchy
            .as_mut()
            .unwrap()
            .player
            .as_mut()
            .unwrap()
            .duty = SectDuty::Carrying;
        self.school.as_mut().unwrap().duty = Duty::Carrying;
        self.record_hierarchy_event(PLAYER, HierarchyEventKind::DutyAccepted, true);
        Ok(Outcome::DutyAccepted)
    }

    pub(super) fn note_player_duty_accepted(&mut self) {
        let Some(member) = self
            .sect_hierarchy
            .as_mut()
            .and_then(|hierarchy| hierarchy.player.as_mut())
        else {
            return;
        };
        member.duty = SectDuty::Carrying;
        self.record_hierarchy_event(PLAYER, HierarchyEventKind::DutyAccepted, true);
    }

    pub(super) fn note_player_duty_delivered(&mut self) {
        let Some(member) = self
            .sect_hierarchy
            .as_mut()
            .and_then(|hierarchy| hierarchy.player.as_mut())
        else {
            return;
        };
        member.duty = SectDuty::Delivered;
        self.record_hierarchy_event(PLAYER, HierarchyEventKind::DutyDelivered, true);
    }

    pub(super) fn complete_player_hierarchy_duty(&mut self) {
        if self.sect_hierarchy.is_none() {
            return;
        }
        self.complete_hierarchy_duty(PLAYER, true);
    }

    pub(super) fn note_steward_hall_supply(&mut self, actor: ActorId) {
        let Some(member) = self.sect_member(actor) else {
            return;
        };
        if member.rank == HallRank::Elder {
            return;
        }
        self.complete_hierarchy_duty(actor, false);
    }

    pub(super) fn claim_hall_allocation(&mut self) -> Result<Outcome, Rejection> {
        self.hall_mentor()?;
        let member = self
            .sect_member(PLAYER)
            .ok_or(Rejection::MembershipRequired)?;
        if !matches!(member.rank, HallRank::SeniorDisciple | HallRank::Elder) {
            return Err(Rejection::RankRequired);
        }
        if member.allocation_claimed {
            return Err(Rejection::DutyComplete);
        }
        let region = self.region.as_mut().expect("hierarchy inherits region");
        if region.hall_offerings == 0 {
            return Err(Rejection::Empty);
        }
        region.hall_offerings -= 1;
        self.player.herbs += 1;
        self.sect_hierarchy
            .as_mut()
            .unwrap()
            .player
            .as_mut()
            .unwrap()
            .allocation_claimed = true;
        self.record_hierarchy_event(PLAYER, HierarchyEventKind::AllocationClaimed, true);
        Ok(Outcome::HallAllocationClaimed)
    }

    fn hall_leader_on_duty_for(&self, actor: ActorId) -> bool {
        if self.is_night() {
            return false;
        }
        let Some(leader) = self.school().and_then(|school| school.leader) else {
            return false;
        };
        if leader == actor {
            return false;
        }
        let npc = self.npcs[&leader];
        npc.alive()
            && npc.position == Region::sites().hall
            && !self.social.relationship(leader, actor).refuses_aid()
    }

    pub(super) fn sect_duty_active(&self, actor: ActorId) -> bool {
        self.sect_member(actor)
            .is_some_and(|member| member.duty != SectDuty::Unassigned)
    }

    pub(super) fn sect_duty_goal(&self, actor: ActorId) -> Option<Goal> {
        let member = self.sect_member(actor)?;
        let npc = self.npcs.get(&actor)?;
        if member.rank == HallRank::Elder || !npc.alive() || npc.definition.role != Role::HallDeputy
        {
            return None;
        }
        let hall = Region::sites().hall;
        match member.duty {
            SectDuty::Unassigned => {
                if self.companion(actor).is_some()
                    || npc.fatigue >= npc.definition.rest_threshold
                    || !self.hall_leader_on_duty_for(actor)
                {
                    return None;
                }
                let offerings = self.region()?.hall_offerings;
                let needs_work = (member.merit < SENIOR_MERIT && offerings > 0)
                    || (offerings >= 2 && self.supply.shrine < self.scenario.shrine_target);
                needs_work.then_some(Goal::SectDuty)
            }
            SectDuty::Carrying => (npc.herbs > 0).then_some(Goal::SectDuty),
            SectDuty::Delivered => {
                if in_reach(npc.position, hall) && !self.hall_leader_on_duty_for(actor) {
                    return None;
                }
                Some(Goal::SectDuty)
            }
        }
    }

    pub(super) fn work_sect_duty(&mut self, actor: ActorId) {
        let Some(member) = self.sect_member(actor) else {
            return;
        };
        let hall = Region::sites().hall;
        match member.duty {
            SectDuty::Unassigned => {
                // Defensive revalidation of a duty plan at execution time.
                if self.rules == super::Rules::DutyCoherence && self.companion(actor).is_some() {
                    return;
                }
                if !self.hall_leader_on_duty_for(actor) {
                    return;
                }
                if !in_reach(self.npcs[&actor].position, hall) {
                    self.travel(actor, hall, 1);
                    return;
                }
                let region = self.region.as_mut().expect("hierarchy inherits region");
                if region.hall_offerings == 0 {
                    return;
                }
                region.hall_offerings -= 1;
                self.npcs.get_mut(&actor).unwrap().herbs += 1;
                self.sect_hierarchy
                    .as_mut()
                    .unwrap()
                    .members
                    .get_mut(&actor)
                    .unwrap()
                    .duty = SectDuty::Carrying;
                self.record_hierarchy_event(actor, HierarchyEventKind::DutyAccepted, false);
            }
            SectDuty::Carrying => {
                if self.npcs[&actor].herbs == 0 {
                    self.sect_hierarchy
                        .as_mut()
                        .unwrap()
                        .members
                        .get_mut(&actor)
                        .unwrap()
                        .duty = SectDuty::Unassigned;
                    return;
                }
                if !in_reach(self.npcs[&actor].position, self.scenario.shrine) {
                    self.travel(actor, self.scenario.shrine, 1);
                    return;
                }
                self.npcs.get_mut(&actor).unwrap().herbs -= 1;
                self.supply.shrine += 1;
                self.supply.npc_deliveries += 1;
                self.sect_hierarchy
                    .as_mut()
                    .unwrap()
                    .members
                    .get_mut(&actor)
                    .unwrap()
                    .duty = SectDuty::Delivered;
                self.record_hierarchy_event(actor, HierarchyEventKind::DutyDelivered, false);
            }
            SectDuty::Delivered => {
                if !in_reach(self.npcs[&actor].position, hall) {
                    self.travel(actor, hall, 1);
                    return;
                }
                if !self.hall_leader_on_duty_for(actor) {
                    return;
                }
                self.complete_hierarchy_duty(actor, false);
            }
        }
    }

    pub(super) fn sync_hierarchy_office(&mut self) {
        let Some(leader) = self.school().and_then(|school| school.leader) else {
            return;
        };
        let Some(member) = self.sect_member(leader) else {
            return;
        };
        if member.rank == HallRank::Elder {
            return;
        }
        let hierarchy = self.sect_hierarchy.as_mut().unwrap();
        let member = hierarchy.members.get_mut(&leader).unwrap();
        member.rank = HallRank::Elder;
        member.duty = SectDuty::Unassigned;
        self.record_hierarchy_event(leader, HierarchyEventKind::OfficeAssumed, false);
    }

    pub(super) fn reconcile_sect_hierarchy_inventory(&mut self) {
        let Some(hierarchy) = self.sect_hierarchy.as_mut() else {
            return;
        };
        for (&actor, member) in &mut hierarchy.members {
            if member.duty == SectDuty::Carrying && self.npcs[&actor].herbs == 0 {
                member.duty = SectDuty::Unassigned;
            }
        }
    }

    fn complete_hierarchy_duty(&mut self, actor: ActorId, witnessed: bool) {
        let hierarchy = self.sect_hierarchy.as_mut().expect("hierarchy rules");
        let member = if actor == PLAYER {
            hierarchy.player.as_mut().expect("enrolled player")
        } else {
            hierarchy.members.get_mut(&actor).expect("Hall member")
        };
        member.duty = SectDuty::Unassigned;
        member.merit = member.merit.saturating_add(1);
        let previous = member.rank;
        member.rank = match previous {
            HallRank::Elder => HallRank::Elder,
            HallRank::SeniorDisciple => HallRank::SeniorDisciple,
            HallRank::Disciple if member.merit >= SENIOR_MERIT => HallRank::SeniorDisciple,
            HallRank::Disciple => HallRank::Disciple,
            HallRank::Apprentice if member.merit >= SENIOR_MERIT => HallRank::SeniorDisciple,
            HallRank::Apprentice if member.merit >= DISCIPLE_MERIT => HallRank::Disciple,
            HallRank::Apprentice => HallRank::Apprentice,
        };
        let promoted = member.rank != previous;
        self.record_hierarchy_event(actor, HierarchyEventKind::MeritEarned, witnessed);
        if promoted {
            self.record_hierarchy_event(actor, HierarchyEventKind::Promoted, witnessed);
        }
    }

    fn record_hierarchy_event(
        &mut self,
        actor: ActorId,
        kind: HierarchyEventKind,
        witnessed: bool,
    ) {
        let member = self
            .sect_member(actor)
            .expect("event actor is a Hall member");
        let position = if actor == PLAYER {
            self.player.position
        } else {
            self.npcs[&actor].position
        };
        let witnessed =
            witnessed || (self.player.alive() && self.perceives(self.player.position, position));
        let hierarchy = self.sect_hierarchy.as_mut().expect("hierarchy rules");
        let event = HierarchyEvent {
            id: hierarchy.next_event,
            at: self.tick,
            actor,
            kind,
            rank: member.rank,
            merit: member.merit,
            witnessed,
        };
        hierarchy.next_event = hierarchy
            .next_event
            .checked_add(1)
            .expect("bounded event id");
        if hierarchy.history.len() == HISTORY_CAP {
            hierarchy.history.remove(0);
        }
        hierarchy.history.push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::{Direction, Intent, Outcome, PLAYER, StepError};

    const MEDIC: ActorId = ActorId::new(1);
    const MENTOR: ActorId = ActorId::new(33);
    const DEPUTY: ActorId = ActorId::new(34);
    const STEWARD: ActorId = ActorId::new(36);

    fn turn(world: &mut World, intent: Intent) -> Outcome {
        world.step(world.tick(), intent).unwrap()
    }

    fn walk_to(world: &mut World, destination: super::super::Position, reach: u32) {
        for _ in 0..200 {
            let from = world.player.position;
            if super::super::navigation::distance(from, destination) <= reach {
                return;
            }
            let next = super::super::navigation::next_step(
                world.scenario,
                from,
                destination,
                reach,
                |site| world.walkable(site) && !world.occupied(site),
            );
            let intent = match next {
                Some(site) if site.x > from.x => Intent::Move(Direction::East),
                Some(site) if site.x < from.x => Intent::Move(Direction::West),
                Some(site) if site.y > from.y => Intent::Move(Direction::South),
                Some(_) => Intent::Move(Direction::North),
                None => Intent::Wait,
            };
            turn(world, intent);
        }
        panic!(
            "no path from {:?} to {destination:?} (reach {reach})",
            world.player.position
        );
    }

    fn material_total(world: &World) -> u32 {
        let region = world.region().unwrap();
        world.player.herbs
            + world.supply.source
            + world.supply.shrine
            + world.supply.consumed
            + world
                .npcs
                .values()
                .map(|npc| npc.herbs + u32::from(npc.packed_herbs))
                .sum::<u32>()
            + world.social.recovery_cargo.values().sum::<u32>()
            + region.market_herbs
            + region.garden_herbs
            + region.hall_offerings
            + (region.remedies + region.remedies_used) * 2
            + world.rivalry().unwrap().consumed_herbs
            + world.rivalry().unwrap().accord_offerings
            + world.company().unwrap().herbs_used
            + u32::from(world.breath)
            + world.school().unwrap().form_learned as u32 * 2
            + u32::from(world.school().unwrap().duty == Duty::Carrying)
    }

    fn wait_for_open_hall(world: &mut World) {
        for _ in 0..1_200 {
            if !world.is_night()
                && !world.rivalry().unwrap().contested(world.tick())
                && world
                    .school()
                    .and_then(|school| school.leader)
                    .is_some_and(|leader| {
                        world.npcs[&leader].alive()
                            && world.npcs[&leader].position == Region::sites().hall
                    })
            {
                return;
            }
            turn(world, Intent::Wait);
        }
        panic!("Reed Hall mentor did not return to duty");
    }

    fn prepare_hall_offering(world: &mut World) {
        let hall = Region::sites().hall;
        let garden = Region::sites().garden;
        for _ in 0..3 {
            wait_for_open_hall(world);
            if world.region().unwrap().hall_offerings > 0 {
                return;
            }
            walk_to(world, garden, 1);
            while world.player.herbs < 2 && world.region().unwrap().garden_herbs > 0 {
                turn(world, Intent::Gather);
            }
            if world.player.herbs == 0 {
                let patch = world.scenario.patch;
                walk_to(world, patch, 1);
                turn(world, Intent::Gather);
            }
            walk_to(world, hall, 1);
            wait_for_open_hall(world);
            while world.player.herbs > 0 && world.region().unwrap().hall_offerings < 2 {
                assert_eq!(turn(world, Intent::SupportHall), Outcome::HallSupported);
            }
            wait_for_open_hall(world);
            if world.region().unwrap().hall_offerings > 0 {
                return;
            }
        }
        panic!("could not stock Reed Hall offering");
    }

    #[test]
    fn player_advancement_loop_unlocks_free_drills_and_physical_allocation() {
        let mut world = World::sect_hierarchy_campaign(0);
        let patch = world.scenario.patch;
        let shrine = world.scenario.shrine;
        let baseline_total = material_total(&world);
        assert_eq!(world.hall_rank(), None);

        // Gather four herbs at Qinghe patch and practice breath once at shrine.
        walk_to(&mut world, patch, 1);
        for _ in 0..4 {
            assert_eq!(turn(&mut world, Intent::Gather), Outcome::Gathered);
        }
        walk_to(&mut world, shrine, 1);
        assert_eq!(
            turn(&mut world, Intent::Practice),
            Outcome::Practiced { progress: 1 }
        );

        // Travel to Reed Hall: non-members cannot draw Hall Qi.
        walk_to(&mut world, Region::sites().hall, 1);
        wait_for_open_hall(&mut world);
        assert_eq!(
            turn(&mut world, Intent::DrawQi),
            Outcome::Rejected(Rejection::MembershipRequired)
        );

        // Enroll as Apprentice; Apprentices still cannot draw Hall Qi or claim allocations.
        wait_for_open_hall(&mut world);
        assert_eq!(turn(&mut world, Intent::JoinHall), Outcome::Enrolled);
        assert_eq!(world.hall_rank(), Some(HallRank::Apprentice));
        assert_eq!(world.sect_member(PLAYER).unwrap().merit, 0);
        assert_eq!(
            turn(&mut world, Intent::DrawQi),
            Outcome::Rejected(Rejection::RankRequired)
        );
        assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);

        // Apprentices cannot claim physical Hall allocations yet.
        wait_for_open_hall(&mut world);
        assert_eq!(
            turn(&mut world, Intent::ClaimHallAllocation),
            Outcome::Rejected(Rejection::RankRequired)
        );

        // First Hall duty: carry Hall offering to Qinghe shrine and report back.
        prepare_hall_offering(&mut world);
        assert_eq!(turn(&mut world, Intent::HallDuty), Outcome::DutyAccepted);
        assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
        walk_to(&mut world, shrine, 1);
        assert_eq!(turn(&mut world, Intent::HallDuty), Outcome::DutyDelivered);
        walk_to(&mut world, Region::sites().hall, 1);
        wait_for_open_hall(&mut world);
        assert_eq!(turn(&mut world, Intent::HallDuty), Outcome::DutyReported);
        assert_eq!(world.hall_rank(), Some(HallRank::Disciple));
        assert_eq!(world.sect_member(PLAYER).unwrap().merit, 1);
        assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);

        // Disciple rank unlocks Reed Hall Qi sanctuary and waives Reed Hall martial tuition.
        if world.player.herbs == 0 {
            walk_to(&mut world, patch, 1);
            assert_eq!(turn(&mut world, Intent::Gather), Outcome::Gathered);
            walk_to(&mut world, Region::sites().hall, 1);
        }
        assert_eq!(turn(&mut world, Intent::DrawQi), Outcome::QiDrawn(1));
        wait_for_open_hall(&mut world);
        let coins_before = world.region().unwrap().coins;
        let hall_coins_before = world.politics().unwrap().hall_coins;
        assert_eq!(
            turn(&mut world, Intent::Drill(MENTOR)),
            Outcome::MartialDrilled {
                teacher: MENTOR,
                learned: false
            }
        );
        assert_eq!(world.region().unwrap().coins, coins_before);
        assert_eq!(world.politics().unwrap().hall_coins, hall_coins_before);

        // Disciples still cannot claim Senior Disciple allocations.
        wait_for_open_hall(&mut world);
        assert_eq!(
            turn(&mut world, Intent::ClaimHallAllocation),
            Outcome::Rejected(Rejection::RankRequired)
        );

        // Ensure Reed Hall has an offering, then perform a second duty to reach SeniorDisciple.
        prepare_hall_offering(&mut world);
        assert_eq!(turn(&mut world, Intent::HallDuty), Outcome::DutyAccepted);
        walk_to(&mut world, shrine, 1);
        assert_eq!(turn(&mut world, Intent::HallDuty), Outcome::DutyDelivered);
        walk_to(&mut world, Region::sites().hall, 1);
        wait_for_open_hall(&mut world);
        assert_eq!(turn(&mut world, Intent::HallDuty), Outcome::DutyReported);
        assert_eq!(world.hall_rank(), Some(HallRank::SeniorDisciple));
        assert_eq!(world.sect_member(PLAYER).unwrap().merit, 2);
        assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);

        // Ensure Hall has a physical herb, then claim the SeniorDisciple allocation.
        prepare_hall_offering(&mut world);
        let offerings_before = world.region().unwrap().hall_offerings;
        let player_herbs_before = world.player.herbs;
        assert_eq!(
            turn(&mut world, Intent::ClaimHallAllocation),
            Outcome::HallAllocationClaimed
        );
        assert_eq!(world.region().unwrap().hall_offerings, offerings_before - 1);
        assert_eq!(world.player.herbs, player_herbs_before + 1);
        assert!(world.sect_member(PLAYER).unwrap().allocation_claimed);

        // Cannot claim the allocation twice.
        wait_for_open_hall(&mut world);
        assert_eq!(
            turn(&mut world, Intent::ClaimHallAllocation),
            Outcome::Rejected(Rejection::DutyComplete)
        );
        assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
        assert_eq!(
            material_total(&world),
            baseline_total + world.supply.regrown + world.region().unwrap().garden_regrown
        );
    }

    #[test]
    fn npc_hall_members_perform_duties_and_advance_autonomously_without_player() {
        for seed in [0, 1] {
            let mut world = World::sect_hierarchy_campaign(seed);
            let baseline_total = material_total(&world);
            assert_eq!(
                world.sect_member(DEPUTY).unwrap().rank,
                HallRank::Apprentice
            );
            assert_eq!(world.sect_member(STEWARD).unwrap().rank, HallRank::Disciple);
            assert_eq!(world.sect_member(MENTOR).unwrap().rank, HallRank::Elder);
            assert!(!world.may_draw_hall_qi(DEPUTY));

            for _ in 0..400 {
                turn(&mut world, Intent::Wait);
            }

            let deputy = world.sect_member(DEPUTY).unwrap();
            assert!(
                matches!(deputy.rank, HallRank::Disciple | HallRank::SeniorDisciple),
                "seed {seed}: deputy={deputy:?} steward={:?} offerings={} pressure={} hall_supplies={}",
                world.sect_member(STEWARD),
                world.region().unwrap().hall_offerings,
                world.rivalry().unwrap().pressure,
                world.politics().unwrap().hall_supplies,
            );
            assert!(deputy.merit >= 1);
            assert!(world.may_draw_hall_qi(DEPUTY));
            let steward = world.sect_member(STEWARD).unwrap();
            assert_eq!(steward.rank, HallRank::SeniorDisciple);
            assert!(steward.merit >= SENIOR_MERIT);

            let history = &world.sect_hierarchy().unwrap().history;
            assert!(history.iter().any(|event| {
                event.actor == DEPUTY && event.kind == HierarchyEventKind::DutyAccepted
            }));
            assert!(history.iter().any(|event| {
                event.actor == DEPUTY && event.kind == HierarchyEventKind::DutyDelivered
            }));
            assert!(history.iter().any(|event| {
                event.actor == DEPUTY && event.kind == HierarchyEventKind::Promoted
            }));
            // Player stayed at Qinghe entrance; remote Willowford Hall events are unwitnessed.
            assert!(history.iter().any(|event| {
                event.actor == DEPUTY
                    && event.kind == HierarchyEventKind::DutyAccepted
                    && !event.witnessed
            }));
            assert_eq!(
                material_total(&world),
                baseline_total + world.supply.regrown + world.region().unwrap().garden_regrown
            );
            assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
        }
    }

    #[test]
    fn hall_offering_contention_between_player_and_npc_never_mints_herbs() {
        let mut world = World::sect_hierarchy_campaign(0);
        let hall = Region::sites().hall;
        world.player.position = super::super::Position {
            x: hall.x - 1,
            y: hall.y,
        };
        world.region.as_mut().unwrap().enrolled = true;
        world.region.as_mut().unwrap().hall_offerings = 1;
        world.sect_hierarchy.as_mut().unwrap().player = Some(SectMember {
            rank: HallRank::SeniorDisciple,
            merit: 2,
            duty: SectDuty::Unassigned,
            allocation_claimed: false,
        });
        world.npcs.get_mut(&DEPUTY).unwrap().position = super::super::Position {
            x: hall.x,
            y: hall.y + 1,
        };

        // Player claims the sole Hall herb; deputy must revalidate and take nothing.
        assert_eq!(
            turn(&mut world, Intent::ClaimHallAllocation),
            Outcome::HallAllocationClaimed
        );
        assert_eq!(world.region().unwrap().hall_offerings, 0);
        assert_eq!(world.player.herbs, 1);
        assert_eq!(world.npcs[&DEPUTY].herbs, 0);
        assert_eq!(
            world.sect_member(DEPUTY).unwrap().duty,
            SectDuty::Unassigned
        );

        // With stock now empty, player repeat duty also fails without mutating state.
        assert_eq!(
            turn(&mut world, Intent::HallDuty),
            Outcome::Rejected(Rejection::Empty)
        );
        assert_eq!(world.region().unwrap().hall_offerings, 0);
    }

    #[test]
    fn mentor_death_blocks_headless_reporting_and_succession_promotes_deputy_to_elder() {
        let mut world = World::sect_hierarchy_campaign(0);
        let hall = Region::sites().hall;
        world
            .sect_hierarchy
            .as_mut()
            .unwrap()
            .members
            .get_mut(&DEPUTY)
            .unwrap()
            .duty = SectDuty::Delivered;
        world.npcs.get_mut(&DEPUTY).unwrap().position = super::super::Position {
            x: hall.x,
            y: hall.y + 1,
        };
        // Mentor falls; deputy is right beside the Hall and discovers the death.
        world.npcs.get_mut(&MENTOR).unwrap().hp = 0;

        assert!(!world.hall_leader_on_duty_for(DEPUTY));
        turn(&mut world, Intent::Wait);
        assert_eq!(world.school().unwrap().leader, Some(DEPUTY));
        let deputy = world.sect_member(DEPUTY).unwrap();
        assert_eq!(deputy.rank, HallRank::Elder);
        assert_eq!(deputy.duty, SectDuty::Unassigned);
        assert!(world.sect_hierarchy().unwrap().history.iter().any(|event| {
            event.actor == DEPUTY
                && event.kind == HierarchyEventKind::OfficeAssumed
                && event.rank == HallRank::Elder
        }));

        // Successor Elder Luo Xiu who mastered Reed Palm can teach drills once on duty.
        world
            .martial
            .as_mut()
            .unwrap()
            .skills
            .get_mut(&DEPUTY)
            .unwrap()
            .form = Some(super::super::Form::ReedPalm);
        world.player.position = super::super::Position {
            x: hall.x - 1,
            y: hall.y,
        };
        world.region.as_mut().unwrap().coins = 1;
        wait_for_open_hall(&mut world);
        assert_eq!(
            turn(&mut world, Intent::Drill(DEPUTY)),
            Outcome::MartialDrilled {
                teacher: DEPUTY,
                learned: false
            }
        );
    }

    #[test]
    fn lodge_oath_forfeits_hierarchy_and_blocks_while_carrying_repeat_duty() {
        let mut world = World::sect_hierarchy_campaign(0);
        let envoy = ActorId::new(35);
        world.player.position = super::super::Position { x: 24, y: 5 };
        world.player.herbs = 2;
        world.social.meet(envoy, PLAYER);
        world.region.as_mut().unwrap().enrolled = true;
        world.school.as_mut().unwrap().duty = Duty::Carrying;
        world.sect_hierarchy.as_mut().unwrap().player = Some(SectMember {
            rank: HallRank::Disciple,
            merit: 1,
            duty: SectDuty::Carrying,
            allocation_claimed: false,
        });

        assert_eq!(
            turn(&mut world, Intent::PledgeLodge),
            Outcome::Rejected(Rejection::BoundToDuty)
        );
        assert_eq!(world.hall_rank(), Some(HallRank::Disciple));

        world.school.as_mut().unwrap().duty = Duty::Reported;
        world
            .sect_hierarchy
            .as_mut()
            .unwrap()
            .player
            .as_mut()
            .unwrap()
            .duty = SectDuty::Unassigned;
        assert_eq!(turn(&mut world, Intent::PledgeLodge), Outcome::LodgePledged);
        assert_eq!(world.hall_rank(), None);
        assert!(world.sect_member(PLAYER).is_none());
    }

    #[test]
    fn revision_25_inherits_wards_and_reconciles_external_npc_duty_custody() {
        for mut world in [
            World::autonomous_agency(0),
            World::sect_hierarchy_campaign(0),
        ] {
            world.contested.as_mut().unwrap().wards = 1;
            let hp = world.player.hp;
            world.damage(MEDIC, PLAYER, 2, false);
            assert_eq!(world.player.hp, hp - 1);
            assert_eq!(world.contested().unwrap().wards, 0);
        }

        let mut world = World::sect_hierarchy_campaign(0);
        world.npcs.get_mut(&DEPUTY).unwrap().herbs = 0;
        world
            .sect_hierarchy
            .as_mut()
            .unwrap()
            .members
            .get_mut(&DEPUTY)
            .unwrap()
            .duty = SectDuty::Carrying;
        turn(&mut world, Intent::Wait);
        assert_eq!(
            world.sect_member(DEPUTY).unwrap().duty,
            SectDuty::Unassigned
        );
    }

    #[test]
    fn hierarchy_history_is_bounded_with_monotonic_ids_and_enters_digest() {
        let mut world = World::sect_hierarchy_campaign(0);
        for _ in 0..300 {
            world.record_hierarchy_event(DEPUTY, HierarchyEventKind::MeritEarned, false);
        }
        let history = &world.sect_hierarchy().unwrap().history;
        assert_eq!(history.len(), HISTORY_CAP);
        assert_eq!(history.first().unwrap().id, 44);
        assert_eq!(history.last().unwrap().id, 299);
        assert_eq!(world.sect_hierarchy().unwrap().next_event, 300);

        let base = super::super::checkpoint::state_digest(&world);
        let mut altered_merit = world.clone();
        altered_merit
            .sect_hierarchy
            .as_mut()
            .unwrap()
            .members
            .get_mut(&DEPUTY)
            .unwrap()
            .merit = 9;
        assert_ne!(super::super::checkpoint::state_digest(&altered_merit), base);

        let mut altered_next = world.clone();
        altered_next.sect_hierarchy.as_mut().unwrap().next_event += 1;
        assert_ne!(super::super::checkpoint::state_digest(&altered_next), base);
    }

    #[test]
    fn baseline_revision_24_checkpoints_remain_byte_exact_and_reject_v25_intent() {
        let mut legacy = World::autonomous_agency(0);
        assert_eq!(
            legacy.step(0, Intent::ClaimHallAllocation),
            Err(StepError::SchoolCampaignRequired)
        );
        assert_eq!(legacy.tick(), 0);

        let passive_golden = include_bytes!("../../tests/fixtures/living/revision24.bin");
        let mut restored = World::restore(passive_golden).unwrap();
        assert_eq!(restored.checkpoint(), passive_golden);
        let mut rebuilt = World::autonomous_agency(7);
        for _ in 0..64 {
            turn(&mut rebuilt, Intent::Wait);
        }
        assert_eq!(rebuilt.checkpoint(), passive_golden);
        for _ in 0..40 {
            assert_eq!(
                turn(&mut restored, Intent::Wait),
                turn(&mut rebuilt, Intent::Wait)
            );
            assert_eq!(restored.checkpoint(), rebuilt.checkpoint());
        }

        let wounded_golden = include_bytes!("../../tests/fixtures/living/revision24_wounded.bin");
        let mut restored_wounded = World::restore(wounded_golden).unwrap();
        assert_eq!(restored_wounded.checkpoint(), wounded_golden);
        let mut rebuilt_wounded = World::autonomous_agency(0);
        for direction in [Direction::North, Direction::East, Direction::East] {
            turn(&mut rebuilt_wounded, Intent::Move(direction));
        }
        turn(&mut rebuilt_wounded, Intent::Attack(MEDIC));
        for (direction, count) in [
            (Direction::North, 3),
            (Direction::East, 9),
            (Direction::South, 2),
            (Direction::North, 2),
            (Direction::West, 6),
            (Direction::South, 3),
            (Direction::West, 2),
        ] {
            for _ in 0..count {
                turn(&mut rebuilt_wounded, Intent::Move(direction));
            }
        }
        turn(&mut rebuilt_wounded, Intent::Attack(MEDIC));
        for _ in 0..140 {
            turn(&mut rebuilt_wounded, Intent::Wait);
        }
        assert_eq!(rebuilt_wounded.checkpoint(), wounded_golden);
        for _ in 0..40 {
            assert_eq!(
                turn(&mut restored_wounded, Intent::Wait),
                turn(&mut rebuilt_wounded, Intent::Wait)
            );
            assert_eq!(restored_wounded.checkpoint(), rebuilt_wounded.checkpoint());
        }
    }

    #[test]
    fn multi_seed_thousand_turn_campaigns_are_deterministic_and_replayable() {
        for seed in [0, 1, 2, 7] {
            let mut left = World::sect_hierarchy_campaign(seed);
            let mut right = World::sect_hierarchy_campaign(seed);
            let baseline_total = material_total(&left);
            for turn_index in 1..=1_000 {
                assert_eq!(
                    turn(&mut left, Intent::Wait),
                    turn(&mut right, Intent::Wait)
                );
                if matches!(turn_index, 200 | 500 | 1_000) {
                    assert_eq!(left.checkpoint(), right.checkpoint());
                    left = World::restore(&left.checkpoint()).unwrap();
                    assert_eq!(left, right);
                    let hierarchy = left.sect_hierarchy().unwrap();
                    assert!(hierarchy.history.len() <= HISTORY_CAP);
                    assert_eq!(
                        material_total(&left),
                        baseline_total
                            + left.supply.regrown
                            + left.region().unwrap().garden_regrown
                    );
                    for (&actor, member) in &hierarchy.members {
                        if member.duty == SectDuty::Carrying {
                            assert!(left.npcs[&actor].herbs > 0);
                        }
                    }
                    for (&actor, state) in &left.character_agency().unwrap().people {
                        assert!(u32::from(state.livelihood_herbs) <= left.npcs[&actor].herbs);
                    }
                }
            }
        }
    }

    fn ready_to_invite_deputy(mut world: World) -> World {
        world.player.position = super::super::Position { x: 26, y: 7 };
        world.npcs.get_mut(&DEPUTY).unwrap().position = super::super::Position { x: 27, y: 7 };
        world.region.as_mut().unwrap().enrolled = true;
        world.sect_hierarchy.as_mut().unwrap().player = Some(SectMember {
            rank: HallRank::Apprentice,
            merit: 0,
            duty: SectDuty::Unassigned,
            allocation_claimed: false,
        });
        world.social.meet(DEPUTY, PLAYER);
        world
    }

    #[test]
    fn revision26_refuses_both_active_duty_phases_but_revision25_preserves_legacy_behavior() {
        for duty in [SectDuty::Carrying, SectDuty::Delivered] {
            let mut current = ready_to_invite_deputy(World::duty_coherence_campaign(0));
            let mut old = ready_to_invite_deputy(World::sect_hierarchy_campaign(0));
            for world in [&mut current, &mut old] {
                world
                    .sect_hierarchy
                    .as_mut()
                    .unwrap()
                    .members
                    .get_mut(&DEPUTY)
                    .unwrap()
                    .duty = duty;
                if duty == SectDuty::Carrying {
                    world.npcs.get_mut(&DEPUTY).unwrap().herbs += 1;
                    world.supply.source -= 1;
                }
            }
            let before = current.clone();
            assert_eq!(current.invite(DEPUTY), Err(Rejection::BoundToDuty));
            assert_eq!(current, before);
            assert_eq!(old.invite(DEPUTY), Ok(Outcome::JoinedCompany(DEPUTY)));
            assert!(old.companion(DEPUTY).is_some());
            assert_eq!(old.sect_member(DEPUTY).unwrap().duty, duty);
            assert_eq!(
                turn(&mut current, Intent::Invite(DEPUTY)),
                Outcome::Rejected(Rejection::BoundToDuty)
            );
            assert!(current.companion(DEPUTY).is_none());
        }
    }

    #[test]
    fn revision26_companion_cannot_pick_up_new_duty_and_dismissal_reenables_it() {
        let mut world = ready_to_invite_deputy(World::duty_coherence_campaign(0));
        let hall = Region::sites().hall;
        world.npcs.get_mut(&MENTOR).unwrap().position = hall;
        world.npcs.get_mut(&DEPUTY).unwrap().position = super::super::Position {
            x: hall.x,
            y: hall.y + 1,
        };
        world.player.position = super::super::Position {
            x: hall.x - 1,
            y: hall.y + 1,
        };
        while world.is_night() {
            world.tick += 1;
        }
        world.region.as_mut().unwrap().hall_offerings = 1;
        world.npcs.get_mut(&DEPUTY).unwrap().fatigue = 0;
        assert!(world.hall_leader_on_duty_for(DEPUTY));
        assert_eq!(world.sect_duty_goal(DEPUTY), Some(Goal::SectDuty));
        assert_eq!(world.invite(DEPUTY), Ok(Outcome::JoinedCompany(DEPUTY)));
        assert_eq!(world.sect_duty_goal(DEPUTY), None);
        let before = world.clone();
        world.work_sect_duty(DEPUTY);
        assert_eq!(
            world, before,
            "a stale duty plan must not acquire offerings"
        );
        assert_eq!(world.part(DEPUTY), Ok(Outcome::Parted(DEPUTY)));
        assert_eq!(world.sect_duty_goal(DEPUTY), Some(Goal::SectDuty));
        let total = material_total(&world);
        world.work_sect_duty(DEPUTY);
        assert_eq!(world.sect_member(DEPUTY).unwrap().duty, SectDuty::Carrying);
        assert_eq!(world.region().unwrap().hall_offerings, 0);
        assert_eq!(material_total(&world), total);
    }

    #[test]
    fn authentic_revision25_fixtures_and_40_turn_continuations_are_byte_exact() {
        let fixtures: [(&[u8], &[u8]); 2] = [
            (
                include_bytes!("../../tests/fixtures/living/revision25.bin"),
                include_bytes!("../../tests/fixtures/living/revision25_plus40.bin"),
            ),
            (
                include_bytes!("../../tests/fixtures/living/revision25_mixed.bin"),
                include_bytes!("../../tests/fixtures/living/revision25_mixed_plus40.bin"),
            ),
        ];
        for (initial, continuation) in fixtures {
            let mut world = World::restore(initial).unwrap();
            assert_eq!(world.checkpoint_revision(), 25);
            assert_eq!(world.checkpoint(), initial);
            for _ in 0..40 {
                turn(&mut world, Intent::Wait);
            }
            assert_eq!(world.checkpoint(), continuation);
        }
    }

    #[test]
    fn revision26_preserves_hierarchy_resource_and_replay_invariants_across_four_seeds() {
        for seed in [0, 1, 2, 7] {
            let mut left = World::duty_coherence_campaign(seed);
            let mut right = World::duty_coherence_campaign(seed);
            assert_eq!(left.checkpoint_revision(), 26);
            assert_ne!(
                left.checkpoint(),
                World::sect_hierarchy_campaign(seed).checkpoint()
            );
            let total = material_total(&left);
            for i in 1..=1000 {
                assert_eq!(
                    turn(&mut left, Intent::Wait),
                    turn(&mut right, Intent::Wait)
                );
                if matches!(i, 200 | 500 | 1000) {
                    assert_eq!(left.checkpoint(), right.checkpoint());
                    left = World::restore(&left.checkpoint()).unwrap();
                    assert_eq!(left, right);
                    assert_eq!(
                        material_total(&left),
                        total + left.supply.regrown + left.region().unwrap().garden_regrown
                    );
                    assert!(left.sect_hierarchy().unwrap().history.len() <= HISTORY_CAP);
                    for (&actor, member) in &left.sect_hierarchy().unwrap().members {
                        if member.duty == SectDuty::Carrying {
                            assert!(left.npcs[&actor].herbs > 0);
                        }
                        if member.duty != SectDuty::Unassigned {
                            assert!(left.companion(actor).is_none());
                        }
                    }
                }
            }
        }
    }

    // Unlike the isolated branch-level fixtures, every state in this test is
    // reached using public commands from a real scenario seed.
    #[test]
    fn revision26_real_campaign_refuses_invitation_during_duty_and_replays() {
        let mut world = World::duty_coherence_campaign(0);
        let patch = world.scenario.patch;
        let shrine = world.scenario.shrine;
        walk_to(&mut world, patch, 1);
        for _ in 0..4 {
            assert_eq!(turn(&mut world, Intent::Gather), Outcome::Gathered);
        }
        walk_to(&mut world, shrine, 1);
        assert_eq!(
            turn(&mut world, Intent::Practice),
            Outcome::Practiced { progress: 1 }
        );
        walk_to(&mut world, Region::sites().hall, 1);
        wait_for_open_hall(&mut world);
        assert_eq!(turn(&mut world, Intent::JoinHall), Outcome::Enrolled);
        for _ in 0..800 {
            let npc = world.npcs[&DEPUTY];
            assert!(npc.alive() && world.player.alive());
            let from = world.player.position;
            let to = npc.position;
            if in_reach(from, to) {
                if world.social.relationship(DEPUTY, PLAYER).familiarity == 0 {
                    turn(&mut world, Intent::Talk(DEPUTY));
                } else if world.sect_duty_active(DEPUTY) {
                    assert_eq!(
                        turn(&mut world, Intent::Invite(DEPUTY)),
                        Outcome::Rejected(Rejection::BoundToDuty)
                    );
                    assert!(world.companion(DEPUTY).is_none());
                    let bytes = world.checkpoint();
                    assert_eq!(World::restore(&bytes).unwrap(), world);
                    assert_eq!(World::restore(&bytes).unwrap().checkpoint(), bytes);
                    println!("Active duty invitation refused at turn {}", world.tick());
                    return;
                } else {
                    turn(&mut world, Intent::Wait);
                }
            } else {
                let next =
                    super::super::navigation::next_step(world.scenario, from, to, 1, |site| {
                        world.walkable(site) && !world.occupied(site)
                    });
                let intent = match next {
                    Some(site) if site.x > from.x => Intent::Move(Direction::East),
                    Some(site) if site.x < from.x => Intent::Move(Direction::West),
                    Some(site) if site.y > from.y => Intent::Move(Direction::South),
                    Some(_) => Intent::Move(Direction::North),
                    None => Intent::Wait,
                };
                turn(&mut world, intent);
            }
        }
        panic!("No active duty invitation opportunity in seeded scenario");
    }
}
