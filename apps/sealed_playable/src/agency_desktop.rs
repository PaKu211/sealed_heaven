//! Disposable C05 view/input. No position, inventory or NPC/task authority.
use bevy::{
    input::{
        ButtonState,
        keyboard::KeyboardInput,
        mouse::{MouseScrollUnit, MouseWheel},
    },
    prelude::*,
    render::{
        renderer::RenderAdapterInfo,
        view::screenshot::{Screenshot, save_to_disk},
    },
    text::LineBreak,
    ui::FocusPolicy,
    window::PrimaryWindow,
};
use sealed_playable::{
    agency::{Action, AgencyPlugin, Input, Session, drive_frame},
    clock::{self, PlannedBlock},
    hex,
    locale::{Catalog, Locale},
    supply::Phase,
    view::View,
};
use sealed_sim::player5::{
    Direction, Event, HEIGHT, Intent, Outcome, Position, Rejection, Target, WIDTH,
};
use std::{collections::VecDeque, fmt::Write};

const CELL: f32 = 48.0;
const PANEL: f32 = 440.0;
const TOP: f32 = 64.0;
const FOOTER: f32 = 94.0;
const INK: Color = Color::srgb(0.92, 0.94, 0.89);

#[derive(Resource)]
struct Strings(Catalog);
#[derive(Resource, Default)]
struct Performance {
    frames: u64,
    intervals_ms: VecDeque<f64>,
}
#[derive(Component)]
struct MainCamera;
#[derive(Component)]
struct PlayerMarker;
#[derive(Component)]
struct PlayerIdentity;
#[derive(Component)]
struct NpcMarker(u32);
#[derive(Component)]
struct TargetRing;
#[derive(Component)]
struct ScrollPanel;
#[derive(Component, Clone, Copy, Debug)]
enum Control {
    Toggle,
    Step,
    Save,
    Load,
    Reset,
    Language,
    Interact,
    Follow,
}
#[derive(Component)]
enum Label {
    Key(&'static str),
    Toggle,
    Summary,
    Body,
    Help,
    Context,
    Marker(Target),
}

pub fn run() {
    let strings = Catalog::agency().expect("validated C05 localization");
    App::new()
        .insert_resource(Strings(strings))
        .init_resource::<Performance>()
        .insert_resource(ClearColor(Color::srgb(0.035, 0.055, 0.065)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Sealed Heaven — C0.7 Qinghe".into(),
                resolution: (1440, 960).into(),
                resize_constraints: bevy::window::WindowResizeConstraints {
                    min_width: 1280.0,
                    min_height: 760.0,
                    ..default()
                },
                ..default()
            }),
            ..default()
        }))
        .add_plugins(AgencyPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (buttons, keyboard, select_target)
                .chain()
                .before(drive_frame),
        )
        .add_systems(Update, (follow_camera, scroll, metrics, day_night))
        .add_systems(Update, (paint, capture).chain().after(drive_frame))
        .run();
}

fn at(position: Position) -> Vec2 {
    Vec2::new(
        (position.x as f32 - 5.5) * CELL,
        (4.0 - position.y as f32) * CELL,
    )
}

fn npc_at(id: u32, position: Position) -> Vec2 {
    // Deliberate presentation-only offsets. Canonical occupancy is cell-based.
    let slot = id.saturating_sub(1);
    at(position)
        + Vec2::new(
            (slot % 5) as f32 * 8.0 - 16.0,
            (slot / 5) as f32 * 8.0 - 12.0,
        )
}

fn targets() -> Vec<Target> {
    [Target::Npc(1), Target::HerbPatch, Target::Shrine]
        .into_iter()
        .chain((2..=20).map(Target::Npc))
        .collect()
}

fn target_name(target: Target, catalog: &Catalog, locale: Locale) -> String {
    catalog.text(
        locale,
        match target {
            Target::Npc(1) => "c05.healer",
            Target::HerbPatch => "c05.patch",
            Target::Shrine => "c05.shrine",
            Target::Npc(_) => "inspector.npc",
        },
    ) + &match target {
        Target::Npc(id) if id != 1 => format!(" {id}"),
        _ => String::new(),
    }
}

fn context_intent(target: Target) -> Intent {
    match target {
        Target::Npc(id) => Intent::Talk(id),
        Target::HerbPatch => Intent::Gather,
        Target::Shrine => Intent::Deliver,
    }
}

fn spawn_player(commands: &mut Commands, position: Position) {
    commands
        .spawn((
            PlayerMarker,
            Sprite::from_color(Color::srgb(0.96, 0.78, 0.22), Vec2::new(26.0, 34.0)),
            Transform::from_translation(at(position).extend(8.0)),
        ))
        .with_children(|player| {
            player.spawn((
                PlayerIdentity,
                Label::Key("c06.you"),
                Text2d::new("You"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::BLACK),
                Transform::from_xyz(0.0, 0.0, 1.0),
            ));
        });
}

