use bevy::prelude::*;

use crate::GameStates;

#[derive(Component)]
pub struct EntryMenuNode;

pub fn setup_menu(mut commands: Commands) {
    let button_entity = commands
        .spawn((
            EntryMenuNode,
            NodeBundle {
                style: Style {
                    // center button
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn(ButtonBundle {
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
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Play",
                        TextStyle {
                            font_size: 40.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                            ..default()
                        },
                    ));
                });
        })
        .id();
}

pub fn entry_menu(
    mut next_state: ResMut<NextState<GameStates>>,
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<Button>)>,
) {
    for interaction in &interaction_query {
        match *interaction {
            Interaction::Pressed => {
                next_state.set(GameStates::GamePlay);
            }
            _ => {}
        }
    }
}

pub fn clean_entry_menu(menu_query: Query<Entity, With<EntryMenuNode>>, mut commands: Commands) {
    for menu_node in menu_query.iter() {
        commands.entity(menu_node).despawn_recursive();
    }
}
