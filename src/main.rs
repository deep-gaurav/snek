//! Shows how to render simple primitive shapes with a single color.
pub mod food;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    sprite::MaterialMesh2dBundle,
};
use bevy_rapier2d::prelude::*;

#[derive(Debug, Resource)]
pub struct GameConfig {
    speed: f32,
    cell_size: (f32, f32),
}

#[derive(Bundle)]
pub struct Snake {
    tag: SnakeTag,
    spatial: SpatialBundle,
    lastmove: LastMoveId,
    moves: Moves,
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

#[derive(Component)]
pub struct SnakeTag;

#[derive(Component)]
pub struct Player;

#[derive(Component, Clone)]
pub struct Direction(Vec2);

#[derive(Component)]
pub struct MoveId(u32);

#[derive(Component)]
pub struct LastMoveId(u32);

#[derive(Component)]
pub struct Moves {
    moves: Vec<(u32, Vec3, Direction)>,
}

#[derive(Component)]
pub struct Head;

#[derive(Component)]
pub struct Tail;

#[derive(Component)]
pub struct HeadSensor;

#[derive(Event)]
struct ChangeDirection {
    direction: Vec2,
    head: Entity,
}

#[derive(Component)]
pub struct CellTag;

fn main() {
    App::new()
        .insert_resource(GameConfig {
            speed: 60.0,
            cell_size: (20.0, 20.0),
        })
        .add_event::<ChangeDirection>()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        .add_plugins(RapierDebugRenderPlugin::default())
        // .add_plugins(LogDiagnosticsPlugin::default())
        // .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_cell_direction,
                move_cells,
                keyboard_input,
                update_head_sensor,
            ), // .before(keyboard_input)
               // .before(update_cell_direction),
        )
        .run();
}

fn setup(config: Res<GameConfig>, mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    let collider_size = (config.cell_size.0 / 2.0, config.cell_size.1 / 2.0);
    let cell_size = config.cell_size;
    let initial_position = (0.0, 0.0);

    let player_snake = commands
        .spawn((
            Snake {
                tag: SnakeTag,
                spatial: Default::default(),

                lastmove: LastMoveId(0),
                moves: Moves { moves: vec![] },
            },
            Player,
        ))
        .id();
    let cell1 = commands
        .spawn((
            SnakeCell {
                cell_tag: CellTag,
                collider: Collider::cuboid(collider_size.0, collider_size.1),
                sensor: Sensor,
                direction: Direction(Vec2 { x: 1.0, y: 0.0 }),
                sprite: SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgb(0.25, 0.25, 0.75),
                        custom_size: Some(Vec2::new(cell_size.0, cell_size.1)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        initial_position.0,
                        initial_position.1,
                        0.,
                    )),
                    ..default()
                },
                move_id: MoveId(0),
            },
            Head,
        ))
        .with_children(|head| {
            head.spawn(Collider::cuboid(1.0, collider_size.1))
                .insert(Sensor)
                .insert(HeadSensor)
                .insert(TransformBundle::from_transform(
                    Transform::from_translation(Vec3 {
                        x: cell_size.0 / 2.0,
                        y: 0.0,
                        z: 0.0,
                    }),
                ));
        })
        .id();

    let cell2 = commands
        .spawn((SnakeCell {
            cell_tag: CellTag,

            collider: Collider::cuboid(collider_size.0, collider_size.1),
            sensor: Sensor,
            direction: Direction(Vec2 { x: 1.0, y: 0.0 }),

            sprite: SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.25, 0.25, 0.75),
                    custom_size: Some(Vec2::new(cell_size.0, cell_size.1)),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(
                    initial_position.0 - cell_size.0,
                    initial_position.1,
                    0.,
                )),
                ..default()
            },
            move_id: MoveId(0),
        },))
        .id();
    let cell3 = commands
        .spawn((
            SnakeCell {
                cell_tag: CellTag,

                collider: Collider::cuboid(collider_size.0, collider_size.1),
                sensor: Sensor,
                direction: Direction(Vec2 { x: 1.0, y: 0.0 }),
                sprite: SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgb(0.25, 0.25, 0.75),
                        custom_size: Some(Vec2::new(cell_size.0, cell_size.1)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        initial_position.0 - (cell_size.0 * 2.0),
                        initial_position.1,
                        0.,
                    )),
                    ..default()
                },
                move_id: MoveId(0),
            },
            Tail,
        ))
        .id();
    commands
        .entity(player_snake)
        .push_children(&[cell1, cell2, cell3]);
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

fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    mut query: Query<(Entity, &mut LastMoveId, &mut Moves), With<Player>>,
    mut head: Query<(&Parent, &Transform, &mut Direction, &mut MoveId, Entity), With<Head>>,
    mut ev_change_direction: EventWriter<ChangeDirection>,
) {
    let val = query.single_mut();
    let player_id = val.0;
    let mut last_move = val.1;
    let mut moves = val.2;
    let direction;
    if keys.just_pressed(KeyCode::Up) {
        direction = Vec2 { x: 0.0, y: 1.0 };
    } else if keys.just_pressed(KeyCode::Down) {
        direction = Vec2 { x: 0.0, y: -1.0 };
    } else if keys.just_pressed(KeyCode::Left) {
        direction = Vec2 { x: -1.0, y: 0.0 };
    } else if keys.just_pressed(KeyCode::Right) {
        direction = Vec2 { x: 1.0, y: 0.0 };
    } else {
        return;
    }
    for (parent, head, head_direction, _, head_id) in head.iter_mut() {
        if parent.get() == player_id
            && head_direction.0 != direction
            && (head_direction.0 + direction) != Vec2::ZERO
        {
            last_move.0 += 1;
            moves.moves.push((
                last_move.0,
                head.translation
                    + Vec3 {
                        x: head_direction.0.x,
                        y: head_direction.0.y,
                        z: 0.0,
                    },
                Direction(direction),
            ));
            ev_change_direction.send(ChangeDirection {
                head: head_id,
                direction,
            });

            // head_direction.0 = direction;
            // moveid.0 = last_move.0;
        }
    }
}

fn update_cell_direction(
    mut query: Query<
        (
            &Parent,
            &mut Transform,
            &mut Direction,
            &mut MoveId,
            Option<&Tail>,
            Entity,
        ),
        With<CellTag>,
    >,
    mut moves_query: Query<&mut Moves, With<SnakeTag>>,
) {
    for mut cell in query.iter_mut() {
        let moves = moves_query.get_mut(cell.0.get());
        if let Ok(mut moves) = moves {
            for (move_index, _move) in moves.moves.iter_mut().enumerate() {
                if _move.0 > cell.3 .0 {
                    let mut current_dir = cell.2;
                    let distance = cell.1.translation - _move.1;
                    let distance_vec2 = Vec2 {
                        x: distance.x,
                        y: distance.y,
                    };
                    if distance_vec2.normalize() - current_dir.0 == Vec2::ZERO {
                        let extra_distance = _move.2 .0 * distance_vec2.distance(Vec2::ZERO);
                        current_dir.0 = _move.2 .0;
                        cell.1.translation = _move.1
                            + Vec3 {
                                x: extra_distance.x,
                                y: extra_distance.y,
                                z: 0.0,
                            };
                        cell.3 .0 = _move.0;

                        if cell.4.is_some() {
                            moves.moves.remove(move_index);
                        }
                    }
                    break;
                }
            }
        }
    }
}

fn update_head_sensor(
    config: Res<GameConfig>,
    mut ev_change_direction: EventReader<ChangeDirection>,
    mut head_sensor: Query<(&Parent, &mut Transform, &mut Collider), With<HeadSensor>>,
) {
    for event in ev_change_direction.iter() {
        for mut head_sensor in head_sensor.iter_mut() {
            if head_sensor.0.get() == event.head {
                if event.direction.x == 1.0 {
                    *head_sensor.2 = Collider::cuboid(1.0, config.cell_size.1 / 2.0);
                    head_sensor.1.translation = Vec3 {
                        x: config.cell_size.0 / 2.0,
                        y: 0.0,
                        z: 0.0,
                    };
                } else if event.direction.x == -1.0 {
                    *head_sensor.2 = Collider::cuboid(1.0, config.cell_size.1 / 2.0);
                    head_sensor.1.translation = Vec3 {
                        x: -config.cell_size.0 / 2.0,
                        y: 0.0,
                        z: 0.0,
                    };
                } else if event.direction.y == 1.0 {
                    *head_sensor.2 = Collider::cuboid(config.cell_size.0 / 2.0, 1.0);
                    head_sensor.1.translation = Vec3 {
                        y: config.cell_size.0 / 2.0,
                        x: 0.0,
                        z: 0.0,
                    };
                } else if event.direction.y == -1.0 {
                    *head_sensor.2 = Collider::cuboid(config.cell_size.0 / 2.0, 1.0);
                    head_sensor.1.translation = Vec3 {
                        y: -config.cell_size.0 / 2.0,
                        x: 0.0,
                        z: 0.0,
                    };
                }
            }
        }
    }
}