fn setup(mut commands: Commands, session: Res<Session>, mut fonts: ResMut<Assets<Font>>) {
    fonts
        .insert(
            bevy::asset::AssetId::default(),
            Font::try_from_bytes(include_bytes!("../assets/fonts/NotoSans-Regular.ttf").to_vec())
                .expect("bundled font"),
        )
        .expect("default font handle");
    commands.spawn((
        Camera2d,
        MainCamera,
        Transform::from_xyz(PANEL / 2.0, 0.0, 0.0),
    ));
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let pos = Position { x, y };
            let lane = session.world().is_walkable(pos) && y == 4;
            let color = if !session.world().is_walkable(pos) {
                Color::srgb(0.12, 0.13, 0.15)
            } else if x == 6 && y == 4 {
                Color::srgb(0.55, 0.42, 0.22)
            } else if lane {
                Color::srgb(0.36, 0.32, 0.22)
            } else {
                Color::srgb(if (x + y) % 2 == 0 { 0.16 } else { 0.2 }, 0.34, 0.22)
            };
            commands.spawn((
                Sprite::from_color(color, Vec2::splat(CELL - 2.0)),
                Transform::from_translation(at(pos).extend(0.0)),
            ));
        }
    }
    for (target, color, mark) in [
        (Target::HerbPatch, Color::srgb(0.28, 0.85, 0.43), "*"),
        (Target::Shrine, Color::srgb(0.76, 0.39, 0.34), "+"),
    ] {
        let p = session
            .world()
            .target_position(target)
            .expect("authored object");
        commands.spawn((
            Sprite::from_color(color, Vec2::splat(30.0)),
            Transform::from_translation(at(p).extend(3.0)),
        ));
        commands.spawn((
            Text2d::new(mark),
            TextFont {
                font_size: 23.0,
                ..default()
            },
            TextColor(Color::BLACK),
            Transform::from_translation(at(p).extend(4.0)),
        ));
        commands.spawn((
            Text2d::default(),
            Label::Marker(target),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(INK),
            Transform::from_translation((at(p) + Vec2::new(0.0, -31.0)).extend(7.0)),
        ));
    }
    for (key, position) in [
        ("c06.yard", Position { x: 3, y: 3 }),
        ("c06.homes", Position { x: 9, y: 5 }),
        ("c06.edge", Position { x: 10, y: 7 }),
    ] {
        commands.spawn((
            Text2d::default(),
            Label::Key(key),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(INK),
            Transform::from_translation((at(position) + Vec2::new(0.0, 28.0)).extend(7.0)),
        ));
    }
    for id in 1..=20 {
        commands.spawn((
            NpcMarker(id),
            Sprite::from_color(
                if id == 1 {
                    Color::srgb(0.6, 0.85, 1.0)
                } else {
                    Color::srgb(0.35, 0.61, 0.81)
                },
                Vec2::splat(if id == 1 { 18.0 } else { 12.0 }),
            ),
            Transform::default(),
            Visibility::Hidden,
        ));
    }
    spawn_player(&mut commands, session.world().player_position());
    commands.spawn((
        TargetRing,
        Sprite::from_color(Color::srgba(0.9, 0.9, 1.0, 0.3), Vec2::splat(44.0)),
        Transform::default(),
    ));
    commands.spawn((
        Text2d::default(),
        Label::Marker(Target::Npc(1)),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(INK),
        Transform::from_xyz(-130.0, 95.0, 9.0),
    ));
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                height: px(TOP),
                padding: UiRect::all(px(10)),
                column_gap: px(8),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.07, 0.1, 0.12)),
        ))
        .with_children(|bar| {
            bar.spawn((
                Text::default(),
                Label::Key("c06.title"),
                TextFont {
                    font_size: 19.0,
                    ..default()
                },
                TextColor(INK),
                Node {
                    width: px(260),
                    ..default()
                },
            ));
            for (control, label) in [
                (Control::Toggle, Label::Toggle),
                (Control::Step, Label::Key("controls.step")),
                (Control::Save, Label::Key("controls.save")),
                (Control::Load, Label::Key("controls.load")),
                (Control::Reset, Label::Key("controls.reset")),
                (Control::Language, Label::Key("controls.language")),
                (Control::Follow, Label::Key("c05.follow")),
            ] {
                bar.spawn((
                    Button,
                    control,
                    Node {
                        padding: UiRect::axes(px(10), px(7)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.23, 0.26)),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::default(),
                        label,
                        // The label sits above the button. Pass lets the click
                        // reach the Button so mouse and keyboard share one action.
                        FocusPolicy::Pass,
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(INK),
                    ));
                });
            }
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(0),
                top: px(TOP),
                bottom: px(FOOTER),
                width: px(PANEL),
                padding: UiRect::all(px(16)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                ..default()
            },
            BackgroundColor(Color::srgb(0.065, 0.085, 0.1)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::default(),
                Label::Summary,
                TextFont {
                    font_size: 17.0,
                    ..default()
                },
                TextColor(INK),
            ));
            panel
                .spawn((
                    Button,
                    Control::Interact,
                    Node {
                        padding: UiRect::all(px(12)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.17, 0.32, 0.3)),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::default(),
                        Label::Context,
                        FocusPolicy::Pass,
                        TextFont {
                            font_size: 15.0,
                            ..default()
                        },
                        TextColor(INK),
                    ));
                });
            panel
                .spawn((
                    ScrollPanel,
                    ScrollPosition::default(),
                    Node {
                        flex_grow: 1.0,
                        min_height: px(0),
                        width: percent(100),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                ))
                .with_children(|scroll| {
                    scroll.spawn((
                        Text::default(),
                        Label::Body,
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(INK),
                        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                        Node {
                            width: percent(100),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                });
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                right: px(0),
                height: px(FOOTER),
                padding: UiRect::all(px(14)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.07, 0.1, 0.12)),
        ))
        .with_children(|f| {
            f.spawn((
                Text::default(),
                Label::Help,
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(INK),
            ));
        });
}

