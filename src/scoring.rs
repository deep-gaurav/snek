use bevy::prelude::*;

use crate::{networking::ConnectionState, SnakeTag};

pub fn sync_scores(
    snake_tag: Query<(&SnakeTag, &Children)>,
    mut connection_handler: ResMut<ConnectionState>,
) {
    if let ConnectionState::Connected(connection) = connection_handler.as_mut() {
        for (snek, children) in snake_tag.iter() {
            if children.len() >= 3 {
                let player = match snek {
                    SnakeTag::SelfPlayerSnake => {
                        if let Some(self_id) = connection.self_id {
                            connection.players.iter_mut().find(|p| p.user_id == self_id)
                        } else {
                            None
                        }
                    }
                    SnakeTag::OtherPlayerSnake(id) => {
                        connection.players.iter_mut().find(|p| &p.user_id == id)
                    }
                };
                if let Some(player) = player {
                    player.score = (children.len() - 3) as u32;
                    if player.score > player.highest_score {
                        player.highest_score = player.score;
                    }
                }
            }
        }
    }
}

#[derive(Component)]
pub struct Scoreboard;

#[derive(Component)]
pub struct ScoreContainer(u32);

#[derive(Component)]
pub struct ScoreText(u32);

pub fn setup_score(mut commands: Commands) {
    commands.spawn((
        Scoreboard,
        NodeBundle {
            background_color: Color::rgba(0.2, 0.2, 0.2, 0.2).into(),
            style: Style {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.),
                row_gap: Val::Px(10.),
                position_type: PositionType::Absolute,
                top: Val::Px(20.),
                right: Val::Px(20.),
                ..default()
            },
            ..default()
        },
    ));
}

pub fn display_scores(
    connection_handler: Res<ConnectionState>,
    mut q_score_text: Query<(&mut Text, &ScoreText)>,
    mut commands: Commands,
    q_scoreboard: Query<Entity, With<Scoreboard>>,
) {
    if let ConnectionState::Connected(connection) = connection_handler.as_ref() {
        for player in connection.players.iter() {
            let scoretxt = if player.score==player.highest_score {
                player.score.to_string()
            }else {
                format!("{} ({})", player.score, player.highest_score)
            };
            let score_text = q_score_text.iter_mut().find(|p| p.1 .0 == player.user_id);
            if let Some(mut text) = score_text {
                if let Some(section) = text.0.sections.first() {
                    if section.value != scoretxt {
                        let new_section = TextSection::new(
                            scoretxt,
                            section.style.clone(),
                        );
                        text.0.sections = vec![new_section];
                    }
                }
            } else {
                if let Ok(scoreboard) = q_scoreboard.get_single() {
                    commands.entity(scoreboard).with_children(|parent| {
                        parent
                            .spawn((
                                ScoreContainer(player.user_id),
                                NodeBundle {
                                    style: Style {
                                        flex_direction: FlexDirection::Row,
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
                                parent.spawn(
                                    NodeBundle{
                                        border_color: BorderColor::DEFAULT,
                                        background_color: player.color.into(),
                                        style: Style{
                                            width: Val::Px(20.),
                                            border: UiRect::all(Val::Px(
                                                if Some(player.user_id)== connection.self_id {
                                                    1.
                                                } else{
                                                    0.
                                                }
                                            )),
                                            height: Val::Px(20.),
                                            margin: UiRect::right(Val::Px(5.)),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    }
                                );
                                parent.spawn((
                                    ScoreText(player.user_id),
                                    TextBundle::from_section(
                                        scoretxt,
                                        TextStyle {
                                            font_size: 20.0,
                                            color: Color::rgb(0.9, 0.9, 0.9),
                                            ..Default::default()
                                        },
                                    ),
                                ));
                            });
                    });
                }
            }
        }
    }
}
