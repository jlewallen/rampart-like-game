use bevy::math::*;

use crate::model::TILE_SIZE;

pub trait XyIndex<T> {
    fn get_xy(&self, p: IVec2) -> Option<&T>;
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

    pub fn into_cells(self) -> Vec<T> {
        self.cells
    }

    pub fn set(&mut self, p: IVec2, value: T) {
        let index = self.coordinates_to_index(p).expect("set coordinates");
        self.cells[index] = value;
    }

    pub fn get(&self, p: IVec2) -> Option<&T> {
        self.coordinates_to_index(p)
            .and_then(|index| self.cells.get(index))
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

    pub fn outline(&mut self, p0: IVec2, p1: IVec2, value: T)
    where
        T: Clone,
    {
        for x in p0.x..(p1.x + 1) {
            self.set(IVec2::new(x, p0.y), value.clone());
            self.set(IVec2::new(x, p1.y), value.clone());
        }
        for y in (p0.y + 1)..p1.y {
            self.set(IVec2::new(p0.x, y), value.clone());
            self.set(IVec2::new(p1.x, y), value.clone());
        }
    }

    fn coordinates_to_index(&self, p: IVec2) -> Option<usize> {
        if p.x < 0 || p.y < 0 || p.x + 1 > self.size.x as i32 || p.y + 1 > self.size.y as i32 {
            None
        } else {
            Some(p.y as usize * self.size.x as usize + p.x as usize)
        }
    }
}

/// Functions dealing with geometry. I kind of feel like these should be some
/// place else, SquareGrid doesn't quite feel right. One glaring red flag is the
/// TILE_SIZE dependency.
impl<T> SquareGrid<T> {
    pub fn world_to_local(&self) -> Vec3 {
        let size = self.size.as_vec2();
        let half_tile_size = Vec2::splat(TILE_SIZE) / 2.0;
        (Vec3::new(size.x, 0., size.y) / 2.0) - Vec3::new(half_tile_size.x, 0., half_tile_size.y)
    }

    pub fn local_to_world(&self) -> Vec3 {
        -self.world_to_local()
    }

    pub fn grid_to_world(&self, grid: IVec2) -> Vec3 {
        let grid = grid.as_vec2();
        Vec3::new(grid.x, 0., grid.y) + self.local_to_world()
    }

    pub fn layout(&self) -> Vec<(IVec2, Vec3, &T)> {
        self.cells
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let x = index as i32 % self.size.x as i32;
                let y = index as i32 / self.size.x as i32;
                let grid = IVec2::new(x, y);
                (grid, self.grid_to_world(grid), value)
            })
            .collect()
    }
}

impl<T: Default> Default for SquareGrid<T> {
    fn default() -> Self {
        Self {
            size: Default::default(),
            cells: Default::default(),
        }
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

impl<T> XyIndex<T> for SquareGrid<T> {
    fn get_xy(&self, p: IVec2) -> Option<&T> {
        self.coordinates_to_index(p).map(|index| &self.cells[index])
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

pub trait AroundCenter<T> {
    fn around(&self, center: IVec2) -> Around<Option<T>>;
}

#[derive(Debug)]
pub struct Around<T>(pub (T, T, T), pub (T, T, T), pub (T, T, T));

impl<T> Around<T> {
    #[cfg(test)]
    pub fn new(value: ((T, T, T), (T, T, T), (T, T, T))) -> Self {
        Self(value.0, value.1, value.2)
    }

    pub fn map<R>(self, map_fn: impl Fn(T) -> R) -> Around<R> {
        Around(
            (map_fn(self.0 .0), map_fn(self.0 .1), map_fn(self.0 .2)),
            (map_fn(self.1 .0), map_fn(self.1 .1), map_fn(self.1 .2)),
            (map_fn(self.2 .0), map_fn(self.2 .1), map_fn(self.2 .2)),
        )
    }

    pub fn to_vec(self) -> Vec<T> {
        vec![
            self.0 .0, self.0 .1, self.0 .2, self.1 .0, self.1 .1, self.1 .2, self.2 .0, self.2 .1,
            self.2 .2,
        ]
    }

    pub fn center(&self) -> &T {
        &self.1 .1
    }
}

impl Around<IVec2> {
    pub fn centered(c: IVec2) -> Self {
        Self(
            (
                IVec2::new(c.x - 1, c.y - 1),
                IVec2::new(c.x, c.y - 1),
                IVec2::new(c.x + 1, c.y - 1),
            ),
            (
                IVec2::new(c.x - 1, c.y),
                IVec2::new(c.x, c.y),
                IVec2::new(c.x + 1, c.y),
            ),
            (
                IVec2::new(c.x - 1, c.y + 1),
                IVec2::new(c.x, c.y + 1),
                IVec2::new(c.x + 1, c.y + 1),
            ),
        )
    }
}

impl<T, V> AroundCenter<V> for T
where
    T: XyIndex<V>,
    V: Clone,
{
    fn around(&self, center: IVec2) -> Around<Option<V>> {
        Around::centered(center).map(|xy| self.get_xy(xy).cloned())
    }
}

impl<T: PartialEq> PartialEq for Around<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1 && self.2 == other.2
    }
}