fn language(locale: Locale) -> Locale {
    if locale == Locale::English {
        Locale::Indonesian
    } else {
        Locale::English
    }
}

/// Effective control state for an ordered frame, never a gameplay-state copy.
struct PendingControls {
    paused: bool,
    selected: Target,
    locale: Locale,
}
impl PendingControls {
    fn new(session: &Session, actions: &[Action]) -> Self {
        let mut state = Self {
            paused: session.paused,
            selected: session.selected,
            locale: session.locale,
        };
        for action in actions {
            state.observe(action);
        }
        state
    }
    fn observe(&mut self, action: &Action) {
        match action {
            Action::Pause | Action::Load | Action::Reset => self.paused = true,
            Action::Resume => self.paused = false,
            Action::Select(target) => self.selected = *target,
            Action::Language(locale) => self.locale = *locale,
            _ => {}
        }
    }
    fn toggle(&self) -> Action {
        if self.paused {
            Action::Resume
        } else {
            Action::Pause
        }
    }
}

fn buttons(
    buttons: Query<(&Interaction, &Control), Changed<Interaction>>,
    session: Res<Session>,
    mut input: ResMut<Input>,
) {
    let mut pending = PendingControls::new(&session, &input.actions);
    for (interaction, control) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let action = match control {
            Control::Toggle => pending.toggle(),
            Control::Step => Action::Step,
            Control::Save => Action::Save,
            Control::Load => Action::Load,
            Control::Reset => Action::Reset,
            Control::Language => Action::Language(language(pending.locale)),
            Control::Interact => Action::Intent(context_intent(pending.selected)),
            Control::Follow => continue,
        };
        pending.observe(&action);
        input.actions.push(action);
    }
}

fn keyboard(
    mut keys: MessageReader<KeyboardInput>,
    time: Res<Time<Real>>,
    held: Res<ButtonInput<KeyCode>>,
    session: Res<Session>,
    mut input: ResMut<Input>,
    mut repeat: Local<f64>,
) {
    input.elapsed_ns = time.delta().as_nanos();
    let mut pending = PendingControls::new(&session, &input.actions);
    let movement = |key| match key {
        KeyCode::KeyW | KeyCode::ArrowUp => Some(Direction::North),
        KeyCode::KeyD | KeyCode::ArrowRight => Some(Direction::East),
        KeyCode::KeyS | KeyCode::ArrowDown => Some(Direction::South),
        KeyCode::KeyA | KeyCode::ArrowLeft => Some(Direction::West),
        _ => None,
    };
    let mut new_move = false;
    for event in keys.read() {
        if event.state != ButtonState::Pressed || event.repeat {
            continue;
        }
        let action = if let Some(direction) = movement(event.key_code) {
            *repeat = 0.0;
            new_move = true;
            Some(Action::Intent(Intent::Move(direction)))
        } else {
            match event.key_code {
                KeyCode::Space => Some(pending.toggle()),
                KeyCode::KeyN => Some(Action::Step),
                KeyCode::F5 => Some(Action::Save),
                KeyCode::F9 => Some(Action::Load),
                KeyCode::KeyR => Some(Action::Reset),
                KeyCode::KeyL => Some(Action::Language(language(pending.locale))),
                KeyCode::KeyE => Some(Action::Intent(context_intent(pending.selected))),
                KeyCode::Tab => {
                    let all = targets();
                    let index = all.iter().position(|t| *t == pending.selected).unwrap_or(0);
                    Some(Action::Select(all[(index + 1) % all.len()]))
                }
                _ => None,
            }
        };
        if let Some(action) = action {
            pending.observe(&action);
            input.actions.push(action);
        }
    }
    *repeat += time.delta_secs_f64();
    if !new_move && *repeat >= 0.5 && session.queued() == 0 && !pending.paused {
        for key in [
            KeyCode::KeyW,
            KeyCode::ArrowUp,
            KeyCode::KeyD,
            KeyCode::ArrowRight,
            KeyCode::KeyS,
            KeyCode::ArrowDown,
            KeyCode::KeyA,
            KeyCode::ArrowLeft,
        ] {
            if held.pressed(key) {
                input
                    .actions
                    .push(Action::Intent(Intent::Move(movement(key).unwrap())));
                *repeat = 0.0;
                break;
            }
        }
    }
}

