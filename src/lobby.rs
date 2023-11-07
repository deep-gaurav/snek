use bevy::prelude::*;

use crate::{
    networking::{ConnectionState, PlayerProp, PlayersChanged},
    GameStates, Host,
};

#[derive(Component)]
pub struct LobbyMainNode;

#[derive(Component)]
pub struct PlayersNode;

#[derive(Component)]
pub struct PlayerNode(PlayerProp);

#[derive(Component)]
pub struct StartButton;

pub fn setup_lobby_menu(mut commands: Commands, connection_handler: Res<ConnectionState>) {
    if let ConnectionState::Connected(connection) = connection_handler.as_ref() {
         commands
            .spawn((
                LobbyMainNode,
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
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    format!("Room {}", connection.room_id),
                    TextStyle {
                        font_size: 40.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                        ..default()
                    },
                ));
                parent
                    .spawn((
                        PlayersNode,
                        NodeBundle {
                            style: Style {
                                display: Display::Flex,
                                flex_direction: FlexDirection::Column,
                                min_height: Val::Px(100.),
                                padding: UiRect::all(Val::Px(20.)),
                                // horizontally center child text
                                justify_content: JustifyContent::Center,
                                // vertically center child text
                                align_items: AlignItems::Center,
                                ..Default::default()
                            },
                            background_color: Color::rgb(0.5, 0.5, 0.5).into(),
                            ..default()
                        },
                    ))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "Players",
                            TextStyle {
                                font_size: 30.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                                ..default()
                            },
                        ));
                        parent.spawn(NodeBundle {
                            style: Style {
                                height: Val::Px(20.),
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                    });
            });
    }
}

pub fn update_player_details(
    lobby_query: Query<Entity, With<LobbyMainNode>>,
    players_node: Query<(Entity, &PlayersNode)>,
    game_button: Query<Entity, With<StartButton>>,
    host: Query<Entity, With<Host>>,
    mut players_changed: EventReader<PlayersChanged>,
    mut commands: Commands,
) {
    if let Some(player_ev) = players_changed.iter().next() {
        let players_node: (Entity, &PlayersNode) = players_node.single();
        commands.entity(players_node.0).despawn_descendants();
        for player in player_ev.players.iter() {
            let node = commands
                .spawn((
                    PlayerNode(player.clone()),
                    NodeBundle {
                        ..Default::default()
                    },
                ))
                .with_children(|parent| {
                    parent.spawn(NodeBundle {
                        background_color: player.color.into(),
                        style: Style {
                            width: Val::Px(30.),
                            height: Val::Px(30.),
                            margin: UiRect::right(Val::Px(10.)),
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                    parent.spawn(TextBundle::from_section(
                        format!(
                            "Player {}{}",
                            player.user_id,
                            if Some(player.user_id) == player_ev.self_player {
                                " (You)"
                            } else {
                                ""
                            }
                        ),
                        TextStyle {
                            font_size: 30.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                            ..default()
                        },
                    ));
                })
                .id();
            commands.get_entity(players_node.0).unwrap().add_child(node);
        }
        if host.is_empty() {
            for button in game_button.iter() {
                commands.entity(button).despawn_recursive();
            }
        } else {
            if game_button.is_empty() {
                let id = commands
                    .spawn((
                        StartButton,
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
                            "Start Game",
                            TextStyle {
                                font_size: 40.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                                ..default()
                            },
                        ));
                    })
                    .id();
                commands.entity(lobby_query.single()).add_child(id);
            }
        }
    }
}

pub fn lobby_handle_button(
    mut next_state: ResMut<NextState<GameStates>>,
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<StartButton>)>,
    connection_handler: ResMut<ConnectionState>,
    time: Res<Time>,
) {
    for interaction in &interaction_query {
        match *interaction {
            Interaction::Pressed => {
                if let ConnectionState::Connected(connection) = connection_handler.as_ref() {
                    if let Err(err) = connection
                        .sender
                        .send(crate::networking::SendMessage::TransportMessage(
                            crate::networking::TransportMessage::StartGame(time.elapsed_seconds()),
                        )) {
                            warn!("{err:?}")
                        }
                    next_state.set(GameStates::GamePlay);
                }
            }
            _ => {}
        }
    }
}

pub fn clean_lobby(lobby_query: Query<Entity, With<LobbyMainNode>>, mut commands: Commands) {
    for lobby_node in lobby_query.iter() {
        commands.entity(lobby_node).despawn_recursive();
    }
}
