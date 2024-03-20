use std::time::Duration;

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
};
use bevy_rapier3d::prelude::*;
use bevy_tweening::{
    component_animator_system, lens::TransformPositionLens, AnimationSystem, Animator,
    EaseFunction, RepeatCount, RepeatStrategy, Tween,
};
use noise::{
    utils::{NoiseMap, NoiseMapBuilder, PlaneMapBuilder},
    Perlin, Terrace,
};

use super::model::Seed;

#[derive(Debug)]
struct TerrainOptions {
    #[allow(dead_code)]
    seed: TerrainSeed,
    noise_scale: UVec2,
}

#[derive(Debug, Component)]
struct TerrainSeed {
    size: UVec2,
    seed: Seed<u32>,
}

impl TerrainSeed {
    pub fn new(size: UVec2, seed: Seed<u32>) -> Self {
        Self { size, seed }
    }

    fn noise(&self) -> NoiseMap {
        let perlin = Perlin::new(self.seed.into());

        let terraced: Terrace<_, _, 2> = Terrace::new(perlin)
            .add_control_point(-1.0)
            .add_control_point(-0.5)
            .add_control_point(0.1)
            .add_control_point(1.0)
            .invert_terraces(true);

        PlaneMapBuilder::new(terraced)
            .set_size(self.size.x as usize, self.size.y as usize)
            .build()
    }
}

#[derive(Component)]
pub struct Water {}

#[derive(Component)]
pub struct Terrain {
    options: TerrainOptions,
    noise: NoiseMap,
}

impl Into<Terrain> for TerrainSeed {
    fn into(self) -> Terrain {
        Terrain {
            noise: self.noise(),
            options: TerrainOptions {
                seed: self,
                noise_scale: UVec2::splat(2),
            },
        }
    }
}

impl Meshable for Terrain {
    type Output = Mesh;

    fn mesh(&self) -> Self::Output {
        let (w, h) = self.noise.size();
        let noise_size = UVec2::new(w as u32, h as u32);
        let noise_scale = self.options.noise_scale;
        let grid_size = noise_size * noise_scale;
        let offset: Vec2 = grid_size.as_vec2() * Vec2::splat(-0.5) + Vec2::splat(0.5);

        info!("grid-size={:?} offset={:?}", grid_size, offset);

        let grid_noise: Vec<_> = (0..grid_size.y)
            .into_iter()
            .map(|r| {
                (0..grid_size.x).into_iter().map(move |c| {
                    let grid = UVec2::new(c, r);
                    let index = grid / noise_scale;
                    let value = self.noise[(index.x as usize, index.y as usize)];
                    (grid, value as f32)
                })
            })
            .flatten()
            .collect();

        let positions: Vec<_> = grid_noise
            .iter()
            .map(|(grid, value)| {
                let p = grid.as_vec2() + offset;
                Vec3::new(p.x, *value, p.y)
            })
            .collect();

        let uvs: Vec<_> = grid_noise
            .iter()
            .map(|(grid, _)| grid.as_vec2() / grid_size.as_vec2())
            .collect();

        let colors: Vec<[f32; 4]> = grid_noise
            .iter()
            .map(|(_grid, value)| get_color(*value).as_rgba_f32())
            .collect();

        let normals: Vec<Vec3> = grid_noise.iter().map(|_| Vec3::Y).collect();

        let indices: Vec<_> = (0..grid_size.y - 1)
            .into_iter()
            .map(|r| {
                (0..grid_size.x - 1).into_iter().map(move |c| {
                    let i = r * grid_size.x;
                    let l = (r + 1) * grid_size.x;
                    vec![
                        i + c,     //
                        l + c,     //
                        l + c + 1, //
                        i + c,     //
                        l + c + 1, //
                        i + c + 1, //
                    ]
                })
            })
            .flatten()
            .flatten()
            .collect();

        let indices: Vec<_> = indices.into_iter().map(|v| v as u32).collect();

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(Indices::U32(indices))
    }
}

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_terrain).add_systems(
            Update,
            component_animator_system::<Water>.in_set(AnimationSystem::AnimationUpdate),
        );
    }
}

#[derive(Bundle)]
pub struct TerrainBundle {
    pbr: PbrBundle,
}

#[derive(Bundle)]
pub struct WaterBundle {
    pbr: PbrBundle,
}

fn generate_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let seed = TerrainSeed::new(UVec2::new(32, 32), Seed::default());

    let terrain: Terrain = seed.into();

    let terrain = TerrainBundle {
        pbr: PbrBundle {
            mesh: meshes.add(terrain.mesh()),
            material: materials.add(Color::rgb(1., 1., 1.)),
            ..Default::default()
        },
    };

    let water = TerrainBundle {
        pbr: PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(64.0, 64.0)),
            material: materials.add(Color::rgb(0., 0., 1.)),
            ..Default::default()
        },
    };

    commands.spawn((
        Name::new("Ground"),
        terrain,
        CollisionGroups::new(Group::all(), Group::all()),
        Collider::cuboid(20., 0.1, 20.),
        bevy_rts_camera::Ground,
    ));

    let tween = Tween::new(
        EaseFunction::QuadraticInOut,
        Duration::from_secs(2),
        TransformPositionLens {
            start: Vec3::ZERO,
            end: Vec3::new(0.0, 0.02, 0.0),
        },
    )
    .with_repeat_count(RepeatCount::Infinite)
    .with_repeat_strategy(RepeatStrategy::MirroredRepeat);

    commands.spawn((
        Name::new("Water"),
        Water {},
        water,
        CollisionGroups::new(Group::all(), Group::all()),
        Collider::cuboid(20., 0.1, 20.),
        Animator::new(tween),
    ));

    commands.spawn((
        Name::new("Sun"),
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                illuminance: 5000.,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform {
                translation: Vec3::new(-6.0, 20.0, 0.0),
                rotation: Quat::from_rotation_x(-std::f32::consts::PI / 4.),
                ..default()
            },
            ..default()
        },
    ));
}

fn get_color(val: f32) -> Color {
    let color = match val.abs() {
        v if v < 0.1 => Color::hex("#0a7e0a"),
        v if v < 0.2 => Color::hex("#0da50d"),
        v if v < 0.3 => Color::hex("#10cb10"),
        v if v < 0.4 => Color::hex("#18ed18"),
        v if v < 0.5 => Color::hex("#3ff03f"),
        v if v < 0.6 => Color::hex("#65f365"),
        v if v < 0.7 => Color::hex("#8cf68c"),
        v if v < 0.8 => Color::hex("#b2f9b2"),
        v if v < 0.9 => Color::hex("#d9fcd9"),
        v if v <= 1.0 => Color::hex("#ffffff"),
        _ => panic!("unexpected value"),
    };
    color.expect("bad color")
}
