//! Desktop projection of the regional campaign. Keyboard actions enter the
//! same session as terminal commands, and rendering never advances the clock.

use bevy::{prelude::*, window::PrimaryWindow};
use sealed_sim::living::{ActorId, BREATH_MASTERY, MAX_QI, Position, Region};

use crate::{
    clock,
    living_ascii::{
        Session, bonds_session, campaign_session, circles_session, communication_session,
        cooperation_session, development_session, diplomacy_session, encounter_session,
        expanded_session, favors_session, generated_session, governance_session, history_session,
        knowledge_session, lod_session, narrative_session, obligations_session, outcome_label,
        production_session, requests_session, rumor_session, social_network_session,
    },
    locale::Locale,
    locale_living::{living_format, living_text},
};

const SIDE: f32 = 32.0;
const INK: Color = Color::srgb(0.94, 0.90, 0.78);
const MOSS: Color = Color::srgb(0.18, 0.26, 0.22);
const OCHRE: Color = Color::srgb(0.80, 0.63, 0.36);
const STONE: Color = Color::srgb(0.30, 0.35, 0.38);
const FOG: Color = Color::srgb(0.07, 0.09, 0.10);

#[derive(Resource)]
struct Game {
    session: Session,
    target: Option<ActorId>,
}

#[derive(Component)]
struct Projection;

pub(crate) fn run() {
    run_session(campaign_session(false, 26));
}

pub(crate) fn run_generated(seed: u32) {
    run_session(generated_session(seed, false));
}

pub(crate) fn run_expanded(seed: u32) {
    run_session(expanded_session(seed, false));
}

pub(crate) fn run_knowledge(seed: u32) {
    run_session(knowledge_session(seed, false));
}

pub(crate) fn run_lod(seed: u32) {
    run_session(lod_session(seed, false));
}

pub(crate) fn run_production(seed: u32) {
    run_session(production_session(seed, false));
}

pub(crate) fn run_history(seed: u32) {
    run_session(history_session(seed, false));
}

pub(crate) fn run_requests(seed: u32) {
    run_session(requests_session(seed, false));
}

pub(crate) fn run_favors(seed: u32) {
    run_session(favors_session(seed, false));
}

pub(crate) fn run_governance(seed: u32) {
    run_session(governance_session(seed, false));
}

pub(crate) fn run_diplomacy(seed: u32) {
    run_session(diplomacy_session(seed, false));
}

pub(crate) fn run_circles(seed: u32) {
    run_session(circles_session(seed, false));
}

pub(crate) fn run_development(seed: u32) {
    run_session(development_session(seed, false));
}

pub(crate) fn run_narrative(seed: u32) {
    run_session(narrative_session(seed, false));
}

pub(crate) fn run_rumor(seed: u32) {
    run_session(rumor_session(seed, false));
}

pub(crate) fn run_communication(seed: u32) {
    run_session(communication_session(seed, false));
}

pub(crate) fn run_obligations(seed: u32) {
    run_session(obligations_session(seed, false));
}

pub(crate) fn run_cooperation(seed: u32) {
    run_session(cooperation_session(seed, false));
}

pub(crate) fn run_social_network(seed: u32) {
    run_session(social_network_session(seed, false));
}

pub(crate) fn run_encounters(seed: u32) {
    run_session(encounter_session(seed, false));
}

pub(crate) fn run_bonds(seed: u32) {
    run_session(bonds_session(seed, false));
}

fn run_session(session: Session) {
    App::new()
        .insert_resource(Game {
            session,
            target: None,
        })
        .insert_resource(ClearColor(Color::srgb(0.048, 0.062, 0.065)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: living_text(Locale::English, "living.pixel.window.title").into(),
                resolution: (900, 660).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (keyboard, draw).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

const COMMANDS: &[(KeyCode, &str)] = &[
    (KeyCode::KeyW, "w"),
    (KeyCode::KeyA, "a"),
    (KeyCode::KeyS, "s"),
    (KeyCode::KeyD, "d"),
    (KeyCode::Space, "wait"),
    (KeyCode::KeyB, "guard"),
    (KeyCode::KeyG, "gather"),
    (KeyCode::KeyH, "deliver"),
    (KeyCode::KeyM, "practice"),
    (KeyCode::KeyU, "buy"),
    (KeyCode::KeyI, "sell"),
    (KeyCode::KeyJ, "join"),
    (KeyCode::KeyK, "brew"),
    (KeyCode::KeyL, "remedy"),
    (KeyCode::KeyY, "duty"),
    (KeyCode::KeyZ, "study"),
    (KeyCode::KeyX, "center"),
    (KeyCode::KeyC, "accord"),
    (KeyCode::KeyV, "sponsor"),
    (KeyCode::Digit4, "pledge"),
    (KeyCode::Digit5, "aidlodge"),
    (KeyCode::Digit6, "renounce"),
    (KeyCode::Digit1, "background wanderer"),
    (KeyCode::Digit2, "background herbalist"),
    (KeyCode::Digit3, "background peddler"),
    (KeyCode::F5, "save"),
    (KeyCode::F9, "load"),
];

const TARGET_COMMANDS: &[(KeyCode, &str)] = &[
    (KeyCode::KeyE, "talk"),
    (KeyCode::KeyF, "attack"),
    (KeyCode::KeyP, "form"),
    (KeyCode::KeyN, "invite"),
    (KeyCode::KeyO, "pack"),
    (KeyCode::KeyR, "part"),
];

fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<Game>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        let command = if game.session.locale == Locale::English {
            "lang id"
        } else {
            "lang en"
        };
        game.session.input(command);
        if let Ok(mut window) = windows.single_mut() {
            window.title = living_text(game.session.locale, "living.pixel.window.title").to_owned();
        }
    } else if game.session.world.governance().is_some()
        && keys.just_pressed(KeyCode::F9)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        game.session.input("governancenext");
    } else if game.session.world.rumors().is_some()
        && keys.just_pressed(KeyCode::F2)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        game.session.input("rumorsnext");
    } else if game.session.world.encounters().is_some()
        && keys.just_pressed(KeyCode::F2)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        game.session.input("contacts 2");
    } else if keys.just_pressed(KeyCode::F2) && game.session.world.generated_region().is_some() {
        game.session.input("survey");
    } else if game.session.world.communication().is_some()
        && keys.just_pressed(KeyCode::F3)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        game.session.input("messages 2");
    } else if game.session.world.social_network().is_some()
        && keys.just_pressed(KeyCode::F3)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        game.session.input("sociallog 2");
    } else if keys.just_pressed(KeyCode::F3) && game.session.world.regional_news().is_some() {
        game.session.input("newsnext");
    } else if game.session.world.obligations().is_some()
        && keys.just_pressed(KeyCode::F4)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        game.session.input("obligations 2");
    } else if game.session.world.cooperation().is_some()
        && keys.just_pressed(KeyCode::F4)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        game.session.input("teams 2");
    } else if keys.just_pressed(KeyCode::F4) && game.session.world.production().is_some() {
        game.session.input("production");
    } else if keys.just_pressed(KeyCode::F6) && game.session.world.road_history().is_some() {
        game.session.input("encounters");
    } else if game.session.world.narrative().is_some()
        && keys.just_pressed(KeyCode::F10)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        game.session.input("storiesnext");
    } else if keys.just_pressed(KeyCode::F10) && game.session.world.road_history().is_some() {
        game.session.input("history");
    } else if keys.just_pressed(KeyCode::F7) && game.session.world.road_history().is_some() {
        game.session.input("recovercache");
    } else if keys.just_pressed(KeyCode::F8) && game.session.world.road_history().is_some() {
        game.session.input("entrustcache");
    } else if game.session.world.development().is_some()
        && keys.just_pressed(KeyCode::F11)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        game.session.input("profilesnext");
    } else if game.session.world.requests().is_some() && keys.just_pressed(KeyCode::F11) {
        game.session.input("quests");
    } else if game.session.world.diplomacy().is_some()
        && keys.just_pressed(KeyCode::F12)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        game.session.input("diplomacynext");
    } else if game.session.world.circles().is_some()
        && keys.just_pressed(KeyCode::F12)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        game.session.input("circlesnext");
    } else if game.session.world.requests().is_some() && keys.just_pressed(KeyCode::F12) {
        game.session.input("acceptquest");
    } else if game.session.world.requests().is_some() && keys.just_pressed(KeyCode::Digit7) {
        game.session.input("declinequest");
    } else if game.session.world.requests().is_some() && keys.just_pressed(KeyCode::Digit8) {
        game.session.input("abandonquest shrine");
    } else if game.session.world.requests().is_some() && keys.just_pressed(KeyCode::Digit9) {
        game.session.input("abandonquest market");
    } else if game.session.world.requests().is_some() && keys.just_pressed(KeyCode::Digit0) {
        game.session.input("questlog 2");
    } else if game.session.world.reciprocity().is_some()
        && keys.just_pressed(KeyCode::F1)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        let log = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        let mut lines = if !log {
            game.target
                .and_then(|a| game.session.known_bond_line(a))
                .map(|s| vec![s])
                .unwrap_or_else(|| game.session.bond_lines(false, 1))
        } else {
            game.session.bond_lines(true, 1)
        };
        lines.push(living_text(game.session.locale, "living.bond.pixel.controls").into());
        game.session.notice = lines.join("\n");
    } else if game.session.world.reciprocity().is_some()
        && [KeyCode::Comma, KeyCode::Slash, KeyCode::Semicolon]
            .into_iter()
            .any(|k| keys.just_pressed(k))
    {
        let command = if keys.just_pressed(KeyCode::Comma) {
            "requestaid"
        } else if keys.just_pressed(KeyCode::Slash) {
            "repayaid"
        } else {
            "waiveaid"
        };
        let local = nearby(&game.session);
        // Never silently choose an arbitrary debtor if multiple people are nearby.
        let target = game.target.filter(|a| local.contains(a)).or_else(|| {
            if local.len() == 1 {
                local.first().copied()
            } else {
                None
            }
        });
        if let Some(id) = target {
            game.session.input(&format!("{command} {}", id.get()));
        } else {
            game.session.notice = living_text(game.session.locale, "living.bond.select").into();
        }
    } else if game.session.world.favors().is_some() && keys.just_pressed(KeyCode::F1) {
        game.session.input(
            if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
                "favorlog 2"
            } else {
                "favors 2"
            },
        );
    } else if game.session.world.favors().is_some()
        && [
            KeyCode::KeyQ,
            KeyCode::Backspace,
            KeyCode::Delete,
            KeyCode::Period,
        ]
        .into_iter()
        .any(|k| keys.just_pressed(k))
    {
        let command = if keys.just_pressed(KeyCode::KeyQ) {
            "acceptfavor"
        } else if keys.just_pressed(KeyCode::Backspace) {
            "declinefavor"
        } else if keys.just_pressed(KeyCode::Delete) {
            "withdrawfavor"
        } else {
            "fundfavor"
        };
        // An explicit adjacent target wins; otherwise require a unique eligible known promise.
        let target = game
            .target
            .filter(|a| game.session.world.at_favor_actor(*a));
        if let Some(id) = game.session.favor_selection(command, target) {
            game.session.input(&format!("{command} {id}"));
        } else {
            game.session.notice =
                living_text(game.session.locale, "living.favor.no_selection").into();
        }
    } else if keys.just_pressed(KeyCode::KeyT) {
        let nearby = nearby(&game.session);
        let next = nearby
            .iter()
            .position(|&id| Some(id) == game.target)
            .map_or(0, |index| index + 1);
        game.target = nearby
            .get(next)
            .copied()
            .or_else(|| nearby.first().copied());
    } else if let Some((_, command)) = TARGET_COMMANDS
        .iter()
        .find(|(key, _)| keys.just_pressed(*key))
    {
        let nearby = nearby(&game.session);
        let target = game
            .target
            .filter(|id| nearby.contains(id))
            .or_else(|| nearby.first().copied());
        match target {
            Some(id) => {
                game.session.input(&format!("{command} {}", id.get()));
            }
            None => {
                game.session.input(command);
            }
        }
    } else if let Some((_, command)) = COMMANDS.iter().find(|(key, _)| keys.just_pressed(*key)) {
        game.session.input(command);
    }
}

