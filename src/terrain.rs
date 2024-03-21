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

use super::model::Around;
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
    /// Resolution of the terrain noise, (2, 2) is a good default.
    resolution: UVec2,
}

impl TerrainOptions {
    fn new(seed: TerrainSeed, size: UVec2) -> Self {
        Self {
            seed,
            size,
            resolution: UVec2::splat(2),
        }
    }

    fn noise(&self) -> NoiseMap {
        let perlin = Perlin::new(self.seed.clone().into());

        let terraced: Terrace<_, _, 2> = Terrace::new(perlin)
            .add_control_point(-1.0)
            .add_control_point(-0.5)
            .add_control_point(0.1)
            .add_control_point(1.0)
            .invert_terraces(true);

        let size = self.size / self.resolution;

        PlaneMapBuilder::new(terraced)
            .set_size(size.x as usize, size.y as usize)
            .build()
    }
}

#[derive(Component, Debug)]
pub struct Water {}

#[derive(Component)]
pub struct Terrain {
    options: TerrainOptions,
    noise: NoiseMap,
}

impl Terrain {
    pub fn world_to_grid(&self, position: Vec3) -> Option<UVec2> {
        let local = position + self.world_to_local();
        let local = local.xz();

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
                let grid = GridView::new(&self.noise, self.noise.size().0);
                let around = grid.adjacent(index.as_ivec2());

                info!("{:#?}", around);

                None
            }
            None => None,
        }
    }

    fn size(&self) -> UVec2 {
        self.options.size
    }

    fn world_to_local(&self) -> Vec3 {
        let size = self.options.size.as_vec2();
        Vec3::new(size.x, 0., size.y) / 2.0
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
        Self {
            noise: value.noise(),
            options: value,
        }
    }
}

impl Meshable for Terrain {
    type Output = Mesh;

    fn mesh(&self) -> Self::Output {
        let resolution = self.options.resolution;
        let size = self.options.size + UVec2::ONE;
        let offset: Vec2 = (size - UVec2::ONE).as_vec2() * Vec2::splat(-0.5);

        info!("size={:?} offset={:?}", size, offset);

        let grid_noise: Vec<_> = (0..size.y)
            .flat_map(|r| {
                (0..size.x).map(move |c| {
                    let grid = UVec2::new(c, r);
                    let index = grid / resolution;
                    let value = self.noise[(index.x as usize, index.y as usize)];
                    (grid, value as f32)
                })
            })
            .collect();

        let positions: Vec<_> = grid_noise
            .iter()
            .map(|(grid, value)| {
                let p = grid.as_vec2() + offset;
                Vec3::new(p.x, *value * 2.0, p.y)
            })
            .collect();

        info!(
            "min={:?} max={:?}",
            positions.first(),
            positions.iter().last()
        );

        let uvs: Vec<_> = grid_noise
            .iter()
            .map(|(grid, _)| grid.as_vec2() / size.as_vec2())
            .collect();

        let colors: Vec<[f32; 4]> = grid_noise
            .iter()
            .map(|(_grid, value)| get_color(*value).as_rgba_f32())
            .collect();

        let normals: Vec<Vec3> = grid_noise.iter().map(|_| Vec3::Y).collect();

        let indices: Vec<_> = (0..size.y - 1)
            .flat_map(|r| {
                (0..size.x - 1).map(move |c| {
                    let i = r * size.x;
                    let l = (r + 1) * size.x;
                    vec![
                        i + c,     // This zips two rows of vertices together.
                        l + c,     // i is the top row
                        l + c + 1, // l is the one below
                        i + c,     //
                        l + c + 1, //
                        i + c + 1, //
                    ]
                })
            })
            .flatten()
            .collect();

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

pub struct GridView<'a, T> {
    width: usize,
    target: &'a T,
}

impl<'a, T> GridView<'a, T> {
    pub fn new(target: &'a T, width: usize) -> Self {
        Self { width, target }
    }
}

#[allow(dead_code)]
impl<'a, V> GridView<'a, Vec<V>> {
    fn get(&self, p: &IVec2) -> Option<&V> {
        let index = p.y * (self.width as i32) + p.x;
        self.target.get(index as usize)
    }

    pub fn adjacent(&self, center: IVec2) -> Around<Option<&V>> {
        Around::center(center).map(|xy| self.get(xy))
    }
}

impl<'a> GridView<'a, NoiseMap> {
    fn get(&self, p: &IVec2) -> Option<f64> {
        let size = self.target.size();

        if p.x >= 0 && p.y >= 0 && p.x < size.0 as i32 && p.y < size.1 as i32 {
            Some(self.target.get_value(p.x as usize, p.y as usize))
        } else {
            None
        }
    }

