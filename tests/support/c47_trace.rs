//! Canonical journey to the Road outrider: no injected state or teleport.
use sealed_sim::living::{
    ActorId, Background, Direction, Intent, Outcome, Position, World,
    favors::{Need, Status},
};
use std::collections::{BTreeSet, VecDeque};
pub const TRAVELER: ActorId = ActorId::new(22);
fn dist(a: Position, b: Position) -> u32 {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
}
fn direction(w: &World, target: Position) -> Option<Direction> {
    let start = w.player().position;
    let mut q = VecDeque::from([(start, None)]);
    let mut seen = BTreeSet::from([start]);
    while let Some((p, first)) = q.pop_front() {
        if dist(p, target) <= 1 {
            return first;
        }
        for (d, x, y) in [
            (Direction::North, Some(p.x), p.y.checked_sub(1)),
            (Direction::East, p.x.checked_add(1), Some(p.y)),
            (Direction::South, Some(p.x), p.y.checked_add(1)),
            (Direction::West, p.x.checked_sub(1), Some(p.y)),
        ] {
            let (Some(x), Some(y)) = (x, y) else {
                continue;
            };
            let next = Position { x, y };
            if !w.walkable(next)
                || w.npcs().any(|n| n.alive() && n.position == next)
                || !seen.insert(next)
            {
                continue;
            }
            q.push_back((next, first.or(Some(d))));
        }
    }
    None
}

#[derive(Clone)]
pub struct Trace {
    pub world: World,
    pub commands: Vec<String>,
}
impl Trace {
    pub fn step(&mut self, intent: Intent, command: impl Into<String>) -> Outcome {
        self.commands.push(command.into());
        self.world.step(self.world.tick(), intent).unwrap()
    }
    pub fn walk(&mut self, target: Position) -> bool {
        for _ in 0..250 {
            if !self.world.player().alive() {
                return false;
            }
            if dist(self.world.player().position, target) <= 1 {
                return true;
            }
            let (intent, cmd) = match direction(&self.world, target) {
                Some(d) => (
                    Intent::Move(d),
                    match d {
                        Direction::North => "north",
                        Direction::East => "east",
                        Direction::South => "south",
                        Direction::West => "west",
                    },
                ),
                None => (Intent::Wait, "wait"),
            };
            self.step(intent, cmd);
        }
        false
    }
}

impl Trace {
    fn approach(&mut self, actor: ActorId) -> bool {
        for _ in 0..220 {
            if !self.world.player().alive() {
                return false;
            }
            let Some(n) = self.world.npc(actor).filter(|n| n.alive()) else {
                return false;
            };
            if self.world.at_favor_actor(actor) {
                return true;
            }
            let (intent, cmd) = match direction(&self.world, n.position) {
                Some(d) => (
                    Intent::Move(d),
                    match d {
                        Direction::North => "north",
                        Direction::East => "east",
                        Direction::South => "south",
                        Direction::West => "west",
                    },
                ),
                None => (Intent::Wait, "wait"),
            };
            self.step(intent, cmd);
        }
        false
    }
    fn talk_traveler(&mut self) -> bool {
        self.approach(TRAVELER)
            && self.step(Intent::Talk(TRAVELER), "talk 22") == Outcome::Spoke(TRAVELER)
    }
}
pub struct Journey {
    pub offer: Trace,
    pub accepted: Trace,
    pub fulfilled: Trace,
    pub id: u32,
}
pub fn journey() -> Journey {
    for seed in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 42, 99, u32::MAX] {
        let mut t = Trace {
            world: World::personal_favors_campaign(seed),
            commands: Vec::new(),
        };
        t.step(
            Intent::ChooseBackground(Background::Herbalist),
            "background herbalist",
        );
        if !t.walk(t.world.scenario().patch) {
            continue;
        }
        for _ in 0..40 {
            if t.world.player().herbs >= 2 {
                break;
            }
            t.step(Intent::Gather, "gather");
        }
        if t.world.player().herbs < 2 {
            continue;
        }
        if !t.talk_traveler() {
            continue;
        }
        if !t.approach(TRAVELER) {
            continue;
        }
        if t.step(Intent::Invite(TRAVELER), "invite 22") != Outcome::JoinedCompany(TRAVELER) {
            continue;
        }
        if !t.talk_traveler() {
            continue;
        }
        if !t.approach(TRAVELER) {
            continue;
        }
        let Some(f) = t.world.known_favors().into_iter().find(|s| {
            s.favor.actor == TRAVELER
                && s.favor.need == Need::TravelKit
                && s.favor.status == Status::Offered
        }) else {
            continue;
        };
        let id = f.favor.id;
        let mut a = t.clone();
        if a.step(Intent::AcceptFavor(id), format!("acceptfavor {id}"))
            != Outcome::FavorAccepted(id)
        {
            continue;
        }
        if !a.approach(TRAVELER) {
            continue;
        }
        let mut c = a.clone();
        if c.step(Intent::Provision(TRAVELER), "pack 22") != Outcome::Provisioned(TRAVELER) {
            continue;
        }
        if c.world
            .favor_receipts()
            .last()
            .is_some_and(|s| s.favor.status == Status::Fulfilled && s.favor.id == id)
        {
            return Journey {
                offer: t,
                accepted: a,
                fulfilled: c,
                id,
            };
        }
    }
    panic!("no canonical companion favor journey");
}
pub fn decline(j: &Journey) -> Trace {
    let mut t = j.offer.clone();
    assert_eq!(
        t.step(Intent::DeclineFavor(j.id), format!("declinefavor {}", j.id)),
        Outcome::FavorDeclined(j.id)
    );
    t
}
pub fn withdraw(j: &Journey) -> Trace {
    let mut t = j.accepted.clone();
    assert_eq!(
        t.step(
            Intent::WithdrawFavor(j.id),
            format!("withdrawfavor {}", j.id)
        ),
        Outcome::FavorWithdrawn(j.id)
    );
    t
}
pub fn part(j: &Journey) -> Trace {
    let mut t = j.accepted.clone();
    assert_eq!(
        t.step(Intent::Part(TRAVELER), "part 22"),
        Outcome::Parted(TRAVELER)
    );
    t
}
pub fn unpromised(j: &Journey) -> Trace {
    let mut t = j.offer.clone();
    assert_eq!(
        t.step(Intent::Provision(TRAVELER), "pack 22"),
        Outcome::Provisioned(TRAVELER)
    );
    t
}
