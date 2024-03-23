use std::ops::Index;

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
        render_asset::RenderAssetUsages,
    },
};
use noise::utils::NoiseMap;

use crate::{model::SquareGrid, model::HEIGHT_SCALE, model::TILE_SIZE};

#[derive(Debug, Clone)]
pub struct HeightOnlyCell([f64; 4]);

impl HeightOnlyCell {
    pub fn new(value: [f64; 4]) -> Self {
        Self(value)
    }

    pub fn iter(&self) -> impl Iterator<Item = &f64> {
        self.0.iter()
    }

    pub fn world_y(&self) -> Vec3 {
        Vec3::new(
            0.,
            *self
                .0
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap() as f32,
            0.,
        )
    }

    pub fn interpolate(&self, idx: UVec2, size: UVec2) -> f64 {
        let r1 = (size.x - idx.x) as f64 / (size.x as f64) * self.0[0]
            + (idx.x as f64 / size.x as f64) * self.0[1];

        let r2 = (size.x - idx.x) as f64 / (size.x as f64) * self.0[2]
            + (idx.x as f64 / size.x as f64) * self.0[3];

        ((size.y - idx.y) as f64 / size.y as f64) * r1 + (idx.y as f64 / size.y as f64) * r2
    }
}

impl Index<usize> for HeightOnlyCell {
    type Output = f64;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl Meshable for HeightOnlyCell {
    type Output = Mesh;

    fn mesh(&self) -> Self::Output {
        let half_size = Vec2::splat(TILE_SIZE) / 2.0;
        let rotation = Quat::from_rotation_arc(Vec3::Y, Vec3::Y);
        let positions = vec![
            rotation * Vec3::new(half_size.x, self.0[1] as f32 * HEIGHT_SCALE, -half_size.y),
            rotation * Vec3::new(-half_size.x, self.0[0] as f32 * HEIGHT_SCALE, -half_size.y),
            rotation * Vec3::new(-half_size.x, self.0[2] as f32 * HEIGHT_SCALE, half_size.y),
            rotation * Vec3::new(half_size.x, self.0[3] as f32 * HEIGHT_SCALE, half_size.y),
        ];

        let normals = vec![Vec3::Y.to_array(); 4];
        let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3]);
        let uvs = vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_COLOR,
            if false {
                vec![
                    get_color(self.0[0] as f32).as_rgba_f32(),
                    get_color(self.0[2] as f32).as_rgba_f32(),
                    get_color(self.0[3] as f32).as_rgba_f32(),
                    get_color(self.0[1] as f32).as_rgba_f32(),
                ]
            } else {
                vec![
                    Color::WHITE.as_rgba_f32(),
                    Color::WHITE.as_rgba_f32(),
                    Color::WHITE.as_rgba_f32(),
                    Color::WHITE.as_rgba_f32(),
                ]
            },
        )
        .with_inserted_indices(indices)
    }
}

struct MeshModifier {
    mesh: Mesh,
}
impl MeshModifier {
    fn new(mesh: Mesh) -> Self {
        Self { mesh }
    }

    fn translated_by(self, v: Vec3) -> Self {
        Self {
            mesh: self.mesh.translated_by(v),
        }
    }

    fn uvs_scaled_by(mut self, v: Vec2) -> Self {
        let uvs = self.mesh.remove_attribute(Mesh::ATTRIBUTE_UV_0).unwrap();

        match uvs {
            VertexAttributeValues::Float32x2(mut uvs) => {
                for uv in uvs.iter_mut() {
                    uv[0] *= v.x;
                    uv[1] *= v.y;
                }

                Self {
                    mesh: self.mesh.with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs),
                }
            }
            _ => todo!(),
        }
    }

    fn uvs_translated_by(mut self, v: Vec2) -> Self {
        let uvs = self.mesh.remove_attribute(Mesh::ATTRIBUTE_UV_0).unwrap();

        match uvs {
            VertexAttributeValues::Float32x2(mut uvs) => {
                for uv in uvs.iter_mut() {
                    uv[0] += v.x;
                    uv[1] += v.y;
                }

                Self {
                    mesh: self.mesh.with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs),
                }
            }
            _ => todo!(),
        }
    }

    fn into_inner(self) -> Mesh {
        self.mesh
    }
}

impl<T> Meshable for SquareGrid<T>
where
    T: Meshable<Output = Mesh>,
{
    type Output = Mesh;

    fn mesh(&self) -> Self::Output {
        let all = self.local_to_world();

        let size = self.size().as_vec2();
        let uv_scale = 1. / size;

        let meshes = self
            .apply(|p, cell| {
                MeshModifier::new(cell.mesh())
                    .translated_by(
                        Vec3::new(p.x as f32, 0.0, p.y as f32) * Vec3::splat(TILE_SIZE) + all,
                    )
                    .uvs_scaled_by(uv_scale)
                    .uvs_translated_by(Vec2::new(p.x as f32, p.y as f32) / size)
                    .into_inner()
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
    pub fn new(map: T) -> Self {
        Self { map }
    }

    pub(super) fn map_coordinates(&self, p: UVec2) -> (UVec2, UVec2, UVec2, UVec2) {
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
    pub fn get(&self, p: UVec2) -> [T; 4] {
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
    pub fn get(&self, p: UVec2) -> [f64; 4] {
        let (c0, c1, c2, c3) = self.map_coordinates(p);

        [
            self.map.get_value(c0.x as usize, c0.y as usize),
            self.map.get_value(c1.x as usize, c1.y as usize),
            self.map.get_value(c2.x as usize, c2.y as usize),
            self.map.get_value(c3.x as usize, c3.y as usize),
        ]
    }
}

fn get_color(val: f32) -> Color {
    let color = match val.abs() {
        v if v < 0.1 => Color::hex("#0a7e0a"),
        v if v < 0.2 => Color::hex("#0da50d"),
        v if v < 0.3 => Color::hex("#10cb10"),
        v if v < 0.4 => Color::hex("#18ed18"),
        // v if v < 0.5 => Color::hex("#3ff03f"),
        // v if v < 0.6 => Color::hex("#65f365"),
        // v if v < 0.7 => Color::hex("#8cf68c"),
        // v if v < 0.8 => Color::hex("#b2f9b2"),
        // v if v < 0.9 => Color::hex("#d9fcd9"),
        // v if v <= 1.0 => Color::hex("#ffffff"),
        v if v <= 1.0 => Color::hex("#18ed18"),
        _ => panic!("unexpected value"),
    };
    color.expect("bad color")
}

/*
#[test]
pub fn test_mesh_modifier() {
    let grid: SquareGrid<HeightOnlyCell> = SquareGrid::new_flat(UVec2::new(2, 2));

    println!("{:#?}", grid.mesh());
}
*/