    pub fn adjacent(&self, center: IVec2) -> Around<Option<f64>> {
        Around::center(center).map(|xy| self.get(xy))
    }
}

#[test]
pub fn test_terrain_grid() {
    let options = TerrainOptions::new(default(), UVec2::new(8, 8));
    let terrain: Terrain = options.into();
    let size = terrain.size();

    println!("{:?}", terrain.noise.size());

    let noise: Vec<_> = terrain.noise.iter().collect();

    let rows: Vec<_> = noise.chunks(size.x as usize / 2).collect();

    println!("{:#?}", rows);

    let temp: Vec<_> = rows
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|value| [value, value])
                .flatten()
                .collect::<Vec<_>>()
        })
        .collect();

    println!("{:#?} {:?}", temp, noise.len());

    let v = GridView::new(&noise, terrain.noise.size().0);

    println!("{:#?}", v.adjacent(IVec2::new(0, 0)));
    println!("{:#?}", v.adjacent(IVec2::new(1, 0)));
    println!("{:#?}", v.adjacent(IVec2::new(2, 3)));
}

struct Grid<T> {
    size: (usize, usize),
    items: Vec<T>,
}

#[allow(dead_code)]
impl<T> Grid<T> {
    pub fn new(size: (usize, usize), items: Vec<T>) -> Self {
        assert!(size.0 * size.1 == items.len());
        Self { size, items }
    }

    ///
    /// If we start with this:
    ///
    /// A B C D
    /// E F G H
    /// I J K L
    ///
    /// expand((2, 2)) will produce this:
    ///
    /// A A B B C C D D
    /// A A B B C C D D
    /// E E F F G G H H
    /// E E F F G G H H
    /// I I J J K K L L
    /// I I J J K K L L
    ///
    pub fn expand_by(self, by: (usize, usize)) -> Self
    where
        T: Copy,
    {
        let size = (self.size.0 * by.0, self.size.1 * by.1);
        let items: Vec<_> = self
            .items
            .into_iter()
            .flat_map(|v| (0..by.0).map(move |_| v))
            .collect::<Vec<_>>()
            .chunks(size.0)
            .flat_map(|row| {
                let row: Vec<_> = row.to_vec();
                (0..by.1).map(move |_| row.clone())
            })
            .flatten()
            .collect();

        Self::new(size, items)
    }

    /// A B C D
    /// E F G H
    /// I J K L
    ///
    /// [ A A   [ A B   [ B B   [ B C   [ C C   [ C D   [ D D
    ///   A A ]   A B ]   B B ]   B C ]   C C ]   C D ]   D D ]
    ///
    /// [ A A   [ A B   [ B B   [ B C   [ C C   [ C D   [ D D
    ///   E E ]   E F ]   F F ]   F G ]   G G ]   G H ]   H H ]
    ///
    /// [ E E   [ E F   [ F F   [ F G   [ G G   [ G H   [ H H
    ///   E E ]   E F ]   F F ]   F G ]   G G ]   G H ]   H H ]
    ///
    /// [ E E   [ E F   [ F F   [ F G   [ G G   [ G H   [ H H
    ///   I I ]   I J ]   J J ]   J K ]   K K ]   K L ]   L L ]
    ///
    /// [ I I   [ I J   [ J J   [ J K   [ K K   [ K L   [ L L
    ///   I I ]   I J ]   J J ]   J K ]   K K ]   K L ]   L L ]
    ///
    /// etc
    ///
    /// A B
    /// C D
    ///
    /// A A B B
    /// A A B B
    /// C C D D
    /// C C D D
    ///
    /// AA AB BB
    /// AA AB BB
    ///
    /// AA AB BB
    /// CC CD DD
    ///
    /// CC CD DD
    /// CC CD DD
    ///
    pub fn expand(self) -> Grid<Vec<T>>
    where
        T: Copy + std::fmt::Debug,
    {
        let rows: Vec<Vec<_>> = self
            .items
            .chunks(self.size.0)
            .map(|row| row.iter().collect::<Vec<_>>())
            .collect();

        let items: Vec<Vec<Vec<T>>> = rows
            .into_iter()
            .map(|row| {
                row.windows(2)
                    .enumerate()
                    .flat_map(|(i, pair)| {
                        if i == 0 {
                            vec![
                                vec![*pair[0], *pair[0]],
                                vec![*pair[0], *pair[1]],
                                vec![*pair[1], *pair[1]],
                            ]
                        } else {
                            vec![vec![*pair[0], *pair[1]], vec![*pair[1], *pair[1]]]
                        }
                    })
                    .collect::<Vec<Vec<T>>>()
            })
            .collect();

        let items: Vec<Vec<T>> = items
            .windows(2)
            .enumerate()
            .flat_map(|(i, pair)| {
                let r0: Vec<Vec<_>> = pair[0]
                    .clone()
                    .into_iter()
                    .map(|pair| vec![pair.clone(), pair].into_iter().flatten().collect())
                    .collect();
                let r2: Vec<Vec<_>> = pair[1]
                    .clone()
                    .into_iter()
                    .map(|pair| vec![pair.clone(), pair].into_iter().flatten().collect())
                    .collect();
                let r1: Vec<Vec<_>> = pair[0]
                    .clone()
                    .into_iter()
                    .zip(pair[1].clone())
                    .map(|(t, b)| vec![t, b].into_iter().flatten().collect())
                    .collect();

                if i == 0 {
                    vec![r0, r1, r2]
                } else {
                    vec![r1, r2]
                }
            })
            .flatten()
            .collect();

        let size = (self.size.0 * 2 - 1, self.size.1 * 2 - 1);

        Grid::new(size, items)
    }
}

