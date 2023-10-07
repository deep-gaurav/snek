pub mod food;
pub mod menu;
pub mod networking;
pub mod snek;
pub mod window;

use bevy::{input::touch::TouchPhase, prelude::*, window::WindowResolution};
use bevy_rapier2d::{na::ComplexField, prelude::*};
use food::{handle_food_collision, spawn_food};
use menu::{clean_entry_menu, entry_menu, setup_menu};
use networking::{
    receive_msgs, send_snake_send, sync_add_move, sync_add_spawner, update_snake, AddMove,
    AddSpawn, ConnectionState, SendMessage, SnakeSyncTimer, SnakeUpdate, TransportMessage,
};
use serde::{Deserialize, Serialize};
use snek::{setup_snek, spawn_new_cell, update_cell_direction, update_head_sensor};
use window::{get_height, get_width};

#[derive(Debug, Resource)]
pub struct GameConfig {
    speed: f32,
    cell_size: (f32, f32),
    game_size: (u32, u32),
}

#[derive(Bundle)]
pub struct Snake {
    tag: SnakeTag,
    spatial: SpatialBundle,
    lastmove: LastMoveId,
    moves: Moves,
    spawners: Spawner,
}

#[derive(Bundle)]
pub struct SnakeCell {
    sprite: SpriteBundle,
    move_id: MoveId,
    direction: Direction,
    collider: Collider,
    sensor: Sensor,
    cell_tag: CellTag,
}

#[derive(Component, PartialEq)]
pub enum SnakeTag {
    SelfPlayerSnake,
    OtherPlayerSnake(u32),
}

#[derive(Component)]
pub struct Player;

#[derive(Component, Clone, Serialize, Deserialize)]
pub struct Direction(Vec2);

#[derive(Component, Serialize, Deserialize)]
pub struct MoveId(u32);

#[derive(Component)]
pub struct LastMoveId(u32);

pub type Move = (u32, Vec3, Direction);
#[derive(Component, Serialize, Deserialize, Clone)]
pub struct Moves {
    moves: Vec<Move>,
}

pub type SpawnDetail = (Vec3, Direction);

#[derive(Component, Serialize, Deserialize, Clone)]
pub struct Spawner {
    pub spawners: Vec<SpawnDetail>,
}

#[derive(Component)]
pub struct Head;

#[derive(Component)]
pub struct Tail;

#[derive(Component)]
pub struct HeadSensor;

#[derive(Event)]
pub struct ChangeDirection {
    direction: Vec2,
    head: Entity,
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct CellTag(u32);

#[derive(Event)]
pub enum InputsActions {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Hash, PartialEq, Eq, States, Default, Clone)]
pub enum GameStates {
    #[default]
    EntryMenu,
    GamePlay,
    GameOver,
}

impl GameStates {
    /// Returns `true` if the game states is [`EntryMenu`].
    ///
    /// [`EntryMenu`]: GameStates::EntryMenu
    #[must_use]
    pub fn is_entry_menu(&self) -> bool {
        matches!(self, Self::EntryMenu)
    }
}

fn main() {
    let mut app = App::new();
    app.insert_resource(GameConfig {
        speed: 60.0,
        cell_size: (20.0, 20.0),
        game_size: (0, 0),
    })
    .insert_resource(SnakeSyncTimer {
        timer: Timer::from_seconds(0.5, TimerMode::Repeating),
    })
    .insert_resource(ConnectionState::NotConnected)
    .add_state::<GameStates>()
    .add_event::<ChangeDirection>()
    .add_event::<InputsActions>()
    .add_event::<SnakeUpdate>()
    .add_event::<AddSpawn>()
    .add_event::<AddMove>()
    .add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resolution: WindowResolution::new(get_width(), get_height()),
            canvas: Some("#main_canvas".into()),
            ..Default::default()
        }),
        ..Default::default()
    }))
    .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
    // .add_plugins(LogDiagnosticsPlugin::default())
    // .add_plugins(FrameTimeDiagnosticsPlugin::default())
    .add_systems(Startup, (setup, setup_borders).chain())
    .add_systems(OnEnter(GameStates::EntryMenu), setup_menu)
    .add_systems(OnEnter(GameStates::GamePlay), setup_snek)
    .add_systems(OnExit(GameStates::EntryMenu), clean_entry_menu)
    .add_systems(Update, entry_menu.run_if(in_state(GameStates::EntryMenu)))
    .add_systems(
        Update,
        (
            update_cell_direction,
            move_cells.before(spawn_new_cell),
            keyboard_input,
            handle_touch,
            handle_input_event,
            update_head_sensor,
            spawn_new_cell,
            spawn_food,
            handle_food_collision,
        )
            .run_if(in_state(GameStates::GamePlay)),
    )
    .add_systems(
        Update,
        (
            receive_msgs,
            send_snake_send.run_if(in_state(GameStates::GamePlay)),
            (update_snake, sync_add_move, sync_add_spawner).run_if(in_state(GameStates::GamePlay)),
        ),
    );

    #[cfg(debug_assertions)]
    debug_plugins(&mut app);
    app.run()
}
#[cfg(debug_assertions)]
fn debug_plugins(app: &mut App) {
    app.add_plugins(RapierDebugRenderPlugin::default());

    // app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new());
}

