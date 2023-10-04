use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{
    CellTag, ChangeDirection, GameConfig, HeadSensor, MoveId, Moves, SnakeCell, SnakeTag, Spawner,
    Tail,
};

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
    mut snek: Query<(Entity, &mut Spawner)>,
    config: Res<GameConfig>,
    tail: Query<(&Parent, &Transform, &crate::Direction, &MoveId, Entity), With<Tail>>,
) {
    let collider_size = (config.cell_size.0 / 2.0, config.cell_size.1 / 2.0);
    for mut snek in snek.iter_mut() {
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
                        cell_tag: CellTag,

                        collider: Collider::cuboid(collider_size.0, collider_size.1),
                        sensor: Sensor,
                        direction: crate::Direction(tail.2 .0),
                        move_id: MoveId(tail.3 .0),
                        sprite: SpriteBundle {
                            sprite: Sprite {
                                color: Color::rgb(0.25, 0.25, 0.75),
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
