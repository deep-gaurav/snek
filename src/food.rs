use bevy::prelude::*;
use bevy_rapier2d::prelude::{Collider, CollisionEvent, ContactForceEvent, Sensor};

use crate::{GameConfig, HeadSensor, Spawner};

#[derive(Component)]
pub struct Food;

pub fn spawn_food(mut commands: Commands, food_query: Query<&Food>, config: Res<GameConfig>) {
    let pad = 20;
    if food_query.is_empty() {
        let pos_x = rand::random::<f32>() * ((config.game_size.0 - pad) as f32)
            - ((config.game_size.0 / 2) as f32);
        let pos_y = rand::random::<f32>() * ((config.game_size.1 - pad) as f32)
            - ((config.game_size.1 / 2) as f32);

        let food = commands
            .spawn((
                Food,
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgb(0.85, 0.25, 0.75),
                        custom_size: Some(Vec2::new(config.cell_size.0, config.cell_size.1)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3 {
                        x: pos_x,
                        y: pos_y,
                        z: 0.,
                    }),
                    ..Default::default()
                },
                Collider::cuboid(config.cell_size.0 / 2.0, config.cell_size.1 / 2.0),
                Sensor,
            ))
            .id();
    }
}

pub fn handle_food_collision(
    mut collision_events: EventReader<CollisionEvent>,
    head_sensor: Query<(Entity, &HeadSensor, &Parent)>,
    food: Query<Entity, &Food>,
    head_cell: Query<(&Parent, &Transform, &crate::Direction)>,
    mut snek: Query<&mut Spawner>,
    mut commands: Commands,
) {
    for collision_event in collision_events.iter() {
        if let CollisionEvent::Started(object, collider, flags) = collision_event {
            let food = food.get(*collider);
            let head = head_sensor.get(*object);
            if let (Ok(head), Ok(food)) = (head, food) {
                commands.entity(food).despawn_recursive();
                let headcell = head_cell.get(head.2.get());
                if let Ok(headcell) = headcell {
                    if let Ok(mut snek) = snek.get_mut(headcell.0.get()) {
                        snek.spawners
                            .push((headcell.1.translation, headcell.2.clone()));
                    }
                }
            }
        }
    }
}

// pub fn