fn setup(mut config: ResMut<GameConfig>, mut commands: Commands, window: Query<&Window>) {
    let window = window.single();
    config.game_size = (
        window.resolution.width() as u32,
        window.resolution.height() as u32,
    );
    commands.spawn(Camera2dBundle::default());
}

fn setup_borders(config: Res<GameConfig>, mut commands: Commands) {
    println!("Window size {},{}", config.game_size.0, config.game_size.1);
    let size = config.game_size;
    let _top_border = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.25),
                custom_size: Some(Vec2::new(size.0 as f32, 10.)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0., (size.1 as f32) / 2., 0.)),
            ..default()
        })
        .insert(Collider::cuboid((size.0 / 2) as f32, 5.));
    let _bottom_border = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.25),
                custom_size: Some(Vec2::new(size.0 as f32, 10.)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0., -(size.1 as f32) / 2., 0.)),
            ..default()
        })
        .insert(Collider::cuboid((size.0 / 2) as f32, 5.));

    let _left_border = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.25),
                custom_size: Some(Vec2::new(10., size.1 as f32)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(-(size.0 as f32) / 2., 0., 0.)),
            ..default()
        })
        .insert(Collider::cuboid(5., (size.1 / 2) as f32));
    let _right_border = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.25),
                custom_size: Some(Vec2::new(10., size.1 as f32)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new((size.0 as f32) / 2., 0., 0.)),
            ..default()
        })
        .insert(Collider::cuboid(5., (size.1 / 2) as f32));
}

fn move_cells(
    mut query: Query<(&mut Transform, &Direction)>,
    time: Res<Time>,
    config: Res<GameConfig>,
) {
    for (mut transform, direction) in query.iter_mut() {
        let direction = Vec3 {
            x: direction.0.x,
            y: direction.0.y,
            z: 0.0,
        };
        transform.translation += time.delta_seconds() * (config.speed) * direction;
    }
}

fn handle_input_event(
    mut event: EventReader<InputsActions>,
    mut query: Query<(Entity, &mut LastMoveId, &mut Moves), With<Player>>,
    mut head: Query<(&Parent, &Transform, &mut Direction, &mut MoveId, Entity), With<Head>>,
    mut ev_change_direction: EventWriter<ChangeDirection>,
    connection_handler: Res<ConnectionState>,
) {
    let Some(event) = event.iter().next() else {
        return;
    };
    let direction = match event {
        InputsActions::Up => Vec2 { x: 0.0, y: 1.0 },
        InputsActions::Down => Vec2 { x: 0.0, y: -1.0 },
        InputsActions::Left => Vec2 { x: -1.0, y: 0.0 },
        InputsActions::Right => Vec2 { x: 1.0, y: 0.0 },
    };

    let val = query.single_mut();
    let player_id = val.0;
    let mut last_move = val.1;
    let mut moves = val.2;
    for (parent, head, head_direction, _, head_id) in head.iter_mut() {
        if parent.get() == player_id
            && head_direction.0 != direction
            && (head_direction.0 + direction) != Vec2::ZERO
        {
            last_move.0 += 1;
            let _move = (
                last_move.0,
                head.translation
                    + Vec3 {
                        x: head_direction.0.x,
                        y: head_direction.0.y,
                        z: 0.0,
                    },
                Direction(direction),
            );
            moves.moves.push(_move.clone());
            if let ConnectionState::Connected(connection) = connection_handler.as_ref() {
                if let Err(err) = connection.sender.send(SendMessage::TransportMessage(
                    TransportMessage::AddMove(_move),
                )) {
                    warn!("{err:?}")
                }
            }
            ev_change_direction.send(ChangeDirection {
                head: head_id,
                direction,
            });

            // head_direction.0 = direction;
            // moveid.0 = last_move.0;
        }
    }
}

fn keyboard_input(keys: Res<Input<KeyCode>>, mut event: EventWriter<InputsActions>) {
    if keys.just_pressed(KeyCode::Up) {
        event.send(InputsActions::Up);
    } else if keys.just_pressed(KeyCode::Down) {
        event.send(InputsActions::Down);
    } else if keys.just_pressed(KeyCode::Left) {
        event.send(InputsActions::Left);
    } else if keys.just_pressed(KeyCode::Right) {
        event.send(InputsActions::Right);
    } else {
        return;
    }
}

fn handle_touch(touch_event: Res<Touches>, mut event: EventWriter<InputsActions>) {
    for touch in touch_event.iter_just_released() {
        let distance = touch.distance();
        const THRESHOLD: f32 = 50.0;
        if distance.x.abs() > distance.y.abs() && distance.x.abs() > THRESHOLD {
            if distance.x > 0. {
                event.send(InputsActions::Right);
            } else {
                event.send(InputsActions::Left);
            }
        } else if distance.y.abs() > THRESHOLD {
            if distance.y > 0. {
                event.send(InputsActions::Down);
            } else {
                event.send(InputsActions::Up);
            }
        }
    }
}
