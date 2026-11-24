//! Authored Qinghe map, identities and duties. Rules consume Scenario values.
use super::{ActorId, Invitation, NpcDefinition, Position, Role, Rules, Scenario};

const HEALER: ActorId = ActorId::new(1);
const GUARD: ActorId = ActorId::new(2);
const HERBALIST: ActorId = ActorId::new(3);
const OUTRIDER: ActorId = ActorId::new(22);

const fn p(x: u32, y: u32) -> Position {
    Position { x, y }
}

const NPCS: [NpcDefinition; 4] = [
    NpcDefinition {
        id: HEALER,
        name: "Yao Lian",
        glyph: 'Y',
        role: Role::Medic,
        spawn: p(3, 3),
        home: p(2, 3),
        route: &[],
        hp: 6,
        rest_threshold: 16,
        responds_to_attack: false,
        shares_news: true,
        report_to: None,
    },
    NpcDefinition {
        id: GUARD,
        name: "Shen Wei",
        glyph: 'G',
        role: Role::Watch,
        spawn: p(5, 4),
        home: p(4, 3),
        route: &[p(5, 4), p(5, 5), p(4, 5), p(4, 4)],
        hp: 6,
        rest_threshold: 14,
        responds_to_attack: true,
        shares_news: true,
        report_to: Some(HEALER),
    },
    NpcDefinition {
        id: HERBALIST,
        name: "Lin Su",
        glyph: 'H',
        role: Role::Supplier,
        spawn: p(8, 2),
        home: p(8, 1),
        route: &[p(8, 2), p(7, 3), p(7, 5), p(9, 5)],
        hp: 6,
        rest_threshold: 22,
        responds_to_attack: false,
        shares_news: true,
        report_to: Some(GUARD),
    },
    NpcDefinition {
        id: OUTRIDER,
        name: "Road outrider",
        glyph: 'R',
        role: Role::Traveler,
        spawn: p(9, 4),
        home: p(10, 6),
        route: &[p(9, 4), p(10, 4), p(10, 5), p(9, 5), p(8, 5), p(8, 4)],
        hp: 4,
        rest_threshold: 18,
        responds_to_attack: true,
        shares_news: false,
        report_to: None,
    },
];

pub(super) static QINGHE: Scenario = Scenario {
    terrain: &[
        "............",
        "......#.....",
        "......#.....",
        "......#.....",
        "............",
        "......#.....",
        "......#.....",
        "......#.....",
        "............",
    ],
    contacts: &[(GUARD, HEALER), (HERBALIST, GUARD)],
    ..QINGHE_CONTENT
};

const REGIONAL_NPCS: &[NpcDefinition] = &[
    NPCS[0],
    NPCS[1],
    NPCS[2],
    NPCS[3],
    NpcDefinition {
        id: ActorId::new(31),
        name: "Mei Fen",
        glyph: 'F',
        role: Role::Grower,
        spawn: p(27, 2),
        home: p(28, 2),
        route: &[],
        hp: 6,
        rest_threshold: 26,
        responds_to_attack: false,
        shares_news: true,
        report_to: Some(ActorId::new(32)),
    },
    NpcDefinition {
        id: ActorId::new(32),
        name: "Bao Jun",
        glyph: 'C',
        role: Role::Courier,
        spawn: p(23, 4),
        home: p(22, 5),
        route: &[],
        hp: 6,
        rest_threshold: 40,
        responds_to_attack: true,
        shares_news: true,
        report_to: Some(GUARD),
    },
    NpcDefinition {
        id: ActorId::new(33),
        name: "Ren Zhi",
        glyph: 'M',
        role: Role::Mentor,
        spawn: p(27, 6),
        home: p(28, 6),
        route: &[],
        hp: 6,
        rest_threshold: 22,
        responds_to_attack: true,
        shares_news: true,
        report_to: Some(ActorId::new(32)),
    },
];

const REGIONAL_CONTACTS: &[(ActorId, ActorId)] = &[
    (GUARD, HEALER),
    (HERBALIST, GUARD),
    (ActorId::new(31), ActorId::new(32)),
    (ActorId::new(32), GUARD),
    (ActorId::new(33), ActorId::new(32)),
];

