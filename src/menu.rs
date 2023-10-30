use crate::networking::{connect_transport, ConnectionState};
use crate::GameStates;
use bevy::{prelude::*, tasks::AsyncComputeTaskPool};
use rand::Rng;
use xwebtransport_core::datagram::Receive;
use xwebtransport_core::{traits::EndpointConnect, AcceptBiStream, Connecting};

#[derive(Component)]
pub struct EntryMenuNode;

#[derive(Component)]
pub struct HostJoinButtonsContainer;

#[derive(Component)]
pub struct HostButton;

#[derive(Component)]
pub struct JoinButton;

#[derive(Component)]
pub struct TypeButton(String);

#[derive(Component)]
pub struct BackButton;

#[derive(Component)]
pub struct RoomIdInputField;

#[derive(Component)]
pub struct JoinRoomSubmitButton;

pub fn setup_menu(mut commands: Commands) {
    let button_entity = commands
        .spawn((
            EntryMenuNode,
            NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.),
                    row_gap: Val::Px(10.),
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Snek",
                TextStyle {
                    font_size: 40.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                    ..default()
                },
            ));
            parent.spawn(TextBundle::from_section(
                "with Friends",
                TextStyle {
                    font_size: 35.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                    ..default()
                },
            ));
            parent
                .spawn((
                    HostJoinButtonsContainer,
                    NodeBundle {
                        style: Style {
                            // center button
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.),
                            ..default()
                        },
                        ..default()
                    },
                ))
                .with_children(|parent| {
                    parent
                        .spawn((
                            HostButton,
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(150.),
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
                                "Host",
                                TextStyle {
                                    font_size: 30.0,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            ));
                        });

                    parent
                        .spawn((
                            JoinButton,
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(150.),
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
                                "Join",
                                TextStyle {
                                    font_size: 30.0,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            ));
                        });
                });
        })
        .id();
}

