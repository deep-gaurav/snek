use bevy::prelude::*;

use crate::{
    networking::{ConnectionState, SendMessage, TransportMessage},
    snek::KillSnake,
    HeadSensor, SnakeTag,
};

pub fn handle_kill_snake(
    mut kill_read: EventReader<KillSnake>,
    mut commands: Commands,
    snakes: Query<Entity, With<SnakeTag>>,
) {
    for event in kill_read.iter() {
        if let Ok(snake) = snakes.get(event.snake_id) {
            commands.entity(snake).despawn_recursive();
        }
    }
}

pub fn check_snek_position(
    head_sensor: Query<&GlobalTransform, With<HeadSensor>>,
    mut kill_write: EventWriter<KillSnake>,
    snek_head: Query<(Entity, &SnakeTag)>,
    connection_handler: Res<ConnectionState>,
) {
    for transform in head_sensor.iter() {
        const RADIUS_SQ: f32 = 1000.0 * 1000.0;
        let pos = transform.translation();
        if pos.x * pos.x + pos.y * pos.y > RADIUS_SQ {
            if let Some(snek) = snek_head.iter().find(|p| p.1 == &SnakeTag::SelfPlayerSnake) {
                kill_write.send(KillSnake { snake_id: snek.0 });
                if let ConnectionState::Connected(connection) = connection_handler.as_ref() {
                    connection
                        .sender
                        .send(SendMessage::TransportMessage(TransportMessage::KillSnake));
                }
            }
        }
    }
}
