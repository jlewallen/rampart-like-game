use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
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
}

impl Meshable for HeightOnlyCell {
    type Output = Mesh;

    fn mesh(&self) -> Self::Output {
        let half_size = Vec2::splat(TILE_SIZE) / 2.0;
        let rotation = Quat::from_rotation_arc(Vec3::Y, Vec3::Y);
        let positions = vec![
            rotation * Vec3::new(-half_size.x, self.0[0] as f32 * HEIGHT_SCALE, -half_size.y),
            rotation * Vec3::new(-half_size.x, self.0[2] as f32 * HEIGHT_SCALE, half_size.y),
            rotation * Vec3::new(half_size.x, self.0[3] as f32 * HEIGHT_SCALE, half_size.y),
            rotation * Vec3::new(half_size.x, self.0[1] as f32 * HEIGHT_SCALE, -half_size.y),
        ];

        let normals = vec![Vec3::Y.to_array(); 4];
        let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3]);
        let uvs = vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];

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
    pub fn new(map: T) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
