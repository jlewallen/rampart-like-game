use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
};
use bevy_rapier3d::prelude::*;
use noise::{
    utils::{NoiseMap, NoiseMapBuilder, PlaneMapBuilder},
    Perlin, Terrace,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seed<T>(T);

impl<T: Default> Default for Seed<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(Component)]
pub struct TerrainSeed {
    size: (usize, usize),
    seed: Seed<u32>,
}

impl TerrainSeed {
    pub fn new(size: (usize, usize), seed: Seed<u32>) -> Self {
        Self { size, seed }
    }

    fn noise_map(&self) -> NoiseMap {
        let perlin = Perlin::new(self.seed.0);

        let terrace_inverted: Terrace<_, _, 2> = Terrace::new(perlin)
            .add_control_point(-1.0)
            .add_control_point(-0.5)
            .add_control_point(0.1)
            .add_control_point(1.0)
            .invert_terraces(true);

        PlaneMapBuilder::new(terrace_inverted)
            .set_size(self.size.0, self.size.1)
            .build()
    }
}

impl Meshable for TerrainSeed {
    type Output = Mesh;

    fn mesh(&self) -> Self::Output {
        let map = self.noise_map();
        let (w, h) = map.size();
        let scale: Vec2 = Vec2::new(1.0, 1.0);
        let offset: Vec2 = Vec2::new(w as f32 / -2., h as f32 / -2.);

        /*
        CCW Order

        0   1
        2   3

        0,2,3
        0,3,1

        0  1  2
        3  4  5
        6  7  8

        0,3,4
        0,4,1
        1,4,5
        1,5,2
        3,6,7
        3,7,4
        4,7,8
        4,8,5
        */

        let normals: Vec<Vec3> = map.iter().map(|_| Vec3::new(0., 1., 0.)).collect();

        // Note: (0.0, 0.0) = Top-Left in UV mapping, (1.0, 1.0) = Bottom-Right in UV mapping
        let uvs: Vec<_> = map
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let i = (index % w) as f32 / (w as f32);
                let j = (index / w) as f32 / (h as f32);
                Vec2::new(i, j)
            })
            .collect();

        let colors: Vec<[f32; 4]> = map
            .iter()
            .map(|value| get_color(*value).as_rgba_f32())
            .collect();

        let positions: Vec<_> = map
            .into_iter()
            .enumerate()
            .map(|(i, value)| {
                let x = ((i % w) as f32 + offset.x) * scale.x;
                let y = ((i / w) as f32 + offset.y) * scale.y;
                Vec3::new(x, value as f32, y)
            })
            .collect();

        let mut indices = Vec::new();
        for r in 0..(h - 1) {
            for c in 0..(w - 1) {
                let i = r * w;
                let l = (r + 1) * w;
                indices.push(i + c);
                indices.push(l + c);
                indices.push(l + c + 1);
                indices.push(i + c);
                indices.push(l + c + 1);
                indices.push(i + c + 1);
            }
        }

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(Indices::U32(
            indices.into_iter().map(|v| v as u32).collect(),
        ))
    }
}

fn get_color(val: f64) -> Color {
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

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_terrain);
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
    let seed = TerrainSeed::new((32, 32), Seed::default());

    let terrain = TerrainBundle {
        pbr: PbrBundle {
            mesh: meshes.add(seed.mesh()),
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
    commands.spawn((
        Name::new("Water"),
        water,
        CollisionGroups::new(Group::all(), Group::all()),
        Collider::cuboid(20., 0.1, 20.),
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
