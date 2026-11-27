//! Desktop projection of the regional campaign. Keyboard actions enter the
//! same session as terminal commands, and rendering never advances the clock.

use bevy::{prelude::*, window::PrimaryWindow};
use sealed_sim::living::{ActorId, BREATH_MASTERY, MAX_QI, Position, Region};

use crate::{
    clock,
    living_ascii::{
        Session, campaign_session, expanded_session, generated_session, knowledge_session,
        lod_session, outcome_label, production_session,
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
    } else if keys.just_pressed(KeyCode::F2) && game.session.world.generated_region().is_some() {
        game.session.input("survey");
    } else if keys.just_pressed(KeyCode::F3) && game.session.world.regional_news().is_some() {
        game.session.input("newsnext");
    } else if keys.just_pressed(KeyCode::F4) && game.session.world.production().is_some() {
        game.session.input("production");
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
                if world.production().is_some() {
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
}