const HALL_NPCS: &[NpcDefinition] = &[
    REGIONAL_NPCS[0],
    REGIONAL_NPCS[1],
    REGIONAL_NPCS[2],
    REGIONAL_NPCS[3],
    REGIONAL_NPCS[4],
    REGIONAL_NPCS[5],
    REGIONAL_NPCS[6],
    NpcDefinition {
        id: ActorId::new(34),
        name: "Luo Xiu",
        glyph: 'X',
        role: Role::HallDeputy,
        spawn: p(27, 7),
        home: p(29, 7),
        route: &[p(27, 7), p(28, 7), p(28, 5)],
        hp: 6,
        rest_threshold: 24,
        responds_to_attack: true,
        shares_news: true,
        report_to: Some(ActorId::new(32)),
    },
];

const HALL_RECRUITS: &[(ActorId, Invitation)] = &[
    (OUTRIDER, Invitation::OpenRoad),
    (ActorId::new(34), Invitation::SchoolMember),
];

const HALL_CONTACTS: &[(ActorId, ActorId)] = &[
    REGIONAL_CONTACTS[0],
    REGIONAL_CONTACTS[1],
    REGIONAL_CONTACTS[2],
    REGIONAL_CONTACTS[3],
    REGIONAL_CONTACTS[4],
    (ActorId::new(33), ActorId::new(34)),
    (ActorId::new(34), ActorId::new(32)),
];

const RIVAL_NPCS: &[NpcDefinition] = &[
    HALL_NPCS[0],
    HALL_NPCS[1],
    HALL_NPCS[2],
    HALL_NPCS[3],
    HALL_NPCS[4],
    HALL_NPCS[5],
    HALL_NPCS[6],
    HALL_NPCS[7],
    NpcDefinition {
        id: ActorId::new(35),
        name: "Stonebank lodge envoy",
        glyph: 'V',
        role: Role::RivalEnvoy,
        spawn: p(25, 5),
        home: p(26, 6),
        route: &[p(25, 5), p(26, 5)],
        hp: 6,
        rest_threshold: 25,
        responds_to_attack: true,
        shares_news: true,
        report_to: Some(ActorId::new(31)),
    },
];

const RIVAL_CONTACTS: &[(ActorId, ActorId)] = &[
    HALL_CONTACTS[0],
    HALL_CONTACTS[1],
    HALL_CONTACTS[2],
    HALL_CONTACTS[3],
    HALL_CONTACTS[4],
    HALL_CONTACTS[5],
    HALL_CONTACTS[6],
    (ActorId::new(35), ActorId::new(31)),
];

const POLITICAL_NPCS: &[NpcDefinition] = &[
    RIVAL_NPCS[0],
    RIVAL_NPCS[1],
    RIVAL_NPCS[2],
    RIVAL_NPCS[3],
    RIVAL_NPCS[4],
    RIVAL_NPCS[5],
    RIVAL_NPCS[6],
    RIVAL_NPCS[7],
    RIVAL_NPCS[8],
    NpcDefinition {
        id: ActorId::new(36),
        name: "Qiao Lin, Hall steward",
        glyph: 'Q',
        role: Role::HallSteward,
        spawn: p(29, 4),
        home: p(29, 5),
        route: &[p(29, 4), p(29, 5)],
        hp: 6,
        rest_threshold: 24,
        responds_to_attack: true,
        shares_news: true,
        report_to: Some(ActorId::new(33)),
    },
];

const POLITICAL_CONTACTS: &[(ActorId, ActorId)] = &[
    RIVAL_CONTACTS[0],
    RIVAL_CONTACTS[1],
    RIVAL_CONTACTS[2],
    RIVAL_CONTACTS[3],
    RIVAL_CONTACTS[4],
    RIVAL_CONTACTS[5],
    RIVAL_CONTACTS[6],
    RIVAL_CONTACTS[7],
    (ActorId::new(36), ActorId::new(33)),
    (ActorId::new(36), ActorId::new(34)),
];

const POLITICAL_RECRUITS: &[(ActorId, Invitation)] = &[
    HALL_RECRUITS[0],
    HALL_RECRUITS[1],
    (ActorId::new(35), Invitation::LodgeMember),
    (ActorId::new(36), Invitation::SchoolMember),
];

const CLOSED_EDGE: &str = concat!("............", "#########", ".........");
const HIGH_GARDEN: &str = concat!("......#.....", "####...##", ".........");
const MID_ROAD: &str = concat!("............", ".........", ".........");
const MID_BLOCKED: &str = concat!("............", "####...##", ".........");
const LOW_ROAD: &str = concat!("......#.....", ".........", ".........");