fn nearby(session: &Session) -> Vec<ActorId> {
    let player = session.world.player().position;
    session
        .world
        .npcs()
        .filter(|npc| {
            npc.alive()
                && session.world.player_sees(npc.position)
                && npc.position.x.abs_diff(player.x) + npc.position.y.abs_diff(player.y) <= 1
        })
        .map(|npc| npc.id())
        .collect()
}

fn draw(mut commands: Commands, game: Res<Game>, previous: Query<Entity, With<Projection>>) {
    if !game.is_changed() {
        return;
    }
    for entity in &previous {
        commands.entity(entity).despawn();
    }
    let session = &game.session;
    let world = &session.world;
    let player = world.player();
    let scenario = world.scenario();
    let tr = |key| living_text(session.locale, key);
    let sites = Region::sites();
    let distance =
        |site: Position| player.position.x.abs_diff(site.x) + player.position.y.abs_diff(site.y);
    let near_hall = world.player_sees(sites.hall);
    let near_market = world.player_sees(sites.market);

    label(
        &mut commands,
        living_format(
            session.locale,
            "living.pixel.draw.template.scenario_name",
            &[("scenario_name", scenario.name)],
        ),
        257.0,
        27.0,
        INK,
    );
    let (hour, minute) = clock::clock_hhmm(clock::minute_of_day(world.calendar_tick()));
    let mut stats = living_format(
        session.locale,
        "living.pixel.draw.template.day_clock_day_number_world_calendar",
        &[
            (
                "clock_day_number_world_calendar_tick",
                &format!("{}", clock::day_number(world.calendar_tick())),
            ),
            ("hour", &format!("{:02}", hour)),
            ("minute", &format!("{:02}", minute)),
            ("player_hp", &format!("{}", player.hp)),
            ("player_herbs", &format!("{}", player.herbs)),
            (
                "world_breath_progress",
                &format!("{}", world.breath_progress()),
            ),
            ("breath_mastery", &format!("{}", BREATH_MASTERY)),
        ],
    );
    if let Some(region) = world.region() {
        stats.push_str(&living_format(
            session.locale,
            "living.pixel.draw.template.coins_region_coins_remedies_region_remedies",
            &[
                ("region_coins", &format!("{}", region.coins)),
                ("region_remedies", &format!("{}", region.remedies)),
            ],
        ));
    }
    if let Some(school) = world.school().filter(|school| school.form_learned) {
        stats.push_str(&living_format(
            session.locale,
            "living.pixel.draw.template.school_qi_max_qi",
            &[
                ("school_qi", &format!("{}", school.qi)),
                ("max_qi", &format!("{}", MAX_QI)),
            ],
        ));
    }
    label(&mut commands, stats, 219.0, 16.0, OCHRE);
    if let Some(map) = world.generated_region() {
        label(
            &mut commands,
            living_format(
                session.locale,
                if world.requests().is_some() {
                    "living.quest.pixel.hint"
                } else if world.road_history().is_some() {
                    "living.history.pixel.hint"
                } else if world.production().is_some() {
                    "living.production.pixel.hint"
                } else if world.regional_news().is_some() {
                    "living.regional.pixel.hint"
                } else {
                    "living.worldgen.pixel.hint"
                },
                &[("seed", &map.seed().to_string())],
            ),
            190.0,
            14.0,
            INK,
        );
    }

    if world.favors().is_some() {
        label(
            &mut commands,
            tr(if world.governance().is_some() {
                "living.governance.pixel.hint"
            } else if world.diplomacy().is_some() {
                "living.diplomacy.pixel.hint"
            } else if world.circles().is_some() {
                "living.circle.pixel.hint"
            } else if world.development().is_some() {
                "living.profile.pixel.hint"
            } else if world.narrative().is_some() {
                "living.story.pixel.hint"
            } else if world.rumors().is_some() {
                "living.rumor.pixel.hint"
            } else if world.communication().is_some() {
                "living.message.pixel.hint"
            } else if world.obligations().is_some() {
                "living.obligation.pixel.hint"
            } else if world.cooperation().is_some() {
                "living.team.pixel.hint"
            } else if world.social_network().is_some() {
                "living.network.pixel.hint"
            } else if world.encounters().is_some() {
                "living.contact.pixel.hint"
            } else if world.reciprocity().is_some() {
                "living.bond.pixel.hint"
            } else {
                "living.favor.pixel.hint"
            })
            .into(),
            172.0,
            12.0,
            OCHRE,
        );
    }

    let width = scenario.width.min(17);
    let left = player
        .position
        .x
        .saturating_sub(width / 2)
        .min(scenario.width - width);
    let height = scenario.height.min(9);
    let top = player
        .position
        .y
        .saturating_sub(height / 2)
        .min(scenario.height - height);
    for y in top..top + height {
        for x in left..left + width {
            let cell = Position { x, y };
            let symbol = session.tile_symbol(cell);
            let at = Vec2::new(
                ((x - left) as f32 - (width - 1) as f32 / 2.0) * SIDE,
                ((height - 1) as f32 / 2.0 - (y - top) as f32) * SIDE + 37.0,
            );
            let tile_color = match symbol {
                '?' => FOG,
                '#' => STONE,
                '~' => Color::srgb(0.12, 0.28, 0.43),
                '%' => Color::srgb(0.42, 0.29, 0.16),
                ':' => Color::srgb(0.34, 0.30, 0.21),
                ';' => Color::srgb(0.10, 0.22, 0.14),
                ',' => Color::srgb(0.28, 0.38, 0.23),
                _ if (x + y) % 2 == 0 => MOSS,
                _ => Color::srgb(0.21, 0.29, 0.24),
            };
            sprite(&mut commands, at, 0.0, Vec2::splat(SIDE - 1.0), tile_color);
            draw_icon(&mut commands, at, symbol);
        }
    }

    let event = world
        .company()
        .and_then(|company| company.history.last())
        .filter(|event| event.witnessed && event.at == world.tick());
    let local = if let Some(event) = event {
        session.company_line(event)
    } else if let Some(hint) = session.local_favor_hint() {
        hint
    } else if let Some(hint) = session.local_quest_hint() {
        hint
    } else if let Some(hint) = session.duty_hint() {
        hint.into()
    } else if let Some(hint) = session.frontier_hint() {
        hint
    } else if world.player_sees(scenario.shrine) {
        living_format(
            session.locale,
            "living.pixel.draw.template.shrine_herbs_world_supply_shrine_scenario",
            &[
                ("world_supply_shrine", &format!("{}", world.supply().shrine)),
                (
                    "scenario_shrine_target",
                    &format!("{}", scenario.shrine_target),
                ),
            ],
        )
    } else if world.region().is_some()
        && near_hall
        && (!near_market || distance(sites.hall) < distance(sites.market))
    {
        session.hall_hint()
    } else if let Some(region) = world.region().filter(|_| near_market) {
        if world.is_night() {
            tr("living.pixel.draw.willowford_market_opens_at_dawn").into()
        } else {
            let quote = world.market_quote().expect("regional market");
            living_format(
                session.locale,
                "living.pixel.draw.template.market_herbs_region_market_herbs_buy",
                &[
                    ("region_market_herbs", &format!("{}", region.market_herbs)),
                    ("quote_buy", &format!("{}", quote.buy)),
                    ("quote_sell", &format!("{}", quote.sell)),
                ],
            )
        }
    } else {
        tr("living.pixel.draw.willowford_lies_east_of_qinghe").into()
    };
    label(&mut commands, local, -132.0, 16.0, OCHRE);
    let message = if session.notice.is_empty() {
        world.latest_outcome().map_or_else(
            || tr("living.pixel.draw.choose_a_background_1_wanderer_2_herbalist").into(),
            |event| outcome_label(session.locale, world, event),
        )
    } else {
        session.notice.clone()
    };
    let mut message = message;
    if session.notice.is_empty()
        && let Some(line) = session.latest_quest_notice()
    {
        message.push('\n');
        message.push_str(&line);
    }
    if session.notice.is_empty()
        && let Some(line) = session.latest_favor_notice()
    {
        message = line;
    }
    if session.notice.is_empty()
        && world.latest_outcome() == Some(sealed_sim::living::Outcome::Waited)
        && let Some(line) = session.latest_aid_notice()
    {
        message = line;
    }
    let message_size = if world.regional_news().is_some() && message.contains('\n') {
        14.0
    } else {
        16.0
    };
    label(&mut commands, message, -171.0, message_size, INK);
    let nearby = nearby(session);
    let target = game
        .target
        .filter(|id| nearby.contains(id))
        .or_else(|| nearby.first().copied());
    if let Some(npc) = target.and_then(|id| world.npc(id)) {
        label(
            &mut commands,
            living_format(
                session.locale,
                "living.pixel.draw.template.next_target_npc_definition_name",
                &[("npc_definition_name", npc.definition.name)],
            ),
            -198.0,
            14.0,
            OCHRE,
        );
    }
    label(
        &mut commands,
        tr("living.pixel.draw.wasd_move_space_wait_e_talk_f").into(),
        -230.0,
        14.0,
        INK,
    );
    label(
        &mut commands,
        tr("living.pixel.draw.u_buy_i_sell_j_join_k").into(),
        -257.0,
        14.0,
        INK,
    );
    if world.company().is_some() {
        label(
            &mut commands,
            tr("living.pixel.draw.n_invite_o_pack_herb_r_part").into(),
            -304.0,
            14.0,
            INK,
        );
    }
    if world.school().is_some() {
        label(
            &mut commands,
            tr("living.pixel.draw.y_shrine_duty_z_study_x_center").into(),
            -280.0,
            14.0,
            OCHRE,
        );
    }
    if world.politics().is_some() {
        label(
            &mut commands,
            tr("living.pixel.draw.4_pledge_5_supply_stonebank_6_renounce").into(),
            -318.0,
            13.0,
            OCHRE,
        );
    }
}

