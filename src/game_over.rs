use bevy::prelude::*;

use crate::{
    networking::{ConnectionState, SendMessage, TransportMessage},
    snek::{KillSnake, SpawnSnake},
    HeadSensor, SnakeTag, Head,
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

#[derive(Component)]
pub struct GameOvermenu;

#[derive(Component)]
pub struct RespawnButton;

pub fn respawn_menu_system(game_over_menu: Query<Entity, With<GameOvermenu>>, head: Query<&Head>, mut commands: Commands){

    if head.is_empty() && game_over_menu.is_empty() {
        info!("You died");
          commands
            .spawn((
                GameOvermenu,
                NodeBundle {
                    style: Style {
                        // center button
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        justify_content: JustifyContent::SpaceEvenly,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(10.),
                        
                        ..default()
                    },
                    background_color: Color::rgba(0.2, 0.2, 0.2, 0.7).into(),
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    "You died",
                    TextStyle {
                        font_size: 40.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                        ..default()
                    },
                ));
                parent.spawn((
                    RespawnButton,
                    ButtonBundle {
                        style: Style {
                            height: Val::Px(65.),
                            // horizontally center child text
                            justify_content: JustifyContent::Center,
                            // vertically center child text
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                        ..default()
                    },
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Respawn",
                        TextStyle {
                            font_size: 40.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                            ..default()
                        },
                    ));
                });
            });
    }
    else if !head.is_empty() && !game_over_menu.is_empty() {
        if let Ok(entity) = game_over_menu.get_single(){
            commands.entity(entity).despawn_recursive();
        }
    }
}

pub fn respawn_handle_button(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<RespawnButton>)>,
    mut spawn_snek_writer: EventWriter<SpawnSnake>
) {
    for interaction in &interaction_query {
        match *interaction {
            Interaction::Pressed => {
                info!("Spawn pressed");
                spawn_snek_writer.send(SpawnSnake);
            }
            _ => {}
        }
    }
}