use bevy::{
    pbr::wireframe::NoWireframe,
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
use std::time::Duration;

// use crate::AroundCenter;

use crate::TILE_SIZE;

// use super::model::Grid;
use super::model::Seed;

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
    /// Size of the terrain in grid tiles. This means that there will be an
    /// extra row and column of vertices in the generated mesh.
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
                info!("{:#?}", index.as_ivec2());

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

impl std::fmt::Debug for Terrain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Terrain")
            .field("options", &self.options)
            .finish()
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

pub struct SquareGrid<T> {
    size: UVec2,
    cells: Vec<T>,
}

impl<T> SquareGrid<T> {
    pub fn new(size: UVec2, cells: Vec<T>) -> Self {
        assert!((size.x * size.y) as usize == cells.len());
        Self { size, cells }
    }

    fn local_to_world(&self) -> Vec3 {
        -self.world_to_local()
    }

    fn world_to_local(&self) -> Vec3 {
        let size = self.size.as_vec2();
        (Vec3::new(size.x, 0., size.y) / 2.0) - (Vec3::ONE / 2.0)
    }

    pub fn into_cells(self) -> Vec<T> {
        self.cells
    }

    pub fn apply<V>(&self, mut map_fn: impl FnMut(UVec2, &T) -> V) -> SquareGrid<V> {
        let cells = self
            .cells
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let x = index as u32 % self.size.x;
                let y = index as u32 / self.size.x;
                map_fn(UVec2::new(x, y), value)
            })
            .collect();

        SquareGrid::new(self.size, cells)
    }

    pub fn map<V>(self, mut map_fn: impl FnMut(UVec2, T) -> V) -> SquareGrid<V> {
        let cells = self
            .cells
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                let x = index as u32 % self.size.x;
                let y = index as u32 / self.size.x;
                map_fn(UVec2::new(x, y), value)
            })
            .collect();

        SquareGrid::new(self.size, cells)
    }
}

impl<T> SquareGrid<T>
where
    T: Default + Clone,
{
    pub fn new_flat(size: UVec2) -> Self {
        Self::new(size, vec![T::default(); (size.x * size.y) as usize])
    }
}

impl<T: Clone> Clone for SquareGrid<T> {
    fn clone(&self) -> Self {
        Self {
            size: self.size.clone(),
            cells: self.cells.clone(),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for SquareGrid<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SquareGrid")
            .field("size", &self.size)
            .field("cells", &self.cells)
            .finish()
    }
}

#[derive(Debug)]
pub struct HeightOnlyCell([f64; 4]);

impl HeightOnlyCell {
    pub fn new(value: [f64; 4]) -> Self {
        Self(value)
    }
}

impl Meshable for HeightOnlyCell {
    type Output = Mesh;

    fn mesh(&self) -> Self::Output {
        let half_size = Vec2::splat(TILE_SIZE) / 2.0;
        let rotation = Quat::from_rotation_arc(Vec3::Y, Vec3::Y);
        let positions = vec![
            rotation * Vec3::new(-half_size.x, self.0[0] as f32, -half_size.y),
            rotation * Vec3::new(-half_size.x, self.0[2] as f32, half_size.y),
            rotation * Vec3::new(half_size.x, self.0[3] as f32, half_size.y),
            rotation * Vec3::new(half_size.x, self.0[1] as f32, -half_size.y),
        ];

        let normals = vec![Vec3::Y.to_array(); 4];
        let uvs = vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3]);

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_COLOR,
            vec![
                get_color(self.0[0] as f32).as_rgba_f32(),
                get_color(self.0[2] as f32).as_rgba_f32(),
                get_color(self.0[3] as f32).as_rgba_f32(),
                get_color(self.0[1] as f32).as_rgba_f32(),
            ],
        )
        .with_inserted_indices(indices)
    }
}

impl<T> Meshable for SquareGrid<T>
where
    T: Meshable<Output = Mesh>,
{
    type Output = Mesh;

    fn mesh(&self) -> Self::Output {
        let all = self.local_to_world();

        let meshes = self
            .apply(|p, cell| {
                let local = Vec3::new(p.x as f32, 0.0, p.y as f32) + all;
                cell.mesh().translated_by(local)
            })
            .into_cells();

        let empty = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<Vec3>::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<Vec3>::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<Vec2>::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<Vec4>::default())
        .with_inserted_indices(Indices::U32(Default::default()));

        meshes.into_iter().fold(empty, |mut all, m| {
            all.merge(m);
            all
        })
    }
}

/// Maps values from 2-dimensional structures to 4 array values based on the
/// surrounding values of the coordinate. Specifically such that odd coordinates
/// include adjacent values from the original, and even coordinates include the
/// same values. I've agonized over the name of this and how to better approach
/// this problem. I'm hoping the test goes a long way to explaining what's
/// happening.
pub struct RectangularMapping<T> {
    map: T,
}

impl<T> RectangularMapping<T> {
    fn new(map: T) -> Self {
        Self { map }
    }