fn draw_icon(commands: &mut Commands, at: Vec2, symbol: char) {
    let (art, accent, light) = match symbol {
        '#' => (
            &["11111", "12121", "11111", "21112"][..],
            STONE,
            Color::srgb(0.49, 0.54, 0.55),
        ),
        '*' | '&' => (
            &["00100", "20102", "02220", "00100"][..],
            Color::srgb(0.42, 0.69, 0.36),
            INK,
        ),
        '+' | '=' | '$' | '^' => (
            &["00100", "01110", "11111", "12021", "12221"][..],
            Color::srgb(0.53, 0.30, 0.21),
            OCHRE,
        ),
        '!' => (
            &["01110", "12221", "11111", "10001", "11111"][..],
            Color::srgb(0.62, 0.34, 0.18),
            OCHRE,
        ),
        '@' => (
            &["01110", "01210", "11111", "01210", "01010"][..],
            OCHRE,
            INK,
        ),
        'x' => (
            &["10001", "01010", "00100", "01010", "10001"][..],
            Color::srgb(0.56, 0.31, 0.27),
            INK,
        ),
        '~' => (
            &["01010", "10101", "01010", "10101"][..],
            Color::srgb(0.21, 0.49, 0.65),
            Color::srgb(0.48, 0.71, 0.80),
        ),
        '%' => (
            &["11111", "22222", "11111", "22222"][..],
            OCHRE,
            Color::srgb(0.52, 0.38, 0.24),
        ),
        ';' => (
            &["00100", "01110", "11111", "00100", "00100"][..],
            Color::srgb(0.34, 0.55, 0.28),
            OCHRE,
        ),
        '?' | '.' | ':' | ',' => return,
        _ => (
            &["01110", "01210", "01110", "11111", "01010"][..],
            Color::srgb(0.42, 0.62, 0.68),
            INK,
        ),
    };
    for (row, pixels) in art.iter().enumerate() {
        for (column, shade) in pixels.bytes().enumerate() {
            let color = match shade {
                b'1' => accent,
                b'2' => light,
                _ => continue,
            };
            let pixel = at + Vec2::new((column as f32 - 2.0) * 4.0, (2.0 - row as f32) * 4.0);
            sprite(commands, pixel, 1.0, Vec2::splat(4.0), color);
        }
    }
}

fn sprite(commands: &mut Commands, at: Vec2, depth: f32, size: Vec2, color: Color) {
    commands.spawn((
        Projection,
        Sprite::from_color(color, size),
        Transform::from_translation(at.extend(depth)),
    ));
}