fn select_target(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    session: Res<Session>,
    mut input: ResMut<Input>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if cursor.x >= window.width() - PANEL || cursor.y < TOP || cursor.y > window.height() - FOOTER {
        return;
    }
    let (camera, global) = *camera;
    let Ok(world) = camera.viewport_to_world_2d(global, cursor) else {
        return;
    };
    let nearest = targets()
        .into_iter()
        .filter_map(|target| {
            let p = session.world().target_position(target)?;
            let point = match target {
                Target::Npc(id) => npc_at(id, p),
                _ => at(p),
            };
            Some((target, point.distance_squared(world)))
        })
        .filter(|(_, distance)| *distance < 28.0 * 28.0)
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((target, _)) = nearest {
        input.actions.push(Action::Select(target));
    }
}

fn follow_camera(
    session: Res<Session>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut camera: Single<&mut Transform, With<MainCamera>>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Query<(&Interaction, &Control), Changed<Interaction>>,
) {
    // Clamped follow keeps the whole compact map visible while centering on the
    // player within its safe range. UI/camera positions never enter checkpoints.
    let p = at(session.world().player_position());
    let half = ((window.width() - PANEL) / 2.0 - (WIDTH as f32 * CELL) / 2.0).max(0.0);
    let desired = p.x.clamp(-half, half) + PANEL / 2.0;
    camera.translation.x = desired;
    camera.translation.y = 0.0;
    if keys.just_pressed(KeyCode::KeyC)
        || buttons
            .iter()
            .any(|(i, c)| *i == Interaction::Pressed && matches!(c, Control::Follow))
    {
        camera.translation.x = PANEL / 2.0;
    }
}

fn scroll(
    mut wheel: MessageReader<MouseWheel>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut panel: Single<&mut ScrollPosition, With<ScrollPanel>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let over = window
        .cursor_position()
        .is_some_and(|p| p.x > window.width() - PANEL);
    for event in wheel.read() {
        if over {
            panel.y = (panel.y
                - event.y
                    * if event.unit == MouseScrollUnit::Line {
                        34.0
                    } else {
                        1.0
                    })
            .max(0.0);
        }
    }
    if keys.just_pressed(KeyCode::PageDown) {
        panel.y += 220.0;
    }
    if keys.just_pressed(KeyCode::PageUp) {
        panel.y = (panel.y - 220.0).max(0.0);
    }
    if keys.just_pressed(KeyCode::Home) {
        panel.y = 0.0;
    }
    if keys.just_pressed(KeyCode::End) {
        panel.y = 10000.0;
    }
}

fn rejection_key(rejection: Rejection) -> &'static str {
    match rejection {
        Rejection::OutOfBounds | Rejection::Terrain => "c05.r_terrain",
        Rejection::Occupied => "c05.r_occupied",
        Rejection::OutOfRange => "c05.r_out_of_range",
        Rejection::UnknownTarget => "c05.r_npc",
        Rejection::RequestNotAccepted => "c05.r_request",
        Rejection::Empty => "c05.r_empty",
        Rejection::AlreadyCompleted => "c05.r_complete",
        Rejection::Overflow => "c05.r_overflow",
    }
}

fn event_text(event: &Event, cat: &Catalog, locale: Locale) -> String {
    let label = match event.outcome {
        Outcome::Moved => cat.text(locale, "c05.moved"),
        Outcome::Requested => cat.text(locale, "c05.requested"),
        Outcome::Gathered => cat.text(locale, "c05.gathered"),
        Outcome::Delivered => cat.text(locale, "c05.delivered_ok"),
        Outcome::Rejected(reason) => format!(
            "{}: {}",
            cat.text(locale, "c05.rejected"),
            cat.text(locale, rejection_key(reason))
        ),
    };
    format!("#{} · t{} · {label}", event.id, event.tick)
}

