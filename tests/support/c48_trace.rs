//! Replay the accepted C4.7 actual journey, then explore reciprocal choices.
use sealed_sim::living::{
    ActorId, Background, Direction, Intent, Outcome, PLAYER, Position, Rejection, World,
    reciprocity::Asset,
};
use std::collections::{BTreeSet, VecDeque};
pub const TRAVELER: ActorId = ActorId::new(22);
pub const MEDIC: ActorId = ActorId::new(1);
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
}

pub fn command(c: &str) -> Intent {
    match c {
        "background herbalist" => Intent::ChooseBackground(Background::Herbalist),
        "background wanderer" => Intent::ChooseBackground(Background::Wanderer),
        "north" => Intent::Move(Direction::North),
        "east" => Intent::Move(Direction::East),
        "south" => Intent::Move(Direction::South),
        "west" => Intent::Move(Direction::West),
        "wait" => Intent::Wait,
        "gather" => Intent::Gather,
        "guard" => Intent::Guard,
        _ => {
            let (cmd, arg) = c.split_once(' ').unwrap();
            let id: u32 = arg.parse().unwrap();
            let a = ActorId::new(id);
            match cmd {
                "talk" => Intent::Talk(a),
                "invite" => Intent::Invite(a),
                "pack" => Intent::Provision(a),
                "part" => Intent::Part(a),
                "acceptfavor" => Intent::AcceptFavor(id),
                "requestaid" => Intent::RequestAid(a),
                "repayaid" => Intent::RepayAid(a),
                "waiveaid" => Intent::WaiveAid(a),
                "attack" => Intent::Attack(a),
                _ => panic!("unknown canonical trace command {c}"),
            }
        }
    }
}
pub fn initial() -> Trace {
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/living/c47_branch_trace_v1.json")).unwrap();
    let row = data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["choice"] == "fulfill")
        .unwrap();
    let seed = row["seed"].as_u64().unwrap().try_into().unwrap();
    let mut t = Trace {
        world: World::reciprocal_bonds_campaign(seed),
        commands: Vec::new(),
    };
    for c in row["commands"].as_array().unwrap() {
        let c = c.as_str().unwrap();
        t.step(command(c), c);
    }
    assert_eq!(t.world.reciprocity().unwrap().net(PLAYER, TRAVELER), 1);
    assert_eq!(t.world.npc(TRAVELER).unwrap().packed_herbs, 1);
    t
}
pub struct Journey {
    pub one: Trace,
    pub two: Trace,
    pub returned: Trace,
    pub waived: Trace,
    pub parted: Trace,
    pub reserve: Trace,
}
pub fn journey() -> Journey {
    let one = initial();
    let mut reserve = one.clone();
    assert!(reserve.approach(TRAVELER));
    assert!(matches!(
        reserve.step(Intent::RequestAid(TRAVELER), "requestaid 22"),
        Outcome::Rejected(Rejection::Empty | Rejection::Refused)
    ));
    let mut two = one.clone();
    assert!(two.approach(TRAVELER));
    assert_eq!(
        two.step(Intent::Provision(TRAVELER), "pack 22"),
        Outcome::Provisioned(TRAVELER)
    );
    let mut returned = two.clone();
    let mut complete = false;
    for _ in 0..32 {
        assert!(returned.approach(TRAVELER));
        if returned.step(Intent::RequestAid(TRAVELER), "requestaid 22")
            == (Outcome::ReciprocalAid {
                actor: TRAVELER,
                asset: Asset::Herb,
            })
        {
            complete = true;
            break;
        }
    }
    assert!(complete, "surplus herb never returned");
    let mut waived = returned.clone();
    assert!(waived.approach(TRAVELER));
    assert_eq!(
        waived.step(Intent::WaiveAid(TRAVELER), "waiveaid 22"),
        Outcome::AidWaived(TRAVELER)
    );
    let mut parted = two.clone();
    assert!(parted.approach(TRAVELER));
    assert_eq!(
        parted.step(Intent::Part(TRAVELER), "part 22"),
        Outcome::Parted(TRAVELER)
    );
    Journey {
        one,
        two,
        returned,
        waived,
        parted,
        reserve,
    }
}
/// Search only canonical commands; a missing naturally encountered treatment
/// is represented as None rather than writing a fabricated injury into a save.
pub fn clinic_journey() -> Option<Trace> {
    for seed in [0, 1, 2, 7, 42, 99] {
        for target in [TRAVELER, ActorId::new(2), ActorId::new(35)] {
            let mut t = Trace {
                world: World::reciprocal_bonds_campaign(seed),
                commands: Vec::new(),
            };
            let initial_hp = t.world.player().hp;
            t.step(
                Intent::ChooseBackground(Background::Wanderer),
                "background wanderer",
            );
            if !t.approach(target) {
                continue;
            }
            t.step(Intent::Attack(target), format!("attack {}", target.get()));
            if t.world.player().hp == initial_hp {
                t.step(Intent::Guard, "guard");
            }
            if !t.world.player().alive() || t.world.player().hp == initial_hp {
                continue;
            }
            if !t.walk(t.world.scenario().clinic) {
                continue;
            }
            if !t.approach(MEDIC) {
                continue;
            }
            if !matches!(
                t.step(Intent::Talk(MEDIC), "talk 1"),
                Outcome::Treated { .. }
            ) {
                continue;
            }
            if !t.approach(MEDIC) {
                continue;
            }
            if t.step(Intent::RepayAid(MEDIC), "repayaid 1") == Outcome::AidRepaid(MEDIC) {
                return Some(t);
            }
        }
    }
    None
}