fn label(commands: &mut Commands, value: String, y: f32, size: f32, color: Color) {
    commands.spawn((
        Projection,
        Text2d::new(value),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(color),
        Transform::from_xyz(0.0, y, 3.0),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_tab_refreshes_real_projection_without_advancing_gameplay() {
        let mut app = App::new();
        app.insert_resource(Game {
            session: campaign_session(false, 26),
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((
            Window {
                title: living_text(Locale::English, "living.pixel.window.title").into(),
                ..default()
            },
            PrimaryWindow,
        ));
        app.update();
        let before = app.world().resource::<Game>().session.world.checkpoint();
        let snapshot = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|x| x.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(snapshot(&mut app).contains("Day"));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();
        assert_eq!(
            app.world().resource::<Game>().session.locale,
            Locale::Indonesian
        );
        assert!(snapshot(&mut app).contains("Hari"));
        assert!(snapshot(&mut app).contains("Spasi tunggu"));
        let mut windows = app
            .world_mut()
            .query_filtered::<&Window, With<PrimaryWindow>>();
        assert_eq!(
            windows.single(app.world()).unwrap().title,
            living_text(Locale::Indonesian, "living.pixel.window.title")
        );
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }

    #[test]
    fn generated_pixel_projection_and_survey_preserve_world_and_hide_unseen_terrain() {
        let mut app = App::new();
        app.insert_resource(Game {
            session: generated_session(7, false),
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let before = app.world().resource::<Game>().session.world.checkpoint();
        let session = &app.world().resource::<Game>().session;
        assert_eq!(session.tile_symbol(Position { x: 16, y: 4 }), '?');
        assert_eq!(session.world.checkpoint_revision(), 27);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F2);
        app.update();
        assert!(
            app.world()
                .resource::<Game>()
                .session
                .notice
                .contains("elevation")
        );
        let text = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(text(&mut app).contains("F2: survey"));
        assert!(text(&mut app).contains("elevation"));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();
        assert!(text(&mut app).contains("F2: amati"));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::F2);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F2);
        app.update();
        assert!(text(&mut app).contains("elevasi"));
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );

        // The renderer uses the same generated tile map when the explicitly
        // requested developer projection removes fog.
        let session = generated_session(7, true);
        let map = session.world.generated_region().unwrap();
        for y in 0..9 {
            for x in 12..21 {
                let p = Position { x, y };
                assert_eq!(session.tile_symbol(p), map.tile(p).unwrap().terrain.glyph());
            }
        }
    }

    #[test]
    fn expanded_pixel_camera_and_frontier_inspection_follow_the_canonical_world() {
        use sealed_sim::living::{Direction, Intent, expansion::DEPOT};
        use std::collections::{BTreeSet, VecDeque};
        let mut session = expanded_session(7, false);
        assert_eq!(session.tile_symbol(DEPOT), '?');
        assert!(session.frontier_hint().is_none());
        let target = Position { x: 46, y: 8 };
        // Independent BFS over the actual public collision map.
        for _ in 0..400 {
            let start = session.world.player().position;
            if start == target {
                break;
            }
            let mut queue = VecDeque::from([(start, None)]);
            let mut seen = BTreeSet::from([start]);
            let mut choice = None;
            while let Some((at, first)) = queue.pop_front() {
                if at == target {
                    choice = first;
                    break;
                }
                for (direction, x, y) in [
                    (Direction::North, Some(at.x), at.y.checked_sub(1)),
                    (Direction::East, at.x.checked_add(1), Some(at.y)),
                    (Direction::South, Some(at.x), at.y.checked_add(1)),
                    (Direction::West, at.x.checked_sub(1), Some(at.y)),
                ] {
                    let (Some(x), Some(y)) = (x, y) else {
                        continue;
                    };
                    let p = Position { x, y };
                    if !session.world.walkable(p)
                        || session.world.npcs().any(|n| n.alive() && n.position == p)
                        || !seen.insert(p)
                    {
                        continue;
                    }
                    queue.push_back((p, first.or(Some(direction))));
                }
            }
            let intent = choice.map_or(Intent::Wait, Intent::Move);
            session.world.step(session.world.tick(), intent).unwrap();
        }
        assert_eq!(session.world.player().position, target);
        assert!(session.frontier_hint().unwrap().contains("Pineford depot:"));
        let before = session.world.checkpoint();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let text = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(text(&mut app).contains("Pineford depot:"));
        // Exactly 17 x 9 ground tiles; the extra vertical rows scroll rather
        // than overflowing the fixed-size scene and controls.
        let mut sprites = app.world_mut().query::<(&Sprite, &Transform)>();
        let ground = sprites
            .iter(app.world())
            .filter(|(_, t)| t.translation.z == 0.0)
            .count();
        assert_eq!(ground, 17 * 9);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F2);
        app.update();
        assert!(text(&mut app).contains("elevation"));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();
        assert!(text(&mut app).contains("Depot Pineford:"));
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }

    #[test]
    fn regional_pixel_news_pages_are_localized_read_only_and_never_reveal_remote_history() {
        use sealed_sim::living::{Direction, Intent, PLAYER, regional_news::Site};
        use std::collections::{BTreeSet, VecDeque};
        let mut session = knowledge_session(7, false);
        assert!(session.regional_news_lines()[0].contains("No regional knowledge"));
        let target = session.world.scenario().shrine;
        for _ in 0..100 {
            let start = session.world.player().position;
            if start.x.abs_diff(target.x) + start.y.abs_diff(target.y) <= 1 {
                break;
            }
            let mut q = VecDeque::from([(start, None)]);
            let mut seen = BTreeSet::from([start]);
            let mut choice = None;
            while let Some((at, first)) = q.pop_front() {
                if at.x.abs_diff(target.x) + at.y.abs_diff(target.y) <= 1 {
                    choice = first;
                    break;
                }
                for (d, x, y) in [
                    (Direction::North, Some(at.x), at.y.checked_sub(1)),
                    (Direction::East, at.x.checked_add(1), Some(at.y)),
                    (Direction::South, Some(at.x), at.y.checked_add(1)),
                    (Direction::West, at.x.checked_sub(1), Some(at.y)),
                ] {
                    let (Some(x), Some(y)) = (x, y) else {
                        continue;
                    };
                    let p = Position { x, y };
                    if !session.world.walkable(p)
                        || session.world.npcs().any(|n| n.alive() && n.position == p)
                        || !seen.insert(p)
                    {
                        continue;
                    }
                    q.push_back((p, first.or(Some(d))));
                }
            }
            session
                .world
                .step(
                    session.world.tick(),
                    choice.map_or(Intent::Wait, Intent::Move),
                )
                .unwrap();
        }
        assert!(
            session
                .world
                .regional_news()
                .unwrap()
                .stock(PLAYER, Site::QingheShrine)
                .is_some()
        );
        for _ in 0..150 {
            session
                .world
                .step(session.world.tick(), Intent::Wait)
                .unwrap();
        }
        let before = session.world.checkpoint();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let text = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(text(&mut app).contains("F3: known regional news"));
        let press = |app: &mut App, key| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.clear();
            keys.release(key);
            keys.press(key);
            app.update();
        };
        let len = app
            .world()
            .resource::<Game>()
            .session
            .regional_news_lines()
            .len();
        for _ in 0..len {
            press(&mut app, KeyCode::F3);
            let value = text(&mut app);
            assert!(
                !value.contains("developer record")
                    && !value.contains("Pineford depot:")
                    && !value.contains("Willowford market:")
            );
        }
        press(&mut app, KeyCode::F3);
        assert!(text(&mut app).contains("Qinghe shrine:"));
        press(&mut app, KeyCode::Tab);
        assert!(text(&mut app).contains("F3: kabar regional"));
        for _ in 0..len {
            press(&mut app, KeyCode::F3);
        }
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }

    #[test]
    fn lod_projection_rebuilds_are_canonical_read_only() {
        use sealed_sim::living::Intent;
        let mut session = lod_session(7, false);
        for _ in 0..180 {
            session
                .world
                .step(session.world.tick(), Intent::Wait)
                .unwrap();
        }
        let before = session.world.checkpoint();
        let mut reference = session.world.clone();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .add_systems(Update, draw);
        app.update();
        fn projections(app: &mut App) -> Vec<Entity> {
            let mut q = app.world_mut().query_filtered::<Entity, With<Projection>>();
            q.iter(app.world()).collect()
        }
        let expected = projections(&mut app).len();
        assert!(expected > 153);
        for i in 0..1000 {
            // Alternate explicit teardown and in-place projection rebuild.
            if i % 3 == 0 {
                for entity in projections(&mut app) {
                    app.world_mut().despawn(entity);
                }
            }
            let lang = if i % 2 == 0 { "lang id" } else { "lang en" };
            app.world_mut().resource_mut::<Game>().session.input(lang);
            app.update();
            assert_eq!(projections(&mut app).len(), expected);
            assert_eq!(
                app.world().resource::<Game>().session.world.checkpoint(),
                before
            );
        }
        for _ in 0..32 {
            reference.step(reference.tick(), Intent::Wait).unwrap();
            let mut game = app.world_mut().resource_mut::<Game>();
            game.session.input("wait");
            assert_eq!(game.session.world.checkpoint(), reference.checkpoint());
            app.update();
        }
    }

    #[test]
    fn production_pixel_f4_obeys_local_knowledge_and_preserves_checkpoint() {
        use sealed_sim::living::{Direction, Intent};
        use std::collections::{BTreeSet, VecDeque};
        let mut session = production_session(7, false);
        let target = session.world.scenario().patch;
        // Reach the actual site through canonical commands, not a teleport.
        for _ in 0..200 {
            let start = session.world.player().position;
            if start.x.abs_diff(target.x) + start.y.abs_diff(target.y) <= 1 {
                break;
            }
            let mut q = VecDeque::from([(start, None)]);
            let mut seen = BTreeSet::from([start]);
            let mut choice = None;
            while let Some((at, first)) = q.pop_front() {
                if at.x.abs_diff(target.x) + at.y.abs_diff(target.y) <= 1 {
                    choice = first;
                    break;
                }
                for (d, x, y) in [
                    (Direction::North, Some(at.x), at.y.checked_sub(1)),
                    (Direction::East, at.x.checked_add(1), Some(at.y)),
                    (Direction::South, Some(at.x), at.y.checked_add(1)),
                    (Direction::West, at.x.checked_sub(1), Some(at.y)),
                ] {
                    let (Some(x), Some(y)) = (x, y) else {
                        continue;
                    };
                    let p = Position { x, y };
                    if !session.world.walkable(p)
                        || session.world.npcs().any(|n| n.alive() && n.position == p)
                        || !seen.insert(p)
                    {
                        continue;
                    }
                    q.push_back((p, first.or(Some(d))));
                }
            }
            session
                .world
                .step(
                    session.world.tick(),
                    choice.map_or(Intent::Wait, Intent::Move),
                )
                .unwrap();
        }
        assert_eq!(session.world.local_production().len(), 1);
        let before = session.world.checkpoint();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let text = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let press = |app: &mut App, key| {
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.clear();
                keys.release(key);
                keys.press(key);
            }
            app.update();
        };
        assert!(text(&mut app).contains("F4: local production"));
        press(&mut app, KeyCode::F4);
        let en = text(&mut app);
        assert!(en.contains("Qinghe herb patch:") && en.contains("Growth check in"));
        assert!(!en.contains("Pineford grove:") && !en.contains("OrderDispatch"));
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::F4);
        let id = text(&mut app);
        assert!(id.contains("Ladang herba Qinghe:") && id.contains("Pemeriksaan tumbuh dalam"));
        assert!(!id.contains("Rumpun Pineford:") && !id.contains("OrderDispatch"));
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }

    mod road_trace {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/c45_trace.rs"
        ));
    }

    #[test]
    fn history_pixel_keys_are_read_only_until_player_selects_a_physical_outcome() {
        use sealed_sim::living::Intent;
        for entrust in [false, true] {
            let (mut expected, _) = road_trace::trace(entrust);
            let mut session = history_session(expected.region().unwrap().seed, false);
            session.world = expected.clone();
            let before = expected.checkpoint();
            let mut app = App::new();
            app.insert_resource(Game {
                session,
                target: None,
            })
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, (keyboard, draw).chain());
            app.world_mut().spawn((Window::default(), PrimaryWindow));
            app.update();
            let text = |app: &mut App| {
                let mut q = app.world_mut().query::<&Text2d>();
                q.iter(app.world())
                    .map(|t| t.0.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let press = |app: &mut App, key| {
                {
                    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                    keys.clear();
                    keys.release(key);
                    keys.press(key);
                }
                app.update();
            };
            assert!(text(&mut app).contains("F6-8 cache/take/entrust"));
            press(&mut app, KeyCode::F6);
            assert!(text(&mut app).contains("Cache #1, Willowford to Qinghe"));
            press(&mut app, KeyCode::F10);
            assert!(text(&mut app).contains("days before the campaign"));
            press(&mut app, KeyCode::Tab);
            press(&mut app, KeyCode::F6);
            assert!(text(&mut app).contains("Simpanan #1, Willowford ke Qinghe"));
            assert_eq!(
                app.world().resource::<Game>().session.world.checkpoint(),
                before
            );
            press(&mut app, if entrust { KeyCode::F8 } else { KeyCode::F7 });
            expected
                .step(
                    expected.tick(),
                    if entrust {
                        Intent::EntrustCache(1)
                    } else {
                        Intent::RecoverCache(1)
                    },
                )
                .unwrap();
            assert_eq!(
                app.world().resource::<Game>().session.world.checkpoint(),
                expected.checkpoint()
            );
            press(&mut app, KeyCode::F10);
            let memory = text(&mut app);
            assert!(memory.contains(if entrust {
                "diserahkan kepada Bao Jun"
            } else {
                "mengambil simpanan #1"
            }));
            assert_eq!(
                app.world().resource::<Game>().session.world.checkpoint(),
                expected.checkpoint()
            );
        }
    }

    #[test]
    fn request_pixel_controls_replay_all_six_actual_choices_without_read_side_effects() {
        use sealed_sim::living::{World, requests::Status};
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/living/c46_branch_trace_v1.json"
        ))
        .unwrap();
        for row in fixture["rows"].as_array().unwrap() {
            let seed = row["seed"].as_u64().unwrap().try_into().unwrap();
            let site = row["site"].as_str().unwrap();
            let choice = row["choice"].as_str().unwrap();
            let mut session = requests_session(seed, false);
            let commands = row["commands"].as_array().unwrap();
            let first = commands
                .iter()
                .position(|c| {
                    c.as_str()
                        .unwrap()
                        .ends_with(&format!("quest {}", row["id"]))
                })
                .unwrap();
            for c in &commands[..first] {
                assert!(session.input(c.as_str().unwrap()));
            }
            let before = session.world.checkpoint();
            let mut reference = World::restore(&before).unwrap();
            let mut app = App::new();
            app.insert_resource(Game {
                session,
                target: None,
            })
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, (keyboard, draw).chain());
            app.world_mut().spawn((Window::default(), PrimaryWindow));
            app.update();
            let press = |app: &mut App, key| {
                {
                    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                    keys.clear();
                    keys.release(key);
                    keys.press(key);
                }
                app.update();
            };
            let text = |app: &mut App| {
                let mut q = app.world_mut().query::<&Text2d>();
                q.iter(app.world())
                    .map(|t| t.0.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            assert!(text(&mut app).contains("F11 quests"));
            press(&mut app, KeyCode::F11);
            assert!(text(&mut app).contains("Offered"));
            press(&mut app, KeyCode::Tab);
            press(&mut app, KeyCode::F11);
            assert!(text(&mut app).contains("Ditawarkan"));
            assert_eq!(
                app.world().resource::<Game>().session.world.checkpoint(),
                before
            );
            for c in &commands[first..] {
                let command = c.as_str().unwrap();
                let id = row["id"].as_u64().unwrap().try_into().unwrap();
                use sealed_sim::living::{Direction, Intent};
                let (key, intent) = match command {
                    "deliver" => (KeyCode::KeyH, Intent::Deliver),
                    "sell" => (KeyCode::KeyI, Intent::SellHerb),
                    "north" => (KeyCode::KeyW, Intent::Move(Direction::North)),
                    "south" => (KeyCode::KeyS, Intent::Move(Direction::South)),
                    "east" => (KeyCode::KeyD, Intent::Move(Direction::East)),
                    "west" => (KeyCode::KeyA, Intent::Move(Direction::West)),
                    "wait" => (KeyCode::Space, Intent::Wait),
                    s if s.starts_with("acceptquest") => (KeyCode::F12, Intent::AcceptRequest(id)),
                    s if s.starts_with("declinequest") => {
                        (KeyCode::Digit7, Intent::DeclineRequest(id))
                    }
                    s if s.starts_with("abandonquest") => (
                        if site == "shrine" {
                            KeyCode::Digit8
                        } else {
                            KeyCode::Digit9
                        },
                        Intent::AbandonRequest(id),
                    ),
                    s => panic!("unexpected command {s}"),
                };
                reference.step(reference.tick(), intent).unwrap();
                press(&mut app, key);
                assert_eq!(
                    app.world().resource::<Game>().session.world.checkpoint(),
                    reference.checkpoint()
                );
            }
            press(&mut app, KeyCode::Digit0);
            let memory = text(&mut app);
            assert!(memory.contains(match choice {
                "complete" => "Selesai",
                "decline" => "Ditolak",
                _ => "Dibatalkan",
            }));
            if choice == "complete" && site == "market" {
                assert!(memory.contains("bonus 1 dibayar"));
            }
            assert_eq!(
                app.world().resource::<Game>().session.world.checkpoint(),
                reference.checkpoint()
            );
            assert_eq!(
                reference.request_receipts().last().unwrap().request.status,
                match choice {
                    "complete" => Status::Completed,
                    "decline" => Status::Declined,
                    _ => Status::Abandoned,
                }
            );
        }
    }

    #[test]
    fn personal_favor_pixel_keys_replay_five_real_branches_and_inspection_is_read_only() {
        use sealed_sim::living::{ActorId, Intent, PLAYER, World, favors::Status};
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/living/c47_branch_trace_v1.json"
        ))
        .unwrap();
        for row in fixture["rows"].as_array().unwrap() {
            let seed = row["seed"].as_u64().unwrap().try_into().unwrap();
            let actor = ActorId::new(row["actor"].as_u64().unwrap().try_into().unwrap());
            let id = row["id"].as_u64().unwrap().try_into().unwrap();
            let mut session = favors_session(seed, false);
            let commands = row["commands"].as_array().unwrap();
            let first = row["offer_tick"].as_u64().unwrap() as usize;
            for c in &commands[..first] {
                assert!(session.input(c.as_str().unwrap()));
            }
            let before = session.world.checkpoint();
            let mut reference = World::restore(&before).unwrap();
            let mut app = App::new();
            app.insert_resource(Game {
                session,
                target: Some(actor),
            })
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, (keyboard, draw).chain());
            app.world_mut().spawn((Window::default(), PrimaryWindow));
            app.update();
            let press = |app: &mut App, key, shift: bool| {
                {
                    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                    keys.clear();
                    keys.release(KeyCode::ShiftLeft);
                    if shift {
                        keys.press(KeyCode::ShiftLeft);
                    }
                    keys.release(key);
                    keys.press(key);
                }
                app.update();
            };
            let text = |app: &mut App| {
                let mut q = app.world_mut().query::<&Text2d>();
                q.iter(app.world())
                    .map(|t| t.0.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            assert!(text(&mut app).contains("F1 favors"));
            press(&mut app, KeyCode::F1, false);
            assert!(text(&mut app).contains("1 herb for travel kit"));
            press(&mut app, KeyCode::F1, true);
            assert!(text(&mut app).contains("No remembered personal outcomes."));
            press(&mut app, KeyCode::Tab, false);
            press(&mut app, KeyCode::F1, false);
            assert!(text(&mut app).contains("1 herba bekal"));
            assert_eq!(
                app.world().resource::<Game>().session.world.checkpoint(),
                before
            );
            for c in &commands[first..] {
                let command = c.as_str().unwrap();
                let (key, intent) = match command {
                    "pack 22" => (KeyCode::KeyO, Intent::Provision(actor)),
                    "part 22" => (KeyCode::KeyR, Intent::Part(actor)),
                    s if s.starts_with("acceptfavor") => (KeyCode::KeyQ, Intent::AcceptFavor(id)),
                    s if s.starts_with("declinefavor") => {
                        (KeyCode::Backspace, Intent::DeclineFavor(id))
                    }
                    s if s.starts_with("withdrawfavor") => {
                        (KeyCode::Delete, Intent::WithdrawFavor(id))
                    }
                    s => panic!("unexpected command {s}"),
                };
                reference.step(reference.tick(), intent).unwrap();
                press(&mut app, key, false);
                assert_eq!(
                    app.world().resource::<Game>().session.world.checkpoint(),
                    reference.checkpoint()
                );
            }
            press(&mut app, KeyCode::F1, true);
            let memory = text(&mut app);
            let status = match row["choice"].as_str().unwrap() {
                "fulfill" => ("Bantuan diberikan", Status::Fulfilled),
                "decline" => ("Ditolak", Status::Declined),
                "withdraw" => ("Dibatalkan", Status::Withdrawn),
                _ => ("Keadaan berubah", Status::Changed),
            };
            assert!(memory.contains(status.0), "{memory}");
            assert_eq!(
                app.world().resource::<Game>().session.world.checkpoint(),
                reference.checkpoint()
            );
            assert_eq!(
                reference.favor_receipts().last().unwrap().favor.status,
                status.1
            );
            assert_eq!(
                i64::from(reference.social().relationship(actor, PLAYER).regard),
                row["regard"].as_i64().unwrap()
            );
        }
    }

    #[test]
    fn reciprocal_keys_match_seven_canonical_endpoints_and_inspection_is_read_only() {
        use sealed_sim::living::{ActorId, Intent, PLAYER, World};
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/living/c48_branch_trace_v1.json"
        ))
        .unwrap();
        for row in fixture["rows"].as_array().unwrap() {
            let seed = row["seed"].as_u64().unwrap().try_into().unwrap();
            let actor = ActorId::new(row["actor"].as_u64().unwrap().try_into().unwrap());
            let mut session = bonds_session(seed, false);
            let commands = row["commands"].as_array().unwrap();
            let last = commands.len() - 1;
            for c in &commands[..last] {
                assert!(session.input(c.as_str().unwrap()));
            }
            let before = session.world.checkpoint();
            let mut reference = World::restore(&before).unwrap();
            let mut app = App::new();
            app.insert_resource(Game {
                session,
                target: Some(actor),
            })
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, (keyboard, draw).chain());
            app.world_mut().spawn((Window::default(), PrimaryWindow));
            app.update();
            let press = |app: &mut App, key, ctrl: bool, shift: bool| {
                {
                    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                    keys.clear();
                    keys.release(KeyCode::ControlLeft);
                    keys.release(KeyCode::ShiftLeft);
                    if ctrl {
                        keys.press(KeyCode::ControlLeft);
                    }
                    if shift {
                        keys.press(KeyCode::ShiftLeft);
                    }
                    keys.release(key);
                    keys.press(key);
                }
                app.update();
            };
            let text = |app: &mut App| {
                let mut q = app.world_mut().query::<&Text2d>();
                q.iter(app.world())
                    .map(|t| t.0.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            assert!(text(&mut app).contains("Ctrl+F1 bonds"));
            press(&mut app, KeyCode::F1, true, false);
            assert!(text(&mut app).contains("Ctrl+Shift+F1 aid log"));
            press(&mut app, KeyCode::F1, true, true);
            assert!(text(&mut app).contains("Ctrl+Shift+F1 aid log"));
            press(&mut app, KeyCode::Tab, false, false);
            press(&mut app, KeyCode::F1, true, false);
            assert!(text(&mut app).contains("Ctrl+Shift+F1 riwayat"));
            assert_eq!(
                app.world().resource::<Game>().session.world.checkpoint(),
                before
            );
            let (key, intent) = match commands[last].as_str().unwrap() {
                "requestaid 22" => (KeyCode::Comma, Intent::RequestAid(actor)),
                "waiveaid 22" => (KeyCode::Semicolon, Intent::WaiveAid(actor)),
                "repayaid 1" => (KeyCode::Slash, Intent::RepayAid(actor)),
                "part 22" => (KeyCode::KeyR, Intent::Part(actor)),
                "pack 22" => (KeyCode::KeyO, Intent::Provision(actor)),
                c => panic!("unexpected final command {c}"),
            };
            reference.step(reference.tick(), intent).unwrap();
            press(&mut app, key, false, false);
            assert_eq!(
                app.world().resource::<Game>().session.world.checkpoint(),
                reference.checkpoint()
            );
            press(&mut app, KeyCode::F1, true, false);
            let memory = text(&mut app);
            let balance = row["balance"].as_i64().unwrap();
            if balance == 0 {
                assert!(memory.contains("Tidak ada utang budi tersisa."), "{memory}");
            } else {
                assert!(
                    memory.contains(&format!("berutang budi {balance} kepadamu")),
                    "{memory}"
                );
            }
            press(&mut app, KeyCode::F1, true, true);
            assert_eq!(
                app.world().resource::<Game>().session.world.checkpoint(),
                reference.checkpoint()
            );
            assert_eq!(
                i64::from(reference.reciprocity().unwrap().net(PLAYER, actor)),
                balance
            );
        }
    }

    #[test]
    fn encounter_keyboard_projection_is_read_only_and_bilingual() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/living/c48_branch_trace_v1.json"
        ))
        .unwrap();
        let mut session = encounter_session(0, false);
        for c in fixture["rows"][0]["commands"]
            .as_array()
            .unwrap()
            .iter()
            .take(30)
        {
            assert!(session.input(c.as_str().unwrap()));
        }
        let before = session.world.checkpoint();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let press = |app: &mut App, key, ctrl: bool| {
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.clear();
                keys.release(KeyCode::ControlLeft);
                if ctrl {
                    keys.press(KeyCode::ControlLeft);
                }
                keys.release(key);
                keys.press(key);
            }
            app.update();
        };
        let text = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(text(&mut app).contains("Ctrl+F2 contacts"));
        press(&mut app, KeyCode::F2, true);
        assert!(text(&mut app).contains("last met t30"));
        press(&mut app, KeyCode::Tab, false);
        press(&mut app, KeyCode::F2, true);
        assert!(text(&mut app).contains("terakhir ditemui t30"));
        press(&mut app, KeyCode::F2, false);
        assert!(!text(&mut app).contains("terakhir ditemui t30"));
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }

    #[test]
    fn witnessed_social_keyboard_projection_and_locale_are_read_only() {
        let mut session = social_network_session(0, false);
        for _ in 0..120 {
            assert!(session.input("wait"));
        }
        assert!(session.world.witnessed_social().next().is_some());
        let before = session.world.checkpoint();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let press = |app: &mut App, key, ctrl: bool| {
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.clear();
                keys.release(KeyCode::ControlLeft);
                if ctrl {
                    keys.press(KeyCode::ControlLeft);
                }
                keys.release(key);
                keys.press(key);
            }
            app.update();
        };
        let text = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(text(&mut app).contains("Ctrl+F3 social log"));
        press(&mut app, KeyCode::F3, true);
        assert!(text(&mut app).contains("greeted at"));
        press(&mut app, KeyCode::Tab, false);
        press(&mut app, KeyCode::F3, true);
        assert!(text(&mut app).contains("bertegur sapa di"));
        press(&mut app, KeyCode::F3, false);
        assert!(!text(&mut app).contains("bertegur sapa di"));
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }

    #[test]
    fn collective_receipt_keyboard_projection_and_locale_are_read_only() {
        let mut session = cooperation_session(3, false);
        for _ in 0..120 {
            assert!(session.input("wait"));
        }
        assert!(session.world.known_cooperation().next().is_none());
        for cmd in ["east", "east", "south"] {
            assert!(session.input(cmd));
        }
        assert!(session.world.known_cooperation().next().is_some());
        let before = session.world.checkpoint();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let press = |app: &mut App, key, ctrl: bool| {
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.clear();
                keys.release(KeyCode::ControlLeft);
                if ctrl {
                    keys.press(KeyCode::ControlLeft);
                }
                keys.release(key);
                keys.press(key);
            }
            app.update();
        };
        let text = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(text(&mut app).contains("Ctrl+F4 teams"));
        press(&mut app, KeyCode::F4, true);
        assert!(text(&mut app).contains("Receipt t37 #0:"));
        press(&mut app, KeyCode::Tab, false);
        press(&mut app, KeyCode::F4, true);
        assert!(text(&mut app).contains("Catatan t37 #0:"));
        press(&mut app, KeyCode::F4, false);
        assert!(!text(&mut app).contains("Catatan t37 #0:"));
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }

    #[test]
    fn c52_promise_keyboard_projection_keeps_old_team_controls_and_private_state() {
        let mut session = obligations_session(3, false);
        for _ in 0..120 {
            assert!(session.input("wait"));
        }
        for cmd in ["east", "east", "south"] {
            assert!(session.input(cmd));
        }
        assert!(session.world.known_cooperation().next().is_some());
        assert!(session.world.witnessed_obligations().next().is_none());
        let before = session.world.checkpoint();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let press = |app: &mut App, key, ctrl: bool, shift: bool| {
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.clear();
                keys.release(KeyCode::ControlLeft);
                keys.release(KeyCode::ShiftLeft);
                if ctrl {
                    keys.press(KeyCode::ControlLeft);
                }
                if shift {
                    keys.press(KeyCode::ShiftLeft);
                }
                keys.release(key);
                keys.press(key);
            }
            app.update();
        };
        let text = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(text(&mut app).contains("Ctrl+Shift+F4 promises"));
        press(&mut app, KeyCode::F4, true, true);
        assert!(text(&mut app).contains("No witnessed promise acknowledgements yet."));
        press(&mut app, KeyCode::F4, true, false);
        assert!(text(&mut app).contains("Receipt t37 #0:"));
        press(&mut app, KeyCode::Tab, false, false);
        press(&mut app, KeyCode::F4, true, true);
        assert!(text(&mut app).contains("Belum ada konfirmasi janji yang disaksikan."));
        press(&mut app, KeyCode::F4, true, false);
        assert!(text(&mut app).contains("Catatan t37 #0:"));
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }

    #[test]
    fn c53_natural_witness_keyboard_projection_is_read_only_and_bilingual() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/living/c53_communication_golden_v1.json"
        ))
        .unwrap();
        let row = fixture["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["witnessed"].as_bool() == Some(true))
            .unwrap();
        let mut session = communication_session(row["seed"].as_u64().unwrap() as u32, false);
        for cmd in row["commands"].as_array().unwrap() {
            assert!(session.input(cmd.as_str().unwrap()));
        }
        assert!(session.world.witnessed_messages().next().is_some());
        let before = session.world.checkpoint();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let press = |app: &mut App, key, ctrl: bool, shift: bool| {
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.clear();
                keys.release(KeyCode::ControlLeft);
                keys.release(KeyCode::ShiftLeft);
                if ctrl {
                    keys.press(KeyCode::ControlLeft);
                }
                if shift {
                    keys.press(KeyCode::ShiftLeft);
                }
                keys.release(key);
                keys.press(key);
            }
            app.update();
        };
        let text = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(text(&mut app).contains("Ctrl+Shift+F3 messages"));
        press(&mut app, KeyCode::F3, true, true);
        assert!(text(&mut app).contains("t71 #0: Bao Jun told Mei Fen: delivery complete."));
        press(&mut app, KeyCode::F3, true, false);
        assert!(!text(&mut app).contains("Bao Jun told Mei Fen"));
        press(&mut app, KeyCode::Tab, false, false);
        press(&mut app, KeyCode::F3, true, true);
        assert!(
            text(&mut app).contains("t71 #0: Bao Jun memberi tahu Mei Fen: pengiriman selesai.")
        );
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }

    #[test]
    fn c54_natural_rumor_pages_are_bilingual_read_only_and_local() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/living/c54_rumors_golden_v1.json"
        ))
        .unwrap();
        let row = fixture["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"].as_str() == Some("heard-account"))
            .unwrap();
        let mut session = rumor_session(row["seed"].as_u64().unwrap() as u32, false);
        for cmd in row["commands"].as_array().unwrap() {
            assert!(session.input(cmd.as_str().unwrap()));
        }
        assert!(session.rumor_lines()[0].contains("source Shen Wei"));
        let before = session.world.checkpoint();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let press = |app: &mut App, key, ctrl: bool, shift: bool| {
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.clear();
                keys.release(KeyCode::ControlLeft);
                keys.release(KeyCode::ShiftLeft);
                if ctrl {
                    keys.press(KeyCode::ControlLeft);
                }
                if shift {
                    keys.press(KeyCode::ShiftLeft);
                }
                keys.release(key);
                keys.press(key);
            }
            app.update();
        };
        let text = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(text(&mut app).contains("Ctrl+Shift+F2 rumors"));
        press(&mut app, KeyCode::F2, true, true);
        assert!(text(&mut app).contains("1/2") && text(&mut app).contains("saw scarce at t21"));
        press(&mut app, KeyCode::F2, true, true);
        assert!(
            text(&mut app).contains("2/2")
                && text(&mut app)
                    .contains("source Shen Wei, via Shen Wei, confidence 67/100, hops 1")
        );
        press(&mut app, KeyCode::F2, true, false);
        assert!(!text(&mut app).contains("confidence 67/100"));
        press(&mut app, KeyCode::Tab, false, false);
        press(&mut app, KeyCode::F2, true, true);
        press(&mut app, KeyCode::F2, true, true);
        assert!(
            text(&mut app)
                .contains("sumber Shen Wei, melalui Shen Wei, keyakinan 67/100, perantara 1")
        );
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }

    #[test]
    fn c55_natural_story_pages_are_bilingual_read_only_and_local() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/living/c55_narrative_golden_v1.json"
        ))
        .unwrap();
        let row = fixture["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"].as_str() == Some("checked-inquiry"))
            .unwrap();
        let mut session = narrative_session(row["seed"].as_u64().unwrap() as u32, false);
        for cmd in row["commands"].as_array().unwrap() {
            assert!(session.input(cmd.as_str().unwrap()));
        }
        let before = session.world.checkpoint();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let press = |app: &mut App, key, ctrl: bool| {
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.clear();
                keys.release(KeyCode::ControlLeft);
                if ctrl {
                    keys.press(KeyCode::ControlLeft);
                }
                keys.release(key);
                keys.press(key);
            }
            app.update();
        };
        let text = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(text(&mut app).contains("Ctrl+F10 stories"));
        press(&mut app, KeyCode::F10, true);
        assert!(text(&mut app).contains("Inquiry #0: heard at t23"));
        press(&mut app, KeyCode::F10, true);
        assert!(text(&mut app).contains("Inquiry #0: checked Qinghe shrine at t24: scarce."));
        press(&mut app, KeyCode::F10, false);
        assert!(!text(&mut app).contains("Inquiry #0: checked"));
        press(&mut app, KeyCode::Tab, false);
        press(&mut app, KeyCode::F10, true);
        assert!(text(&mut app).contains("Penyelidikan #0: dengar t23"));
        press(&mut app, KeyCode::F10, true);
        assert!(text(&mut app).contains("Penyelidikan #0: memeriksa"));
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }

    #[test]
    fn c56_natural_profile_pages_are_bilingual_read_only_and_dated() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/living/c56_development_golden_v1.json"
        ))
        .unwrap();
        let row = fixture["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"].as_str() == Some("updated-partner-disclosure"))
            .unwrap();
        let mut session = development_session(row["seed"].as_u64().unwrap() as u32, false);
        for cmd in row["commands"].as_array().unwrap() {
            assert!(session.input(cmd.as_str().unwrap()));
        }
        let before = session.world.checkpoint();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let press = |app: &mut App, key, ctrl: bool| {
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.clear();
                keys.release(KeyCode::ControlLeft);
                if ctrl {
                    keys.press(KeyCode::ControlLeft);
                }
                keys.release(key);
                keys.press(key);
            }
            app.update();
        };
        let text = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(text(&mut app).contains("Ctrl+F11 profiles"));
        press(&mut app, KeyCode::F11, true);
        assert!(
            text(&mut app)
                .contains("Mei Fen, said at t213: working toward dependable partnerships.")
        );
        press(&mut app, KeyCode::F11, false);
        assert!(!text(&mut app).contains("Mei Fen, said at t213"));
        press(&mut app, KeyCode::Tab, false);
        press(&mut app, KeyCode::F11, true);
        assert!(
            text(&mut app)
                .contains("Mei Fen, menyampaikan pada t213: mengupayakan kerja sama yang andal.")
        );
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }

    #[test]
    fn c57_natural_circle_pages_are_bilingual_read_only_and_dated() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/living/c57_circles_golden_v1.json"
        ))
        .unwrap();
        let row = fixture["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"].as_str() == Some("member-disclosure"))
            .unwrap();
        let mut session = circles_session(row["seed"].as_u64().unwrap() as u32, false);
        for cmd in row["commands"].as_array().unwrap() {
            assert!(session.input(cmd.as_str().unwrap()));
        }
        let before = session.world.checkpoint();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let press = |app: &mut App, key, ctrl: bool| {
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.clear();
                keys.release(KeyCode::ControlLeft);
                if ctrl {
                    keys.press(KeyCode::ControlLeft);
                }
                keys.release(key);
                keys.press(key);
            }
            app.update();
        };
        let text = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(text(&mut app).contains("Ctrl+F12 circles"));
        press(&mut app, KeyCode::F12, true);
        assert!(
            text(&mut app).contains("Mei Fen, said at t213: coordinator of a freight association")
        );
        press(&mut app, KeyCode::F12, true);
        assert!(text(&mut app).contains("t71: Mei Fen and Bao Jun formed a freight association"));
        press(&mut app, KeyCode::Tab, false);
        // Cycle the same dated pages without invoking the F12 quest action.
        for _ in 0..2 {
            press(&mut app, KeyCode::F12, true);
        }
        assert!(text(&mut app).contains("Mei Fen, menyampaikan pada t213: koordinator"));
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }

    #[test]
    fn c59_board_shortcut_is_read_only_and_bilingual() {
        let session = governance_session(3, false);
        let before = session.world.checkpoint();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let press = |app: &mut App, key, ctrl: bool| {
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.clear();
                keys.release(KeyCode::ControlLeft);
                if ctrl {
                    keys.press(KeyCode::ControlLeft);
                }
                keys.release(key);
                keys.press(key);
            }
            app.update();
        };
        press(&mut app, KeyCode::F9, true);
        let expected = app.world().resource::<Game>().session.governance_lines()[0].clone();
        assert!(
            app.world()
                .resource::<Game>()
                .session
                .notice
                .contains(expected.as_str())
        );
        press(&mut app, KeyCode::Tab, false);
        let pages = app
            .world()
            .resource::<Game>()
            .session
            .governance_lines()
            .len();
        for _ in 0..pages {
            press(&mut app, KeyCode::F9, true);
        }
        let expected = app.world().resource::<Game>().session.governance_lines();
        assert!(expected.iter().any(|s| {
            app.world()
                .resource::<Game>()
                .session
                .notice
                .contains(s.as_str())
        }));
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }

    #[test]
    fn c58_diplomatic_pages_bilingual_read_only_and_separate_from_circles() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/living/c58_diplomacy_golden_v1.json"
        ))
        .unwrap();
        let row = fixture["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"].as_str() == Some("member-disclosure"))
            .unwrap();
        let mut session = diplomacy_session(row["seed"].as_u64().unwrap() as u32, false);
        for cmd in row["commands"].as_array().unwrap() {
            assert!(session.input(cmd.as_str().unwrap()));
        }
        let before = session.world.checkpoint();
        let pages = session.diplomacy_lines().len();
        let mut app = App::new();
        app.insert_resource(Game {
            session,
            target: None,
        })
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, (keyboard, draw).chain());
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        let press = |app: &mut App, key, ctrl: bool, shift: bool| {
            {
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.clear();
                keys.release(KeyCode::ControlLeft);
                keys.release(KeyCode::ShiftLeft);
                if ctrl {
                    keys.press(KeyCode::ControlLeft);
                }
                if shift {
                    keys.press(KeyCode::ShiftLeft);
                }
                keys.release(key);
                keys.press(key);
            }
            app.update();
        };
        let text = |app: &mut App| {
            let mut q = app.world_mut().query::<&Text2d>();
            q.iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(text(&mut app).contains("Ctrl+Shift+F12 diplomacy"));
        press(&mut app, KeyCode::F12, true, true);
        assert!(text(&mut app).contains(row["player_lines_en"][0].as_str().unwrap()));
        press(&mut app, KeyCode::F12, true, false);
        assert!(!text(&mut app).contains(row["player_lines_en"][0].as_str().unwrap()));
        press(&mut app, KeyCode::Tab, false, false);
        for _ in 0..pages {
            press(&mut app, KeyCode::F12, true, true);
        }
        assert!(text(&mut app).contains(row["player_lines_id"][0].as_str().unwrap()));
        assert_eq!(
            app.world().resource::<Game>().session.world.checkpoint(),
            before
        );
    }
}
