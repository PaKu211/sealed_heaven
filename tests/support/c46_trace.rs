//! Real canonical journeys: no injected inventory or teleports.
use sealed_sim::living::{
    Background, Direction, Intent, Outcome, Position, Region, World,
    requests::{Site, Status},
};
use std::collections::{BTreeSet, VecDeque};
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
pub fn site_position(w: &World, site: Site) -> Position {
    match site {
        Site::Shrine => w.scenario().shrine,
        Site::Market => Region::sites().market,
    }
}
pub struct Journey {
    pub offer: Trace,
    pub accepted: Trace,
    pub completed: Trace,
    pub id: u32,
}
pub fn journey(site: Site) -> Journey {
    for seed in [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        12,
        13,
        14,
        15,
        42,
        99,
        u32::MAX,
    ] {
        let mut t = Trace {
            world: World::emergent_requests_campaign(seed),
            commands: Vec::new(),
        };
        t.step(
            Intent::ChooseBackground(Background::Herbalist),
            "background herbalist",
        );
        let source = match site {
            Site::Shrine => t.world.scenario().patch,
            Site::Market => Region::sites().garden,
        };
        if !t.walk(source) {
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
        if !t.walk(site_position(&t.world, site)) {
            continue;
        }
        for _ in 0..400 {
            if !t.world.player().alive() {
                break;
            }
            let offer = t
                .world
                .known_requests()
                .into_iter()
                .find(|s| s.request.site == site && s.request.status == Status::Offered);
            if let Some(seen) = offer {
                let id = seen.request.id;
                let mut a = t.clone();
                if a.step(Intent::AcceptRequest(id), format!("acceptquest {id}"))
                    == Outcome::RequestAccepted(id)
                {
                    let mut c = a.clone();
                    for _ in 0..40 {
                        let r = c.world.requests().unwrap().board(site).current.unwrap();
                        if r.status == Status::Completed {
                            return Journey {
                                offer: t,
                                accepted: a,
                                completed: c,
                                id,
                            };
                        }
                        if r.status != Status::Accepted || !c.world.player().alive() {
                            break;
                        }
                        match site {
                            Site::Shrine => {
                                c.step(Intent::Deliver, "deliver");
                            }
                            Site::Market if c.world.is_night() => {
                                c.step(Intent::Wait, "wait");
                            }
                            Site::Market => {
                                c.step(Intent::SellHerb, "sell");
                            }
                        }
                    }
                }
            }
            t.step(Intent::Wait, "wait");
        }
    }
    panic!("no natural complete journey for {site:?}");
}
pub fn decline(j: &Journey) -> Trace {
    let mut t = j.offer.clone();
    assert_eq!(
        t.step(
            Intent::DeclineRequest(j.id),
            format!("declinequest {}", j.id)
        ),
        Outcome::RequestDeclined(j.id)
    );
    t
}
pub fn abandon(j: &Journey) -> Trace {
    let mut t = j.accepted.clone();
    let site = t
        .world
        .known_requests()
        .into_iter()
        .find(|s| s.request.id == j.id)
        .unwrap()
        .request
        .site;
    let target = match site {
        Site::Shrine => Region::sites().market,
        Site::Market => t.world.scenario().patch,
    };
    assert!(t.walk(target));
    assert!(!t.world.at_request_site(site));
    assert_eq!(
        t.step(
            Intent::AbandonRequest(j.id),
            format!("abandonquest {}", j.id)
        ),
        Outcome::RequestAbandoned(j.id)
    );
    t
}
