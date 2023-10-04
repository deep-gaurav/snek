use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{CellTag, ChangeDirection, GameConfig, HeadSensor, MoveId, Moves, SnakeTag, Tail};

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