fn body_text(session: &Session, cat: &Catalog) -> String {
    let world = session.world();
    let locale = session.locale;
    let t = |key| cat.text(locale, key);
    let mut text = format!(
        "{}\n{}: {}\n{}: {}\n{}: {}\n\n{}\n{}\n\n{}\n{}\n",
        t("c05.inventory"),
        t("c05.herbs"),
        world.player_herbs(),
        t("c05.source"),
        world.source_herbs(),
        t("c05.delivered"),
        world.delivered_herbs(),
        if world.completed() {
            t("c05.complete")
        } else if world.requested() {
            t("c05.request")
        } else {
            t("c05.unmet")
        },
        t(&session.status),
        t("c05.target"),
        target_name(session.selected, cat, locale)
    );
    let _ = writeln!(
        text,
        "{}",
        t(if world.in_range(session.selected) {
            "c05.range"
        } else {
            "c05.far"
        })
    );
    if let Some(p) = world.target_position(session.selected) {
        let _ = writeln!(text, "{}: ({},{})", t("c05.cell"), p.x, p.y);
    }
    if let Target::Npc(id) = session.selected
        && let Ok(view) = View::from_sim(world.npcs())
        && let Some(actor) = view.actors.iter().find(|a| a.id == id)
    {
        let goal = view
            .tasks
            .iter()
            .find(|task| task.id == actor.task)
            .map(|task| task.goal);
        let goal_text = t(match goal {
            Some(1) => "c06.goal.1",
            Some(2) => "c06.goal.2",
            Some(3) => "c06.goal.3",
            Some(4) => "c06.goal.4",
            Some(5) => "c06.goal.5",
            _ => "c06.no_task",
        });
        let _ = writeln!(
            text,
            "{}: {:?}\n{}: {}\n{}: {}",
            t("inspector.needs"),
            actor.needs,
            t("inspector.task"),
            actor.task,
            t("c06.goal"),
            goal_text
        );
    }
    let minute = clock::minute_of_day(world.tick());
    let (hour, mins) = clock::clock_hhmm(minute);
    let _ = writeln!(
        text,
        "\n{}: {:02}:{:02} · {} {}\n{}\n{}: {}\n{}",
        t("c06.clock"),
        hour,
        mins,
        t("c06.day"),
        clock::day_number(world.tick()),
        t(if clock::is_night(minute) {
            "c06.night"
        } else {
            "c06.daylight"
        }),
        t("c06.schedule"),
        t(block_key(clock::planned_block(minute))),
        t("c06.schedule_note")
    );
    if let Ok(supply) = session.supply().status() {
        let _ = writeln!(
            text,
            "\n{}\n{}\n{}: {}\n{}: {}\n{}: {} / {}\n{}: {}\n{}: {}\n{}: {}",
            t("c07.supply"),
            t("c07.note"),
            t("c07.demand"),
            supply.demand,
            t("c07.worker"),
            supply.worker,
            t("c07.progress"),
            supply.deliveries,
            supply.demand,
            t("c07.carrier"),
            supply.carrier,
            t("c07.shrine"),
            supply.shrine,
            t("c07.total"),
            supply.total
        );
        let _ = writeln!(text, "{}", t(phase_key(supply.phase)));
        let _ = writeln!(
            text,
            "{}: {}",
            t("c06.goal"),
            supply
                .goal
                .map(|goal| t(goal_key(goal)))
                .unwrap_or_else(|| t("c06.no_task"))
        );
    }
    let _ = writeln!(
        text,
        "\n{}: {} / {}\n{}: {}\n",
        t("c05.npc_alive"),
        world.npcs().tick(),
        world.npcs().next_event_id(),
        t("c05.queue"),
        session.queued()
    );
    let _ = writeln!(text, "{}", t("c05.pending"));
    for event in session.recent.iter().rev().take(6) {
        let _ = writeln!(text, "{}", event_text(event, cat, locale));
    }
    if !session.diagnostic.is_empty() {
        let _ = writeln!(text, "\n{}", session.diagnostic);
    }
    let hash = hex(&world.hash());
    let _ = write!(
        text,
        "\n{}\n{}\n\n{}\n{}",
        t("c05.saved_prefix"),
        hash,
        session.save_path.display(),
        t("help.save")
    );
    text
}