impl<T: PartialEq> PartialEq for Grid<T> {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.items == other.items
    }
}

impl<T: Clone> Clone for Grid<T> {
    fn clone(&self) -> Self {
        Self {
            size: self.size,
            items: self.items.clone(),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Grid<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grid")
            .field("size", &self.size)
            .field("items", &self.items)
            .finish()
    }
}

#[test]
pub fn test_grid_4x4_expand() {
    let grid = Grid::new((4, 4), (1..17).into_iter().collect());

    assert_eq!(
        grid.clone().expand(),
        Grid::new(
            (7, 7),
            vec![
                vec![1, 1, 1, 1],
                vec![1, 2, 1, 2],
                vec![2, 2, 2, 2],
                vec![2, 3, 2, 3],
                vec![3, 3, 3, 3],
                vec![3, 4, 3, 4],
                vec![4, 4, 4, 4],
                //
                vec![1, 1, 5, 5],
                vec![1, 2, 5, 6],
                vec![2, 2, 6, 6],
                vec![2, 3, 6, 7],
                vec![3, 3, 7, 7],
                vec![3, 4, 7, 8],
                vec![4, 4, 8, 8],
                //
                vec![5, 5, 5, 5],
                vec![5, 6, 5, 6],
                vec![6, 6, 6, 6],
                vec![6, 7, 6, 7],
                vec![7, 7, 7, 7],
                vec![7, 8, 7, 8],
                vec![8, 8, 8, 8],
                //
                vec![5, 5, 9, 9],
                vec![5, 6, 9, 10],
                vec![6, 6, 10, 10],
                vec![6, 7, 10, 11],
                vec![7, 7, 11, 11],
                vec![7, 8, 11, 12],
                vec![8, 8, 12, 12],
                //
                vec![9, 9, 9, 9],
                vec![9, 10, 9, 10],
                vec![10, 10, 10, 10],
                vec![10, 11, 10, 11],
                vec![11, 11, 11, 11],
                vec![11, 12, 11, 12],
                vec![12, 12, 12, 12],
                //
                vec![9, 9, 13, 13],
                vec![9, 10, 13, 14],
                vec![10, 10, 14, 14],
                vec![10, 11, 14, 15],
                vec![11, 11, 15, 15],
                vec![11, 12, 15, 16],
                vec![12, 12, 16, 16],
                //
                vec![13, 13, 13, 13],
                vec![13, 14, 13, 14],
                vec![14, 14, 14, 14],
                vec![14, 15, 14, 15],
                vec![15, 15, 15, 15],
                vec![15, 16, 15, 16],
                vec![16, 16, 16, 16],
            ]
        )
    );
}

#[test]
pub fn test_grid_2x2_expand() {
    let grid = Grid::new((2, 2), vec![1, 2, 3, 4]);

    assert_eq!(
        grid.clone().expand(),
        Grid::new(
            (3, 3),
            vec![
                vec![1, 1, 1, 1],
                vec![1, 2, 1, 2],
                vec![2, 2, 2, 2],
                vec![1, 1, 3, 3],
                vec![1, 2, 3, 4],
                vec![2, 2, 4, 4],
                vec![3, 3, 3, 3],
                vec![3, 4, 3, 4],
                vec![4, 4, 4, 4]
            ]
        )
    );

    assert_eq!(
        grid.expand_by((2, 2)),
        Grid::new(
            (4, 4),
            vec![
                1, 1, 2, 2, //
                1, 1, 2, 2, //
                3, 3, 4, 4, //
                3, 3, 4, 4, //
            ]
        )
    );
}
