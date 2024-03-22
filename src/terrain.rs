use bevy::{pbr::wireframe::NoWireframe, prelude::*};
use bevy_rapier3d::prelude::*;
use bevy_tweening::{
    component_animator_system, lens::TransformPositionLens, AnimationSystem, Animator,
    EaseFunction, RepeatCount, RepeatStrategy, Tween,
};
use noise::{
    utils::{NoiseMap, NoiseMapBuilder, PlaneMapBuilder},
    Perlin, Terrace,
};
use std::time::Duration;

mod mesh;

use self::mesh::{HeightOnlyCell, RectangularMapping};

use super::model::Seed;
use super::model::TILE_SIZE;
use crate::{AroundCenter, SquareGrid};

#[derive(Clone, Default, Debug)]
struct TerrainSeed {
    seed: Seed<u32>,
}

impl TerrainSeed {
    #[allow(dead_code)]
    pub fn new(seed: Seed<u32>) -> Self {
        Self { seed }
    }

    fn into(self) -> u32 {
        self.seed.into()
    }
}

#[derive(Debug)]
struct TerrainOptions {
    seed: TerrainSeed,
    size: UVec2,
}

impl TerrainOptions {
    fn new(seed: TerrainSeed, size: UVec2) -> Self {
        Self { seed, size }
    }

    fn noise(&self) -> NoiseMap {
        let perlin = Perlin::new(self.seed.clone().into());

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

#[derive(Component, Debug)]
pub struct Water {}

#[derive(Component)]
pub struct Terrain {
    options: TerrainOptions,
    grid: SquareGrid<HeightOnlyCell>,
}

impl Terrain {
    pub fn world_to_grid(&self, position: Vec3) -> Option<UVec2> {
        let local = position + self.grid.world_to_local() + (TILE_SIZE / 2.0);
        let local = local.xz();

        info!(
            "world-to-local={:?} position={:?} local={:?}",
            self.grid.world_to_local(),
            position,
            local
        );

        if local.x > self.options.size.x as f32
            || local.y > self.options.size.y as f32
            || local.x < 0.0
            || local.y < 0.0
        {
            None
        } else {
            Some(local.as_uvec2())
        }
    }

    pub fn survey(&self, position: Vec3) -> Option<Survey> {
        match self.world_to_grid(position) {
            Some(index) => {
                let index = index.as_ivec2();
                info!("{:?} {:#?}", index, self.grid.around(index));

                None
            }
            None => None,
        }
    }

    fn size(&self) -> UVec2 {
        self.options.size
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Survey {
    Ground,
    Beach,
    Water,
}

impl From<TerrainOptions> for Terrain {
    fn from(value: TerrainOptions) -> Self {
        let flat: SquareGrid<()> = SquareGrid::new_flat(value.size);
        let mapping = RectangularMapping::new(value.noise());
        let grid = flat.map(|p, _| {
            let value = mapping.get(p);
            HeightOnlyCell::new(value)
        });

        Self {
            grid,
            options: value,
        }
    }
}

impl Meshable for Terrain {
    type Output = Mesh;

    fn mesh(&self) -> Self::Output {
        self.grid.mesh()
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
    terrain: Terrain,
    pbr: PbrBundle,
}

#[derive(Bundle)]
pub struct WaterBundle {
    water: Water,
    pbr: PbrBundle,
}

fn water_animation() -> Tween<Transform> {
    Tween::new(
        EaseFunction::QuadraticInOut,
        Duration::from_secs(2),
        TransformPositionLens {
            start: Vec3::ZERO,
            end: Vec3::new(0.0, -0.02, 0.0),
        },
    )
    .with_repeat_count(RepeatCount::Infinite)
    .with_repeat_strategy(RepeatStrategy::MirroredRepeat)
}

fn generate_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let seed = TerrainSeed::default();
    let options = TerrainOptions::new(seed, UVec2::new(64, 64));
    let terrain: Terrain = options.into();
    let size = terrain.size().as_vec2();

    let terrain = TerrainBundle {
        pbr: PbrBundle {
            mesh: meshes.add(terrain.mesh()),
            material: materials.add(Color::rgb(1., 1., 1.)),
            ..Default::default()
        },
        terrain,
    };

    let water = WaterBundle {
        pbr: PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(size.x, size.y)),
            material: materials.add(Color::rgba(0., 0., 1., 0.95)),
            ..Default::default()
        },
        water: Water {},
    };

    commands.spawn((
        Name::new("Ground"),
        CollisionGroups::new(Group::all(), Group::all()),
        Collider::cuboid(size.x, 0.1, size.y),
        bevy_rts_camera::Ground,
        terrain,
    ));

    commands.spawn((
        Name::new("Water"),
        CollisionGroups::new(Group::all(), Group::all()),
        Collider::cuboid(size.x, 0.1, size.y),
        Animator::new(water_animation()),
        NoWireframe,
        water,
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