#[allow(clippy::type_complexity)]
fn paint(
    session: Res<Session>,
    strings: Res<Strings>,
    mut texts: Query<(&Label, Option<&mut Text>, Option<&mut Text2d>)>,
    mut player: Single<
        &mut Transform,
        (With<PlayerMarker>, Without<NpcMarker>, Without<TargetRing>),
    >,
    mut npcs: Query<
        (&NpcMarker, &mut Transform, &mut Visibility, &mut Sprite),
        (Without<PlayerMarker>, Without<TargetRing>),
    >,
    mut ring: Single<
        (&mut Transform, &mut Visibility),
        (With<TargetRing>, Without<PlayerMarker>, Without<NpcMarker>),
    >,
    mut stamp: Local<String>,
) {
    let world = session.world();
    player.translation = at(world.player_position()).extend(8.0);
    let facts = View::from_sim(world.npcs()).ok();
    for (marker, mut transform, mut visible, mut sprite) in &mut npcs {
        if let Some(pos) = world.npc_position(marker.0) {
            transform.translation =
                npc_at(marker.0, pos).extend(if marker.0 == 1 { 6.0 } else { 5.0 });
            *visible = Visibility::Visible;
            let goal = facts.as_ref().and_then(|view| {
                let actor = view.actors.iter().find(|actor| actor.id == marker.0)?;
                view.tasks
                    .iter()
                    .find(|task| task.id == actor.task)
                    .map(|task| task.goal)
            });
            sprite.color = goal_color(goal.unwrap_or(0));
        } else {
            *visible = Visibility::Hidden;
        }
    }
    if let Some(pos) = world.target_position(session.selected) {
        ring.0.translation = at(pos).extend(1.0);
        *ring.1 = Visibility::Visible;
    } else {
        *ring.1 = Visibility::Hidden;
    }
    let state = format!(
        "{}|{}|{:?}|{:?}|{}|{}|{}",
        world.tick(),
        session.paused,
        session.locale,
        session.selected,
        session.queued(),
        session.status,
        session.diagnostic
    );
    if *stamp == state {
        return;
    }
    *stamp = state;
    let cat = &strings.0;
    let locale = session.locale;
    let t = |key| cat.text(locale, key);
    for (label, ui, world_text) in &mut texts {
        let value = match label {
            Label::Key(key) => t(key),
            Label::Toggle => t(if session.paused {
                "controls.resume"
            } else {
                "controls.pause"
            }),
            Label::Help => t("c05.controls"),
            Label::Summary => {
                let minute = clock::minute_of_day(world.tick());
                let (hour, mins) = clock::clock_hhmm(minute);
                format!(
                    "{}\n{}: ({},{}) · {} {} · {:02}:{:02}\n{} {} · {}",
                    t("c05.player"),
                    t("c05.cell"),
                    world.player_position().x,
                    world.player_position().y,
                    t("inspector.tick"),
                    world.tick(),
                    hour,
                    mins,
                    t("c06.day"),
                    clock::day_number(world.tick()),
                    t(if session.paused {
                        "status.paused"
                    } else {
                        "status.running"
                    })
                )
            }
            Label::Body => body_text(&session, cat),
            Label::Context => t(match session.selected {
                Target::Npc(_) => "c05.talk",
                Target::HerbPatch => "c05.gather",
                Target::Shrine => "c05.deliver",
            }),
            Label::Marker(target) => target_name(*target, cat, locale),
        };
        if let Some(mut text) = ui {
            text.0 = value;
        } else if let Some(mut text) = world_text {
            text.0 = value;
        }
    }
}

fn block_key(block: PlannedBlock) -> &'static str {
    match block {
        PlannedBlock::Sleep => "c06.block.sleep",
        PlannedBlock::Eat => "c06.block.eat",
        PlannedBlock::Work => "c06.block.work",
        PlannedBlock::Socialize => "c06.block.socialize",
    }
}

fn phase_key(phase: Phase) -> &'static str {
    match phase {
        Phase::Requested => "c07.phase.requested",
        Phase::Assigned => "c07.phase.assigned",
        Phase::InProgress => "c07.phase.progress",
        Phase::Completed => "c07.phase.completed",
        Phase::Rejected => "c07.phase.rejected",
        Phase::Cancelled => "c07.phase.cancelled",
    }
}

fn goal_key(goal: u8) -> &'static str {
    match goal {
        1 => "c06.goal.1",
        2 => "c06.goal.2",
        3 => "c06.goal.3",
        4 => "c06.goal.4",
        5 => "c06.goal.5",
        _ => "c06.no_task",
    }
}

fn goal_color(goal: u8) -> Color {
    match goal {
        1 => Color::srgb(0.45, 0.78, 0.38),
        2 => Color::srgb(0.45, 0.58, 0.86),
        3 => Color::srgb(0.86, 0.55, 0.24),
        4 => Color::srgb(0.9, 0.78, 0.28),
        5 => Color::srgb(0.72, 0.46, 0.78),
        _ => Color::srgb(0.55, 0.58, 0.6),
    }
}

