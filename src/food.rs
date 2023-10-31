use bevy::{prelude::*, window::PrimaryWindow};
use bevy_rapier2d::prelude::{Collider, CollisionEvent, Sensor};

use crate::{
    networking::{ConnectionState, SendMessage, TransportMessage},
    snek::KillSnake,
    CellTag, GameConfig, HeadSensor, Host, MoveId, SnakeCell, SnakeTag, Tail,
};

#[derive(Component, Debug)]
pub struct Food(pub u32);

pub fn spawn_food_system(
    mut commands: Commands,
    food_query: Query<&Food>,
    config: Res<GameConfig>,
    host: Query<&Host>,
    connection_handler: Res<ConnectionState>,
) {
    if host.is_empty() {
        return;
    }
    let pad = 20;
    if food_query.is_empty() {
        let (pos_x, pos_y) = {
            let rad = rand::random::<f32>() * 900.0;
            let angle = rand::random::<f32>() * std::f32::consts::PI * 2.0;
            let (sin, cos) = angle.sin_cos();
            (rad * sin, rad * cos)
        };
        if let ConnectionState::Connected(connection) = connection_handler.as_ref() {
            let food_id = rand::random();
            connection
                .sender
                .send(SendMessage::TransportMessage(TransportMessage::SpawnFood(
                    food_id,
                    Vec2 { x: pos_x, y: pos_y },
                )));
            commands.spawn(spawn_food(food_id, config.cell_size, pos_x, pos_y));
        }
    }
}

pub fn spawn_food(id: u32, cell_size: (f32, f32), pos_x: f32, pos_y: f32) -> impl Bundle {
    (
        Food(id),
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.85, 0.25, 0.75),
                custom_size: Some(Vec2::new(cell_size.0, cell_size.1)),
                ..default()
            },
            transform: Transform::from_translation(Vec3 {
                x: pos_x,
                y: pos_y,
                z: 2.0,
            }),
            ..Default::default()
        },
        Collider::cuboid(cell_size.0 / 2.0, cell_size.1 / 2.0),
        Sensor,
    )
}

pub fn handle_food_collision(
    mut collision_events: EventReader<CollisionEvent>,
    head_sensor: Query<(Entity, &HeadSensor, &Parent)>,
    food: Query<(Entity, &Food)>,
    head_cell: Query<(&Parent, &Transform, &crate::Direction)>,
    body_cell: Query<(Entity), With<CellTag>>,
    // mut snek: Query<&mut Spawner>,
    mut snek: Query<(Entity, &SnakeTag)>,
    mut commands: Commands,
    connection_handler: Res<ConnectionState>,
    snek_main: Query<(Entity, &SnakeTag)>,
    mut snake_kill_writer: EventWriter<KillSnake>,
    config: Res<GameConfig>,
    tail: Query<(&Parent, &Transform, &crate::Direction, &MoveId, Entity), With<Tail>>,
) {
    for collision_event in collision_events.iter() {
        if let CollisionEvent::Started(object, collider, _flags) = collision_event {
            // let heads = head_sensor.iter().map(|e|e.0).collect::<Vec<_>>();
            // let foods = food.iter().map(|e|e.0).collect::<Vec<_>>();

            let food = food.get(*collider).or(food.get(*object));
            let head = head_sensor.get(*object).or(head_sensor.get(*collider));
            let cell = body_cell.get(*collider);
            // info!("Collision food: {:?} head: {:?} cell {:?} object:{object:?}, collider: {collider:?} flags:{_flags:?}\nheads:{heads:?}\nfoods:{foods:?}", food, head, cell);
            if let (Ok(head), Ok(food)) = (head, food) {
                commands.entity(food.0).despawn_recursive();
                let headcell = head_cell.get(head.2.get());
                if let Ok(headcell) = headcell {
                    let collider_size = (config.cell_size.0 / 2.0, config.cell_size.1 / 2.0);
                    let snek = snek.iter().find(|p| p.1 == &SnakeTag::SelfPlayerSnake);
                    if let Some(snek) = snek {
                        let tail = tail.iter().find(|tail| tail.0.get() == snek.0);
                        if let Some(tail) = tail {
                            if let ConnectionState::Connected(connection) =
                                connection_handler.as_ref()
                            {
                                if let Some(player_id) = connection.self_id {
                                    if let Some(player) =
                                        connection.players.iter().find(|p| p.user_id == player_id)
                                    {
                                        let tail_position = tail.1.translation
                                            - Vec3 {
                                                x: tail.2 .0.x * config.cell_size.0,
                                                y: tail.2 .0.y * config.cell_size.1,
                                                z: 0.0,
                                            };
                                        let new_cell = SnakeCell {
                                            cell_tag: CellTag(rand::random()),

                                            collider: Collider::cuboid(
                                                collider_size.0,
                                                collider_size.1,
                                            ),
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
                                                transform: Transform::from_translation(
                                                    tail_position,
                                                ),
                                                ..default()
                                            },
                                        };
                                        let new_tail = commands.spawn(new_cell).insert(Tail).id();
                                        commands.entity(snek.0).push_children(&[new_tail]);
                                        commands.entity(tail.4).remove::<Tail>();
                                    }

                                    if let Err(err) =
                                        connection.sender.send(SendMessage::TransportMessage(
                                            TransportMessage::DespawnFood(food.1 .0),
                                        ))
                                    {
                                        warn!("{err:?}")
                                    }
                                }
                            }
                        }
                    }
                }
            } else if let (Ok(_head), Ok(_cell)) = (head, cell) {
                if let Some(snek) = snek_main.iter().find(|s| s.1 == &SnakeTag::SelfPlayerSnake) {
                    snake_kill_writer.send(KillSnake { snake_id: snek.0 });
                    if let ConnectionState::Connected(connection) = connection_handler.as_ref() {
                        connection
                            .sender
                            .send(SendMessage::TransportMessage(TransportMessage::KillSnake));
                    }
                }
            }
        }
    }
}

