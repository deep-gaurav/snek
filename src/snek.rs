use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{
    networking::ConnectionState, CellTag, ChangeDirection, GameConfig, Head, HeadSensor,
    LastMoveId, MainCamera, MoveId, Moves, Player, Snake, SnakeCell, SnakeTag, Spawner, Tail,
};

pub fn setup_snek(
    config: Res<GameConfig>,
    mut commands: Commands,
    connection_handler: Res<ConnectionState>,
) {
    if let ConnectionState::Connected(connection) = connection_handler.as_ref() {
        let Some(player_id) = connection.self_id else {
            return;
        };
        let Some(player) = connection.players.iter().find(|p| p.user_id == player_id) else {
            return;
        };

        let collider_size = (config.cell_size.0 / 2.0, config.cell_size.1 / 2.0);
        let cell_size = config.cell_size;
        let initial_position = (0.0, 0.0);

        let player_snake = commands
            .spawn((
                Snake {
                    tag: SnakeTag::SelfPlayerSnake,
                    spatial: Default::default(),

                    lastmove: LastMoveId(0),
                    moves: Moves { moves: vec![] },
                    spawners: Spawner { spawners: vec![] },
                },
                Player,
            ))
            .id();
        let cell1 = commands
            .spawn((
                SnakeCell {
                    cell_tag: CellTag(rand::random()),
                    collider: Collider::cuboid(collider_size.0, collider_size.1),
                    sensor: Sensor,
                    direction: crate::Direction(Vec2 { x: 1.0, y: 0.0 }),
                    sprite: SpriteBundle {
                        sprite: Sprite {
                            color: player.color,
                            custom_size: Some(Vec2::new(cell_size.0, cell_size.1)),
                            ..default()
                        },
                        transform: Transform::from_translation(Vec3::new(
                            initial_position.0,
                            initial_position.1,
                            1.,
                        )),
                        ..default()
                    },
                    move_id: MoveId(0),
                },
                Head,
            ))
            .with_children(|head| {
                head.spawn(Collider::cuboid(1.0, collider_size.1))
                    .insert(RigidBody::KinematicPositionBased)
                    .insert(Ccd::enabled())
                    .insert(HeadSensor)
                    .insert(ActiveCollisionTypes::all())
                    .insert(ActiveEvents::COLLISION_EVENTS)
                    .insert(TransformBundle::from_transform(
                        Transform::from_translation(Vec3 {
                            x: cell_size.0 / 2.0,
                            y: 0.0,
                            z: 0.0,
                        }),
                    ));
                head.spawn((
                    Camera2dBundle {
                        camera: Camera {
                            order: 10,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    MainCamera,
                ));
            })
            .id();

        let cell2 = commands
            .spawn((SnakeCell {
                cell_tag: CellTag(rand::random()),

                collider: Collider::cuboid(collider_size.0, collider_size.1),
                sensor: Sensor,
                direction: crate::Direction(Vec2 { x: 1.0, y: 0.0 }),

                sprite: SpriteBundle {
                    sprite: Sprite {
                        color: player.color,
                        custom_size: Some(Vec2::new(cell_size.0, cell_size.1)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        initial_position.0 - cell_size.0,
                        initial_position.1,
                        1.,
                    )),
                    ..default()
                },
                move_id: MoveId(0),
            },))
            .id();
        let cell3 = commands
            .spawn((
                SnakeCell {
                    cell_tag: CellTag(rand::random()),

                    collider: Collider::cuboid(collider_size.0, collider_size.1),
                    sensor: Sensor,
                    direction: crate::Direction(Vec2 { x: 1.0, y: 0.0 }),
                    sprite: SpriteBundle {
                        sprite: Sprite {
                            color: player.color,
                            custom_size: Some(Vec2::new(cell_size.0, cell_size.1)),
                            ..default()
                        },
                        transform: Transform::from_translation(Vec3::new(
                            initial_position.0 - (cell_size.0 * 2.0),
                            initial_position.1,
                            1.,
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
}

pub fn update_cell_direction(
    mut query: Query<
        (
            &Parent,
            &mut Transform,
            &mut crate::Direction,
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

pub fn update_head_sensor(
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

pub fn spawn_new_cell(
    mut commands: Commands,
    mut snek: Query<(Entity, &mut Spawner, &SnakeTag)>,
    connection_handler: Res<ConnectionState>,
    config: Res<GameConfig>,
    tail: Query<(&Parent, &Transform, &crate::Direction, &MoveId, Entity), With<Tail>>,
) {
    let ConnectionState::Connected(connection) = connection_handler.as_ref() else {
        return;
    };
    let collider_size = (config.cell_size.0 / 2.0, config.cell_size.1 / 2.0);
    for mut snek in snek.iter_mut() {
        let snake_user_id = match snek.2 {
            SnakeTag::SelfPlayerSnake => {
                let Some(id) = connection.self_id else {
                    continue;
                };
                id
            }
            SnakeTag::OtherPlayerSnake(id) => *id,
        };
        let Some(player) = connection
            .players
            .iter()
            .find(|p| p.user_id == snake_user_id)
        else {
            continue;
        };
        if let Some((point_index, spawn_point)) = snek.1.spawners.iter().enumerate().next() {
            let tail = tail.iter().find(|tail| tail.0.get() == snek.0);
            if let Some(tail) = tail {
                let mut current_dir = tail.2;
                let tail_position = tail.1.translation
                    - Vec3 {
                        x: current_dir.0.x * config.cell_size.0,
                        y: current_dir.0.y * config.cell_size.1,
                        z: 0.0,
                    };
                let distance = tail.1.translation - spawn_point.0;
                let distance_vec2 = Vec2 {
                    x: distance.x,
                    y: distance.y,
                };
                if distance_vec2.distance(Vec2::ZERO) < 1.0 {
                    let extra_distance = spawn_point.1 .0 * distance_vec2.distance(Vec2::ZERO);
                    let new_cell = SnakeCell {
                        cell_tag: CellTag(rand::random()),

                        collider: Collider::cuboid(collider_size.0, collider_size.1),
                        sensor: Sensor,
                        direction: crate::Direction(tail.2 .0),
                        move_id: MoveId(tail.3 .0),
                        sprite: SpriteBundle {
                            sprite: Sprite {
                                color: player.color,
                                custom_size: Some(Vec2::new(
                                    config.cell_size.0,
                                    config.cell_size.1,
                                )),
                                ..default()
                            },
                            transform: Transform::from_translation(tail_position),
                            ..default()
                        },
                    };
                    // cell.1.translation = _move.1
                    //     + Vec3 {
                    //         x: extra_distance.x,
                    //         y: extra_distance.y,
                    //         z: 0.0,
                    //     };

                    // if cell.4.is_some() {
                    //     moves.moves.remove(move_index);
                    // }
                    let new_tail = commands.spawn(new_cell).insert(Tail).id();
                    commands.entity(snek.0).push_children(&[new_tail]);
                    commands.entity(tail.4).remove::<Tail>();
                    snek.1.spawners.remove(point_index);
                }
            }
        }
    }
}