const RIDGE_TERRAIN: &[&str] = &[
    CLOSED_EDGE,
    HIGH_GARDEN,
    HIGH_GARDEN,
    LOW_ROAD,
    MID_ROAD,
    HIGH_GARDEN,
    HIGH_GARDEN,
    HIGH_GARDEN,
    CLOSED_EDGE,
];
const LOWER_TERRAIN: &[&str] = &[
    CLOSED_EDGE,
    HIGH_GARDEN,
    HIGH_GARDEN,
    HIGH_GARDEN,
    MID_BLOCKED,
    HIGH_GARDEN,
    LOW_ROAD,
    LOW_ROAD,
    CLOSED_EDGE,
];

static RIDGE_ROAD: Scenario = Scenario {
    terrain: RIDGE_TERRAIN,
    ..RIDGE_CONTENT
};

static LOWER_ROAD: Scenario = Scenario {
    terrain: LOWER_TERRAIN,
    ..RIDGE_CONTENT
};

static RIDGE_HALL: Scenario = Scenario {
    terrain: RIDGE_TERRAIN,
    npcs: HALL_NPCS,
    contacts: HALL_CONTACTS,
    recruits: HALL_RECRUITS,
    ..RIDGE_CONTENT
};

static LOWER_HALL: Scenario = Scenario {
    terrain: LOWER_TERRAIN,
    npcs: HALL_NPCS,
    contacts: HALL_CONTACTS,
    recruits: HALL_RECRUITS,
    ..RIDGE_CONTENT
};

static RIDGE_RIVALRY: Scenario = Scenario {
    terrain: RIDGE_TERRAIN,
    npcs: RIVAL_NPCS,
    contacts: RIVAL_CONTACTS,
    recruits: HALL_RECRUITS,
    ..RIDGE_CONTENT
};

static LOWER_RIVALRY: Scenario = Scenario {
    terrain: LOWER_TERRAIN,
    npcs: RIVAL_NPCS,
    contacts: RIVAL_CONTACTS,
    recruits: HALL_RECRUITS,
    ..RIDGE_CONTENT
};

static RIDGE_POLITICS: Scenario = Scenario {
    terrain: RIDGE_TERRAIN,
    npcs: POLITICAL_NPCS,
    contacts: POLITICAL_CONTACTS,
    recruits: POLITICAL_RECRUITS,
    ..RIDGE_CONTENT
};

static LOWER_POLITICS: Scenario = Scenario {
    terrain: LOWER_TERRAIN,
    npcs: POLITICAL_NPCS,
    contacts: POLITICAL_CONTACTS,
    recruits: POLITICAL_RECRUITS,
    ..RIDGE_CONTENT
};

const QINGHE_CONTENT: Scenario = Scenario {
    name: "Qinghe",
    width: 12,
    height: 9,
    terrain: &[],
    entrance: p(0, 4),
    patch: p(9, 2),
    shrine: p(3, 5),
    clinic: p(3, 3),
    shrine_target: 5,
    initial_source: 12,
    regrow_every: 12,
    npcs: &NPCS,
    contacts: &[],
    recruits: &[],
};

const RIDGE_CONTENT: Scenario = Scenario {
    name: "Qinghe and Willowford",
    width: 30,
    npcs: REGIONAL_NPCS,
    contacts: REGIONAL_CONTACTS,
    ..QINGHE_CONTENT
};

pub(super) fn region(seed: u32, rules: Rules) -> &'static Scenario {
    if matches!(
        rules,
        Rules::SectPolitics
            | Rules::TrafficCourtesy
            | Rules::LivingAgency
            | Rules::FactionAgency
            | Rules::InstitutionalAgency
            | Rules::ContractAccountability
            | Rules::InformationDisputes
            | Rules::PlayerAgency
            | Rules::Consequences
            | Rules::ViolenceLogistics
            | Rules::PlayerReporting
            | Rules::PhysicalRecovery
            | Rules::ContestedResources
            | Rules::SectCultivation
    ) {
        return if seed & 1 == 0 {
            &RIDGE_POLITICS
        } else {
            &LOWER_POLITICS
        };
    }
    if rules == Rules::SectRivalry {
        return if seed & 1 == 0 {
            &RIDGE_RIVALRY
        } else {
            &LOWER_RIVALRY
        };
    }
    match (seed & 1 == 0, rules.has_school()) {
        (true, true) => &RIDGE_HALL,
        (false, true) => &LOWER_HALL,
        (true, false) => &RIDGE_ROAD,
        (false, false) => &LOWER_ROAD,
    }
}