// pub fn

#[derive(Component)]
pub struct FoodPointer;

pub fn sync_food_pointer(
    food: Query<Entity, With<Food>>,
    pointer: Query<Entity, With<FoodPointer>>,
    mut transform: Query<&mut Transform>,
    global_transform: Query<&GlobalTransform>,
    camera: Query<(Entity, &Camera)>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut q_visibility: Query<&mut Visibility>,
) {
    let (Ok(food), Ok(pointer), Ok((camera_entity, camera)), Ok(window)) = (
        food.get_single(),
        pointer.get_single(),
        camera.get_single(),
        q_window.get_single(),
    ) else {
        return;
    };
    let Ok(camera_transform) = global_transform.get(camera_entity) else {
        return;
    };
    let rect = (window.width(), window.height());
    let (Some(top_left), Some(top_right), Some(bottom_left), Some(bottom_right)) = (
        camera.viewport_to_world_2d(camera_transform, Vec2 { x: 0., y: 0. }),
        camera.viewport_to_world_2d(camera_transform, Vec2 { x: rect.0, y: 0. }),
        camera.viewport_to_world_2d(camera_transform, Vec2 { x: 0., y: rect.1 }),
        camera.viewport_to_world_2d(
            camera_transform,
            Vec2 {
                x: rect.0,
                y: rect.1,
            },
        ),
    ) else {
        return;
    };
    let Ok(food_pos) = transform.get(food) else {
        return;
    };

    let ray = Ray {
        origin: camera_transform.translation(),
        direction: food_pos.translation - camera_transform.translation(),
    };
    let normal = [
        (top_left, bottom_left - top_left),
        (bottom_left, top_left - bottom_left),
        (bottom_left, bottom_left - bottom_right),
        (bottom_right, bottom_right - bottom_left),
    ];
    let mut dist = None;
    for (origin, normal) in normal.iter() {
        if let Some(dis) = ray.intersect_plane(
            Vec3::new(origin.x, origin.y, 0.),
            Vec3::new(normal.x, normal.y, 0.),
        ) {
            if let Some(dist_val) = dist {
                dist = Some(dis.min(dist_val));
            } else {
                dist = Some(dis);
            }
        }
    }
    if let Some(dist) = dist {
        let pt = ray.get_point(dist);
        let angle = Vec2 { x: -1.0, y: 0.0 }.angle_between(ray.direction.truncate().normalize());
        let visible = camera_transform
            .translation()
            .truncate()
            .distance_squared(food_pos.translation.truncate())
            > pt.truncate()
                .distance_squared(camera_transform.translation().truncate());

        let Ok(mut pointer_transform) = transform.get_mut(pointer) else {
            return;
        };
        let pt = Vec3::new(pt.x, pt.y, 2.0);
        let offset = -20.0 * ray.direction.truncate().normalize();
        let offset = Vec3::new(offset.x, offset.y, 2.0);
        pointer_transform.translation = pt + offset;
        pointer_transform.rotation = Quat::from_rotation_z(angle);
        *q_visibility.get_mut(pointer).unwrap() = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        // pointer_transform.look_to(food_pos, Vec3::Y);
    }
}
