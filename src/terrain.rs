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
#[cfg(test)]
mod tests;
mod textures;

use super::helpers::GamePlayLifetime;
use super::model::{AppState, AroundCenter, Seed, Settings, SquareGrid, TILE_SIZE};

use mesh::{HeightOnlyCell, RectangularMapping};

#[derive(Clone, Default, Debug)]
struct TerrainSeed {
    seed: Seed<u32>,
}

impl TerrainSeed {
    pub fn new(seed: Seed<u32>) -> Self {
        Self { seed }
    }

    fn into(self) -> u32 {
        self.seed.into()
    }
}

#[derive(Debug, Clone)]
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

        // Yes, this generates more noise than we'll use.
        PlaneMapBuilder::new(terraced)
            .set_size(self.size.x as usize, self.size.y as usize)
            .build()
    }
}

#[derive(Component, Debug)]
struct Water {}

#[derive(Component)]
pub struct Terrain {
    options: TerrainOptions,
    grid: SquareGrid<HeightOnlyCell>,
}

impl Terrain {
    pub fn world_to_grid(&self, position: Vec3) -> Option<UVec2> {
        let local = position + self.grid.world_to_local() + (TILE_SIZE / 2.0);
        let local = local.xz();

        if false {
            info!(
                "world-to-local={:?} position={:?} local={:?}",
                self.grid.world_to_local(),
                position,
                local
            );
        }

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
                let around = self.grid.around(index);
                around.center().clone().map(|v| Survey {
                    world: self.grid.grid_to_world(index) + v.world_y(),
                    location: index,
                    cell: v.into(),
                })
            }
            None => None,
        }
    }

    #[allow(dead_code)]
    fn size(&self) -> UVec2 {
        self.options.size
    }

    fn bounds(&self) -> Vec2 {
        self.options.size.as_vec2() * Vec2::splat(TILE_SIZE)
    }

    fn grid(&self) -> &SquareGrid<HeightOnlyCell> {
        &self.grid
    }
}

#[derive(Debug)]
pub struct Survey {
    world: Vec3,
    location: IVec2,
    cell: SurveyedCell,
}

impl Survey {
    pub fn world(&self) -> Vec3 {
        self.world
    }

    pub fn location(&self) -> IVec2 {
        self.location
    }

    pub fn cell(&self) -> &SurveyedCell {
        &self.cell
    }
}

#[derive(Debug)]
pub enum SurveyedCell {
    Ground(HeightOnlyCell),
    Beach,
    Water,
}