fn day_night(session: Res<Session>, mut clear: ResMut<ClearColor>) {
    let night = clock::is_night(clock::minute_of_day(session.world().tick()));
    clear.0 = if night {
        Color::srgb(0.015, 0.02, 0.04)
    } else {
        Color::srgb(0.07, 0.1, 0.08)
    };
}

fn metrics(time: Res<Time<Real>>, mut perf: ResMut<Performance>) {
    perf.frames += 1;
    if perf.frames > 60 {
        perf.intervals_ms.push_back(time.delta_secs_f64() * 1000.0);
        while perf.intervals_ms.len() > 600 {
            perf.intervals_ms.pop_front();
        }
    }
}

fn summary(values: impl IntoIterator<Item = f64>) -> serde_json::Value {
    let mut values: Vec<_> = values.into_iter().collect();
    if values.is_empty() {
        return serde_json::Value::Null;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    serde_json::json!({"samples":n,"mean_ms":values.iter().sum::<f64>()/n as f64,"p50_ms":values[(n-1)/2],"p95_ms":values[(n-1)*95/100],"max_ms":values[n-1]})
}

#[derive(bevy::ecs::system::SystemParam)]
struct CaptureSurface<'w, 's> {
    adapter: Option<Res<'w, RenderAdapterInfo>>,
    camera: Single<'w, 's, (&'static Camera, &'static GlobalTransform), With<MainCamera>>,
    buttons: Query<
        'w,
        's,
        (
            &'static Control,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    entities: Query<'w, 's, Entity>,
}

fn capture(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    session: Res<Session>,
    strings: Res<Strings>,
    performance: Res<Performance>,
    surface: CaptureSurface,
    mut index: Local<u64>,
) {
    if !keys.just_pressed(KeyCode::F12) {
        return;
    }
    let Some(dir) = std::env::var_os("SEALED_HEAVEN_EVIDENCE_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    if let Err(error) = std::fs::create_dir_all(&dir) {
        eprintln!("{error}");
        return;
    }
    *index += 1;
    let name = format!("c05-{:03}", *index);
    let world = session.world();
    let (camera, transform) = *surface.camera;
    let positions: Vec<_> = targets()
        .into_iter()
        .filter_map(|target| {
            let p = world.target_position(target)?;
            let p = match target {
                Target::Npc(id) => npc_at(id, p),
                _ => at(p),
            };
            camera
                .world_to_viewport(transform, p.extend(5.0))
                .ok()
                .map(|p| serde_json::json!({"target":format!("{target:?}"),"x":p.x,"y":p.y}))
        })
        .collect();
    let buttons:Vec<_>=surface.buttons.iter().map(|(control,node,transform)|serde_json::json!({"control":format!("{control:?}"),"x":transform.translation.x,"y":transform.translation.y,"width":node.size().x,"height":node.size().y})).collect();
    let report = serde_json::json!({"tick":world.tick(),"player_id":21,"player":{"x":world.player_position().x,"y":world.player_position().y,"herbs":world.player_herbs()},"source":world.source_herbs(),"delivered":world.delivered_herbs(),"requested":world.requested(),"completed":world.completed(),
        "paused":session.paused,"queued":session.queued(),"selected":format!("{:?}",session.selected),"locale":format!("{:?}",session.locale),"status":session.status,"diagnostic":session.diagnostic,"hash":hex(&world.hash()),"npc_hash":hex(&world.npcs().domain_checkpoint_sha256()),"npc_next_event":world.npcs().next_event_id(),"npc_count":20,"event_digest":hex(&world.event_digest()),"buttons":buttons,"targets":positions,"body":body_text(&session,&strings.0),
        "entities":surface.entities.iter().count(),"renderer":surface.adapter.map(|a|format!("{} / {:?}",a.name,a.backend)),"frame_time":summary(performance.intervals_ms.iter().copied()),"tick_compute":summary(session.tick_samples_ns.iter().map(|ns|*ns as f64/1_000_000.0)),"timing_note":"frame_time is Bevy wall frame interval, not isolated GPU time; tick_compute measures canonical drive, excludes rendering and save/capture","save_path":session.save_path.display().to_string()});
    if let Err(error) = std::fs::write(dir.join(format!("{name}.json")), report.to_string()) {
        eprintln!("{error}");
        return;
    }
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(dir.join(format!("{name}.png"))));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native_input_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AgencyPlugin))
            .add_message::<KeyboardInput>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Performance>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::ZERO,
            ))
            .add_systems(Update, (keyboard.before(drive_frame), metrics));
        app.update();
        app
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut().write_message(KeyboardInput {
            key_code: key,
            logical_key: bevy::input::keyboard::Key::Unidentified(
                bevy::input::keyboard::NativeKey::Unidentified,
            ),
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }

    #[test]
    fn c05_client01_native_real_clock_retains_stalls_and_frame_intervals() {
        let mut app = native_input_app();
        app.world_mut()
            .resource_mut::<Session>()
            .act(Action::Resume);
        app.world_mut().resource_mut::<Performance>().frames = 60;
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs(1),
        ));
        app.update();
        assert_eq!(app.world().resource::<Session>().world().tick(), 2);
        app.update();
        assert_eq!(app.world().resource::<Session>().world().tick(), 4);
        assert_eq!(
            app.world()
                .resource::<Performance>()
                .intervals_ms
                .back()
                .copied(),
            Some(1000.0)
        );
    }

    #[test]
    fn c05_client04_later_frame_step_shows_out_of_range_rejection() {
        let mut app = native_input_app();
        app.insert_resource(Strings(Catalog::agency().unwrap()));
        press(&mut app, KeyCode::KeyE);
        app.update();
        assert_eq!(app.world().resource::<Session>().queued(), 1);
        assert_eq!(app.world().resource::<Session>().world().tick(), 0);
        press(&mut app, KeyCode::KeyN);
        app.update();
        let world = app.world();
        let session = world.resource::<Session>();
        let body = body_text(session, &world.resource::<Strings>().0);
        assert!(!session.world().requested());
        assert_eq!(session.queued(), 0);
        assert_eq!(session.world().tick(), 1);
        assert!(body.contains("Target is not within one cell"), "{body}");
    }

    #[test]
    fn c05_client02_batched_controls_use_effective_ordered_state() {
        let mut app = native_input_app();
        for key in [KeyCode::Tab, KeyCode::KeyE, KeyCode::KeyN] {
            press(&mut app, key);
        }
        app.update();
        let session = app.world().resource::<Session>();
        assert_eq!(session.selected, Target::HerbPatch);
        assert_eq!(session.admitted.back().unwrap().intent, Intent::Gather);
        for key in [
            KeyCode::Tab,
            KeyCode::Tab,
            KeyCode::Space,
            KeyCode::Space,
            KeyCode::KeyL,
            KeyCode::KeyL,
        ] {
            press(&mut app, key);
        }
        app.update();
        let session = app.world().resource::<Session>();
        assert_eq!(session.selected, Target::Npc(2));
        assert!(session.paused);
        assert_eq!(session.locale, Locale::English);
        // A preceding UI button action must also affect this frame's keys.
        app.world_mut()
            .resource_mut::<Input>()
            .actions
            .push(Action::Select(Target::Npc(1)));
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::KeyE);
        press(&mut app, KeyCode::KeyN);
        app.update();
        let session = app.world().resource::<Session>();
        assert_eq!(session.selected, Target::HerbPatch);
        assert_eq!(session.admitted.back().unwrap().intent, Intent::Gather);
    }

    #[test]
    fn c05_client03_player_label_follows_hierarchy_after_move_and_reset() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, AgencyPlugin))
            .insert_resource(Strings(Catalog::agency().unwrap()))
            .add_systems(Startup, |mut commands: Commands, session: Res<Session>| {
                spawn_player(&mut commands, session.world().player_position());
                commands.spawn((TargetRing, Transform::default(), Visibility::Visible));
            })
            .add_systems(Update, paint.after(drive_frame));
        app.update();
        for action in [Action::Intent(Intent::Move(Direction::East)), Action::Step] {
            app.world_mut().resource_mut::<Input>().actions.push(action);
        }
        app.update();
        for reset in [false, true] {
            if reset {
                app.world_mut()
                    .resource_mut::<Input>()
                    .actions
                    .push(Action::Reset);
                app.update();
            }
            let world = app.world_mut();
            let expected = at(world.resource::<Session>().world().player_position());
            let mut labels = world
                .query_filtered::<(&ChildOf, &Transform, &GlobalTransform), With<PlayerIdentity>>();
            let (parent, local, global) = labels.single(world).unwrap();
            assert!(world.get::<PlayerMarker>(parent.parent()).is_some());
            assert_eq!(local.translation, Vec3::new(0.0, 0.0, 1.0));
            assert_eq!(global.translation().truncate(), expected);
        }
    }
    #[test]
    fn every_reason_and_target_has_a_translated_label() {
        let catalog = Catalog::agency().unwrap();
        for locale in [Locale::English, Locale::Indonesian] {
            for rejection in [
                Rejection::OutOfBounds,
                Rejection::Terrain,
                Rejection::Occupied,
                Rejection::OutOfRange,
                Rejection::UnknownTarget,
                Rejection::RequestNotAccepted,
                Rejection::Empty,
                Rejection::AlreadyCompleted,
                Rejection::Overflow,
            ] {
                let key = rejection_key(rejection);
                assert_ne!(catalog.text(locale, key), key);
            }
            for target in targets() {
                assert!(!target_name(target, &catalog, locale).is_empty());
            }
        }
    }
}