    fn map_coordinates(&self, p: UVec2) -> (UVec2, UVec2, UVec2, UVec2) {
        let x = p.x;
        let y = p.y;

        match (x % 2 == 0, y % 2 == 0) {
            (true, false) => (
                UVec2::new(x / 2, (y - 1) / 2),
                UVec2::new(x / 2, (y - 1) / 2),
                UVec2::new(x / 2, (y - 1) / 2 + 1),
                UVec2::new(x / 2, (y - 1) / 2 + 1),
            ),
            (false, true) => (
                UVec2::new((x - 1) / 2, y / 2),
                UVec2::new((x - 1) / 2 + 1, y / 2),
                UVec2::new((x - 1) / 2, y / 2),
                UVec2::new((x - 1) / 2 + 1, y / 2),
            ),
            (false, false) => (
                UVec2::new((x - 1) / 2, (y - 1) / 2),
                UVec2::new((x - 1) / 2 + 1, (y - 1) / 2),
                UVec2::new((x - 1) / 2, (y - 1) / 2 + 1),
                UVec2::new((x - 1) / 2 + 1, (y - 1) / 2 + 1),
            ),
            (true, true) => {
                let idx = UVec2::new(x / 2, y / 2);
                (idx, idx, idx, idx)
            }
        }
    }
}

#[allow(dead_code)]
impl<T> RectangularMapping<Vec<Vec<T>>>
where
    T: Default + Copy,
{
    fn get(&self, p: UVec2) -> [T; 4] {
        let (c0, c1, c2, c3) = self.map_coordinates(p);

        [
            self.map[c0.y as usize][c0.x as usize],
            self.map[c1.y as usize][c1.x as usize],
            self.map[c2.y as usize][c2.x as usize],
            self.map[c3.y as usize][c3.x as usize],
        ]
    }
}

impl RectangularMapping<NoiseMap> {
    fn get(&self, p: UVec2) -> [f64; 4] {
        let (c0, c1, c2, c3) = self.map_coordinates(p);

        [
            self.map.get_value(c0.x as usize, c0.y as usize),
            self.map.get_value(c1.x as usize, c1.y as usize),
            self.map.get_value(c2.x as usize, c2.y as usize),
            self.map.get_value(c3.x as usize, c3.y as usize),
        ]
    }
}

#[test]
fn test_rectangular_mapping_map_coordinates() {
    // [ 0,  1,  2,  3,  4,  5]
    // [ 6,  7,  8,  9, 10, 11]
    // [12, 13, 14, 15, 16, 17]
    // [18, 19, 20, 21, 22, 23]
    // [24, 25, 26, 27, 28, 29]
    // [30, 31, 32, 33, 34, 35]
    let data = (0..6)
        .into_iter()
        .map(|row| ((row * 6)..((row + 1) * 6)).into_iter().collect::<Vec<_>>())
        .collect::<Vec<_>>();

    let map = RectangularMapping::new(data);
    assert_eq!(
        map.map_coordinates(UVec2::new(0, 0)),
        (
            UVec2::new(0, 0),
            UVec2::new(0, 0),
            UVec2::new(0, 0),
            UVec2::new(0, 0)
        )
    );
    assert_eq!(
        map.map_coordinates(UVec2::new(1, 1)),
        (
            UVec2::new(0, 0),
            UVec2::new(1, 0),
            UVec2::new(0, 1),
            UVec2::new(1, 1)
        )
    );
    assert_eq!(
        map.map_coordinates(UVec2::new(0, 2)),
        (
            UVec2::new(0, 1),
            UVec2::new(0, 1),
            UVec2::new(0, 1),
            UVec2::new(0, 1)
        )
    );
    assert_eq!(
        map.map_coordinates(UVec2::new(5, 5)),
        (
            UVec2::new(2, 2),
            UVec2::new(3, 2),
            UVec2::new(2, 3),
            UVec2::new(3, 3)
        )
    );
}

#[test]
fn test_rectangular_mapping_map_vec_vec() {
    // [ 0,  1,  2,  3,  4,  5]
    // [ 6,  7,  8,  9, 10, 11]
    // [12, 13, 14, 15, 16, 17]
    // [18, 19, 20, 21, 22, 23]
    // [24, 25, 26, 27, 28, 29]
    // [30, 31, 32, 33, 34, 35]
    let data = (0..6)
        .into_iter()
        .map(|row| ((row * 6)..((row + 1) * 6)).into_iter().collect::<Vec<_>>())
        .collect::<Vec<_>>();

    let map = RectangularMapping::new(data);
    assert_eq!(map.get(UVec2::new(0, 0)), [0, 0, 0, 0]);
    assert_eq!(map.get(UVec2::new(1, 1)), [0, 1, 6, 7]);
    assert_eq!(map.get(UVec2::new(1, 0)), [0, 1, 0, 1]);
    assert_eq!(map.get(UVec2::new(2, 0)), [1, 1, 1, 1]);
    assert_eq!(map.get(UVec2::new(3, 0)), [1, 2, 1, 2]);
    assert_eq!(map.get(UVec2::new(4, 0)), [2, 2, 2, 2]);
    assert_eq!(map.get(UVec2::new(5, 0)), [2, 3, 2, 3]);
    assert_eq!(map.get(UVec2::new(0, 1)), [0, 0, 6, 6]);
    assert_eq!(map.get(UVec2::new(0, 2)), [6, 6, 6, 6]);
    assert_eq!(map.get(UVec2::new(0, 3)), [6, 6, 12, 12]);
    assert_eq!(map.get(UVec2::new(0, 4)), [12, 12, 12, 12]);
    assert_eq!(map.get(UVec2::new(0, 5)), [12, 12, 18, 18]);
    assert_eq!(map.get(UVec2::new(5, 5)), [14, 15, 20, 21]);
}

#[test]
pub fn test_terrain_grid() {
    let options = TerrainOptions::new(default(), UVec2::new(4, 4));
    let flat: SquareGrid<()> = SquareGrid::new_flat(options.size);
    let mapping = RectangularMapping::new(options.noise());
    let deformed = flat.map(|p, _| {
        let value = mapping.get(p);
        HeightOnlyCell::new(value)
    });

    println!("{:#?}", deformed);
}
