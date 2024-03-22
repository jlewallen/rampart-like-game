use bevy::math::*;
use noise::utils::NoiseMap;

pub struct Grid<T> {
    size: (usize, usize),
    items: Vec<T>,
}

#[allow(dead_code)]
impl<T> Grid<T> {
    pub fn new(size: (usize, usize), items: Vec<T>) -> Self {
        assert!(size.0 * size.1 == items.len());
        Self { size, items }
    }

    pub fn size_uvec2(&self) -> UVec2 {
        UVec2::new(self.size.0 as u32, self.size.1 as u32)
    }

    pub fn from_rows(rows: impl Iterator<Item = Vec<T>>) -> Self {
        let rows: Vec<Vec<_>> = rows.collect();
        // If any of the rows have a different width then the size check in
        // `new` will fail, so I think this is fine.
        let width = rows[0].len();
        let size = (width, rows.len());
        let items = rows.into_iter().flatten().collect();
        Self::new(size, items)
    }

    pub fn rows(self) -> Vec<Vec<T>> {
        use itertools::Itertools;

        self.items
            .into_iter()
            .chunks(self.size.0)
            .into_iter()
            .map(|chunk| chunk.collect())
            .collect()
    }

    pub fn grow(self) -> Self
    where
        T: Default + Clone,
    {
        let new_width = self.size.0 + 1;
        let rows = self
            .rows()
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .chain(std::iter::once(T::default()))
                    .collect()
            })
            .chain(std::iter::once(vec![T::default(); new_width]));
        Self::from_rows(rows)
    }

    pub fn size(&self) -> (usize, usize) {
        self.size
    }

    pub fn items(&self) -> &Vec<T> {
        &self.items
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, usize, &T)> {
        self.items.iter().enumerate().map(|(index, value)| {
            let x = index % self.size.0;
            let y = index / self.size.0;
            (x, y, value)
        })
    }

    /// Duplicates grid cells, producing a new Grid that's larger by the given
    /// factors. The size of this new grid will be multiplied by `factor`.
    /// For example, if we start with this:
    ///
    /// A B C D
    /// E F G H
    /// I J K L
    ///
    /// duplicate((2, 2)) will produce this:
    ///
    /// A A B B C C D D
    /// A A B B C C D D
    /// E E F F G G H H
    /// E E F F G G H H
    /// I I J J K K L L
    /// I I J J K K L L
    ///
    pub fn duplicate(self, factor: (usize, usize)) -> Self
    where
        T: Copy,
    {
        let size = (self.size.0 * factor.0, self.size.1 * factor.1);
        let items: Vec<_> = self
            .items
            .into_iter()
            .flat_map(|v| (0..factor.0).map(move |_| v))
            .collect::<Vec<_>>()
            .chunks(size.0)
            .flat_map(|row| {
                let row: Vec<_> = row.to_vec();
                (0..factor.1).map(move |_| row.clone())
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
        T: Copy,
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
                    let r0: Vec<Vec<_>> = pair[0]
                        .clone()
                        .into_iter()
                        .map(|pair| vec![pair.clone(), pair].into_iter().flatten().collect())
                        .collect();

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

    pub fn get(&self, idx: IVec2) -> Option<&T> {
        if idx.x >= self.size.0 as i32 || idx.y >= self.size.1 as i32 || idx.x < 0 || idx.y < 0 {
            None
        } else {
            self.items
                .get(idx.x as usize + idx.y as usize * self.size.0)
        }
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

impl From<NoiseMap> for Grid<f64> {
    fn from(value: NoiseMap) -> Self {
        Grid::new(value.size(), value.into_iter().collect())
    }
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
        grid.duplicate((2, 2)),
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
pub fn test_grid_32x32_expand() {
    let grid = Grid::new((32, 32), (1..32 * 32 + 1).into_iter().collect()).expand();

    assert_eq!(grid.size(), (63, 63));
}

pub trait XyIndex<T> {
    fn get_xy(&self, p: IVec2) -> Option<&T>;
}