impl From<HeightOnlyCell> for SurveyedCell {
    fn from(value: HeightOnlyCell) -> Self {
        let all_below_0 = value.iter().all(|v| *v < 0.);
        let any_below_0 = value.iter().any(|v| *v < 0.);
        if all_below_0 {
            SurveyedCell::Water
        } else if any_below_0 {
            SurveyedCell::Beach
        } else {
            SurveyedCell::Ground(value)
        }
    }
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

#[derive(Bundle)]
struct TileBundle {
    pbr: PbrBundle,
}

impl TileBundle {
    fn new(
        position: Vec3,
        cell: &HeightOnlyCell,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
    ) -> Self {
        let mesh = cell.mesh();

        Self {
            pbr: PbrBundle {
                mesh: meshes.add(mesh),
                material: materials.add(Color::rgb(1., 1., 1.)),
                transform: Transform::from_translation(position),
                ..default()
            },
        }
    }
}

#[derive(Bundle)]
struct CombinedTerrainMeshBundle {
    pbr: PbrBundle,
}

impl CombinedTerrainMeshBundle {
    fn new(
        mesh: Mesh,
        texture: Image,
        meshes: &mut ResMut<Assets<Mesh>>,
        images: &mut ResMut<Assets<Image>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
    ) -> Self {
        Self {
            pbr: PbrBundle {
                mesh: meshes.add(mesh),
                material: materials.add(StandardMaterial {
                    base_color: Color::rgb(1., 1., 1.),
                    base_color_texture: Some(images.add(texture)),
                    ..default()
                }),
                ..default()
            },
        }
    }
}

#[derive(Bundle)]
struct TerrainBundle {
    name: Name,
    lifetime: GamePlayLifetime,
    terrain: Terrain,
    ground: bevy_rts_camera::Ground,
    collision_groups: CollisionGroups,
    collider: Collider,
    ivis: InheritedVisibility,
    transform: GlobalTransform,
}

impl TerrainBundle {
    fn new(terrain: Terrain, mesh: &Mesh) -> Self {
        let collider = Collider::from_bevy_mesh(mesh, &ComputedColliderShape::ConvexHull)
            .expect("terrain collider error");

        Self {
            name: Name::new("Terrain"),
            lifetime: GamePlayLifetime,
            terrain,
            collider,
            collision_groups: CollisionGroups::new(Group::all(), Group::all()),
            ground: bevy_rts_camera::Ground,
            ivis: InheritedVisibility::default(),
            transform: GlobalTransform::default(),
        }
    }
}

#[derive(Bundle)]
struct WaterBundle {
    name: Name,
    lifetime: GamePlayLifetime,
    water: Water,
    pbr: PbrBundle,
    collider: Collider,
    collision_groups: CollisionGroups,
    wireframe: NoWireframe,
    animator: Animator<Transform>,
}

impl WaterBundle {
    fn new(
        bounds: Vec2,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
    ) -> Self {
        Self {
            name: Name::new("Water"),
            lifetime: GamePlayLifetime,
            water: Water {},
            pbr: PbrBundle {
                mesh: meshes.add(Plane3d::default().mesh().size(bounds.x, bounds.y)),
                material: materials.add(Color::rgba(0., 0., 1., 0.85)), // TODO WATER_COLOR
                transform: Transform::from_xyz(0.0, -0.5, 0.0),
                ..Default::default()
            },
            animator: Animator::new(WaterBundle::animation()),
            wireframe: NoWireframe,
            collision_groups: CollisionGroups::new(Group::all(), Group::all()),
            collider: Collider::cuboid(bounds.x, 0.5, bounds.y),
        }
    }

    fn animation() -> Tween<Transform> {
        Tween::new(
            EaseFunction::QuadraticInOut,
            Duration::from_secs(2),
            TransformPositionLens {
                start: Vec3::ZERO,
                end: Vec3::new(0.0, -0.01, 0.0),
            },
        )
        .with_repeat_count(RepeatCount::Infinite)
        .with_repeat_strategy(RepeatStrategy::MirroredRepeat)
    }
}

#[derive(Bundle)]
struct SunBundle {
    name: Name,
    lifetime: GamePlayLifetime,
    light: DirectionalLightBundle,
}

impl SunBundle {
    fn new() -> Self {
        Self {
            name: Name::new("Sun"),
            lifetime: GamePlayLifetime,
            light: DirectionalLightBundle {
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
        }
    }
}

fn generate_terrain(
    settings: Res<Settings>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let options = TerrainOptions::new(TerrainSeed::new(settings.seed()), settings.size());
    let terrain: Terrain = options.into();
    let bounds = terrain.bounds();

    let mesh = terrain.mesh();

    if false {
        let all = terrain.grid.local_to_world();
        let tiles = terrain
            .grid
            .apply(|p, cell| {
                let local = Vec3::new(p.x as f32, 0.0, p.y as f32) * Vec3::splat(TILE_SIZE) + all;
                TileBundle::new(local, cell, &mut meshes, &mut materials)
            })
            .into_cells();

        commands
            .spawn(TerrainBundle::new(terrain, &mesh))
            .with_children(|p| {
                for tile in tiles.into_iter() {
                    p.spawn(tile);
                }
            });
    } else {
        let texture =
            textures::TerrainTextureBuilder::new(terrain.grid(), UVec2::splat(32)).build();

        commands
            .spawn(TerrainBundle::new(terrain, &mesh))
            .with_children(|p| {
                p.spawn(CombinedTerrainMeshBundle::new(
                    mesh,
                    texture,
                    &mut meshes,
                    &mut images,
                    &mut materials,
                ));
            });
    }

    commands.spawn(WaterBundle::new(bounds, &mut meshes, &mut materials));

    commands.spawn(SunBundle::new());
}

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Game), generate_terrain)
            .add_systems(
                Update,
                component_animator_system::<Water>
                    .in_set(AnimationSystem::AnimationUpdate)
                    .run_if(in_state(AppState::Game)),
            );
    }
}
