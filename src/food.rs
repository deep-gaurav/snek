use bevy::{prelude::*, window::PrimaryWindow};
use bevy_rapier2d::prelude::{Collider, CollisionEvent, Sensor};

use crate::{
    networking::{ConnectionState, SendMessage, TransportMessage},
    snek::KillSnake,
    CellTag, GameConfig, HeadSensor, Host, SnakeTag, Spawner,
};

#[derive(Component,Debug)]
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
            let (sin,cos) = angle.sin_cos();
            (rad*sin, rad*cos)
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
    mut snek: Query<&mut Spawner>,
    mut commands: Commands,
    connection_handler: Res<ConnectionState>,
    snek_main: Query<(Entity, &SnakeTag)>,
    mut snake_kill_writer: EventWriter<KillSnake>,
) {
    for collision_event in collision_events.iter() {
        if let CollisionEvent::Started(object, collider, _flags) = collision_event {
            let food = food.get(*collider);
            let head = head_sensor.get(*object);
            let cell = body_cell.get(*collider);
            // info!("Collision food: {:?} head: {:?} cell {:?} object:{object:?}, collider: {collider:?} flags:{_flags:?}", food, head, cell);
            if let (Ok(head), Ok(food)) = (head, food) {
                commands.entity(food.0).despawn_recursive();
                let headcell = head_cell.get(head.2.get());
                if let Ok(headcell) = headcell {
                    if let Ok(mut snek) = snek.get_mut(headcell.0.get()) {
                        let spawn = (headcell.1.translation, headcell.2.clone());
                        snek.spawners.push(spawn.clone());

                        if let ConnectionState::Connected(connection) = connection_handler.as_ref()
                        {
                            if let Err(err) = connection.sender.send(SendMessage::TransportMessage(
                                TransportMessage::DespawnFood(food.1 .0),
                            )) {
                                warn!("{err:?}")
                            }
                            if let Err(err) = connection.sender.send(SendMessage::TransportMessage(
                                TransportMessage::AddSpawn(spawn),
                            )) {
                                warn!("{err:?}")
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
        let visible = camera_transform.translation().truncate().distance_squared(food_pos.translation.truncate())> pt.truncate().distance_squared(camera_transform.translation().truncate());

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
