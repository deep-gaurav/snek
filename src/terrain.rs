use bevy::{
    math::vec4,
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::render_resource::AsBindGroup,
    sprite::{Material2d, MaterialMesh2dBundle, Mesh2dHandle},
};

use crate::{GameConfig, Head};

#[derive(AsBindGroup, TypeUuid, Clone, TypePath)]
#[uuid = "1e449d2e-6901-4bff-95fa-d7407ad62b58"]
pub struct TerrainMaterial {
    #[uniform(0)]
    params: Vec4,

    #[texture(1)]
    #[sampler(2)]
    color_texture: Handle<Image>,

    #[texture(3)]
    #[sampler(4)]
    dirt_texture: Handle<Image>,

    #[texture(5)]
    #[sampler(6)]
    grass_texture2: Handle<Image>,

    #[texture(7)]
    #[sampler(8)]
    water_texture: Handle<Image>,
}

#[derive(Resource, Clone)]
pub struct TerrainMeshProp {
    mesh: Mesh2dHandle,
    material: Handle<TerrainMaterial>,
}

impl Material2d for TerrainMaterial {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "terrain_background.wgsl".into()
    }
}

pub fn setup_terrain(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
) {
    let material = terrain_materials.add(TerrainMaterial {
        params: vec4(0.1, 2.8, 14.0, rand::random()),
        color_texture: server.load("grass_03.jpeg"),
        dirt_texture: server.load("dirt_02.jpeg"),
        grass_texture2: server.load("grass_01.jpeg"),
        water_texture: server.load("tex_Water.jpg"),
    });
    let mesh = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(100.0, 100.0))));
    commands.insert_resource(TerrainMeshProp {
        material,
        mesh: mesh.into(),
    });
}

pub fn terrain_tiler(
    mut commands: Commands,
    terrains: Query<(Entity, &Terrain)>,
    player_head: Query<&Transform, With<Head>>,
    terrain_prop: Res<TerrainMeshProp>,
    config: Res<GameConfig>,
) {
    if let Some(head) = player_head.iter().next() {
        let block_x = head.translation.x as i32 / 100;
        let block_y = head.translation.y as i32 / 100;
        let horizontal = (config.game_size.0 as i32) / 200 + 3;
        let vertical = (config.game_size.1 as i32) / 200 + 3;

        for terr in terrains.iter() {
            if !((block_x - horizontal)..(block_x + horizontal)).contains(&terr.1.x)
                || !((block_y - vertical)..(block_y + vertical)).contains(&terr.1.y)
            {
                commands.entity(terr.0).despawn_recursive();
            }
        }
        for x in -horizontal..horizontal {
            for y in -vertical..vertical {
                let block_x = block_x + x;
                let block_y = block_y + y;
                if !terrains
                    .iter()
                    .any(|f| f.1.x == block_x && f.1.y == block_y)
                {
                    commands
                        .spawn((
                            Terrain {
                                x: block_x,
                                y: block_y,
                            },
                            TransformBundle::from_transform(Transform::from_translation(Vec3 {
                                x: (block_x * 100) as f32,
                                y: (block_y * 100) as f32,
                                z: 0.0,
                            })),
                            VisibilityBundle {
                                ..Default::default()
                            },
                        ))
                        .with_children(|terr| {
                            terr.spawn(MaterialMesh2dBundle {
                                mesh: terrain_prop.mesh.clone(),
                                material: terrain_prop.material.clone(),
                                ..Default::default()
                            });
                        });
                }
            }
        }
    }
}

#[derive(Component)]
pub struct Terrain {
    x: i32,
    y: i32,
}

pub fn sync_cam(
    mut transforms: Query<&mut Transform>,
    head: Query<Entity, With<Head>>,
    camera: Query<Entity, With<Camera>>,
) {
    if let (Some(head), Some(camera)) = (head.iter().next(), camera.iter().next()) {
        if let Ok(head_trans) = transforms.get(head) {
            let head_transform = head_trans.translation.clone();
            if let Ok(mut cam_trans) = transforms.get_mut(camera) {
                cam_trans.translation = head_transform;
            }
        }
    }
}
