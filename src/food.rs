use bevy::prelude::*;
use bevy_rapier2d::prelude::{Collider, CollisionEvent, Sensor};

use crate::{
    networking::{ConnectionState, SendMessage, TransportMessage},
    snek::KillSnake,
    CellTag, GameConfig, HeadSensor, Host, SnakeTag, Spawner,
};

#[derive(Component)]
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
        let pos_x = rand::random::<f32>() * ((config.game_size.0 - pad) as f32)
            - ((config.game_size.0 / 2) as f32);
        let pos_y = rand::random::<f32>() * ((config.game_size.1 - pad) as f32)
            - ((config.game_size.1 / 2) as f32);

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