pub fn entry_menu(
    q_host_join_container: Query<Entity, With<HostJoinButtonsContainer>>,
    interaction_query: Query<(Entity, &Interaction), Changed<Interaction>>,
    host_button: Query<Entity, With<HostButton>>,
    join_button: Query<Entity, With<JoinButton>>,
    mut commands: Commands,
    connection_handler: ResMut<ConnectionState>,
    q_entry_menu_node: Query<Entity, With<EntryMenuNode>>,
    q_text_button: Query<&TypeButton>,
    q_back_button: Query<&BackButton>,
    q_join_submit_button: Query<&JoinRoomSubmitButton>,
    mut room_input: Query<&mut Text, With<RoomIdInputField>>,
    asset_server: Res<AssetServer>,
) {
    for interaction in &interaction_query {
        match *interaction.1 {
            Interaction::Pressed => {
                if host_button.get(interaction.0).is_ok() {
                    let mut rng = rand::thread_rng();
                    let random_number: u32 = rng.gen_range(100_000..1_000_000);
                    let random_string = format!("{:06}", random_number);
                
                    connect_transport(&random_string, connection_handler);
                    break;
                } else if join_button.get(interaction.0).is_ok() {
                    for q in q_host_join_container.iter() {
                        commands.entity(q).despawn_recursive();
                    }
                    if let Ok(root) = q_entry_menu_node.get_single() {
                        setup_numpad(root, asset_server, &mut commands);
                    }
                    break;
                } else if let Ok(but) = q_text_button.get(interaction.0) {
                    if let Ok(mut input) = room_input.get_single_mut() {
                        if let Some(section) = input.sections.first() {
                            if section.value.len() < 6 {
                                let new_section = TextSection::new(
                                    format!("{}{}", section.value, but.0),
                                    section.style.clone(),
                                );
                                input.sections = vec![new_section];
                            }
                        }
                    }
                } else if let Ok(but) = q_back_button.get(interaction.0) {
                    if let Ok(mut input) = room_input.get_single_mut() {
                        if let Some(section) = input.sections.first() {
                            let mut chars = section.value.chars();
                            chars.next_back();
                            let new_section =
                                TextSection::new(chars.as_str(), section.style.clone());
                            input.sections = vec![new_section];
                        }
                    }
                }else if let Ok(but) = q_join_submit_button.get(interaction.0) {
                    if let Ok(mut input) = room_input.get_single_mut() {
                        if let Some(section) = input.sections.first() {
                            if section.value.len() == 6 {
                                connect_transport(&section.value, connection_handler);
                                break;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn setup_numpad(parent: Entity, asset_server: Res<AssetServer>, commands: &mut Commands) {
    commands.entity(parent).with_children(|parent| {
        parent
            .spawn(NodeBundle {
                background_color: Color::rgba(0.05, 0.05, 0.05, 1.0).into(),

                style: Style {
                    // center button
                    margin: UiRect::vertical(Val::Px(20.)),
                    width: Val::Px(200.),
                    height: Val::Px(100.),
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ..default()
            })
            .with_children(|parent| {
                parent.spawn((
                    TextBundle::from_section(
                        "",
                        TextStyle {
                            font_size: 30.0,
                            color: Color::rgb(0.9, 0.9, 0.9),

                            ..default()
                        },
                    ),
                    RoomIdInputField,
                ));
            });
        parent
            .spawn(NodeBundle {
                background_color: Color::rgba(0.05, 0.05, 0.05, 1.0).into(),
                style: Style {
                    // center button
                    width: Val::Percent(100.),
                    max_width: Val::Px(300.0),
                    display: Display::Grid,

                    grid_template_columns: RepeatedGridTrack::fr(3, 1.),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Stretch,
                    column_gap: Val::Px(10.),
                    row_gap: Val::Px(10.),
                    ..default()
                },
                ..default()
            })
            .with_children(|parent| {
                (1..10).for_each(|num| {
                    parent
                        .spawn((
                            ButtonBundle {
                                style: Style {
                                    // horizontally center child text
                                    justify_content: JustifyContent::Center,
                                    // vertically center child text
                                    align_items: AlignItems::Center,
                                    padding: UiRect::vertical(Val::Px(10.)),
                                    ..default()
                                },
                                background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                                ..Default::default()
                            },
                            TypeButton(num.to_string()),
                        ))
                        .with_children(|parent| {
                            parent.spawn((TextBundle::from_section(
                                format!("{num}"),
                                TextStyle {
                                    font_size: 30.0,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            ),));
                        });
                });
                parent
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                // horizontally center child text
                                justify_content: JustifyContent::Center,
                                // vertically center child text
                                align_items: AlignItems::Center,
                                padding: UiRect::vertical(Val::Px(10.)),
                                ..default()
                            },
                            background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                            ..Default::default()
                        },
                        BackButton,
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(30.0),
                                    height: Val::Px(30.0),
                                    ..default()
                                },
                                background_color: Color::WHITE.into(),
                                ..default()
                            },
                            UiImage::new(asset_server.load("icons8-backspace-30.png")),
                        ));
                    });

                parent
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                // horizontally center child text
                                justify_content: JustifyContent::Center,
                                // vertically center child text
                                align_items: AlignItems::Center,
                                padding: UiRect::vertical(Val::Px(10.)),
                                ..default()
                            },
                            background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                            ..Default::default()
                        },
                        TypeButton(0.to_string()),
                    ))
                    .with_children(|parent| {
                        parent.spawn((TextBundle::from_section(
                            format!("0"),
                            TextStyle {
                                font_size: 30.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                                ..default()
                            },
                        ),));
                    });

                parent
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                // horizontally center child text
                                justify_content: JustifyContent::Center,
                                // vertically center child text
                                align_items: AlignItems::Center,
                                padding: UiRect::vertical(Val::Px(10.)),
                                ..default()
                            },
                            background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                            ..Default::default()
                        },
                        JoinRoomSubmitButton,
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(30.0),
                                    height: Val::Px(30.0),
                                    ..default()
                                },
                                background_color: Color::WHITE.into(),
                                ..default()
                            },
                            UiImage::new(asset_server.load("icons8-done-50.png")),
                        ));
                    });
            });
    });
}

pub fn clean_entry_menu(menu_query: Query<Entity, With<EntryMenuNode>>, mut commands: Commands) {
    for menu_node in menu_query.iter() {
        commands.entity(menu_node).despawn_recursive();
    }
}
