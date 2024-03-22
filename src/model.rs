use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        event::Event,
        schedule::States,
        system::Resource,
        world::{FromWorld, World},
    },
    math::{IVec2, Vec2},
};

mod grid;

#[allow(unused_imports)]
pub use grid::*;

pub const STRUCTURE_HEIGHT: f32 = 0.6;
pub const GROUND_DEPTH: f32 = 0.2;
pub const WALL_HEIGHT: f32 = 0.6;
pub const WALL_WIDTH: f32 = 0.4;
pub const TILE_SIZE: f32 = 1.0;
pub const ROUND_SHOT_SIZE: f32 = 0.25;
pub const BRICK_COLOR: &str = "E7444A";

// We base all the math on a desired time of flight that
// looks appropriate for the distance.
pub const MAXIMUM_HORIZONTAL_DISTANCE: f32 = 35.0;
pub const MINIMUM_FLIGHT_TIME: f32 = 1.0;
pub const GRAVITY: f32 = 9.8;

pub type Vec2Usize = (usize, usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seed<T>(T);

impl<T: Default> Default for Seed<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl From<Seed<u32>> for u32 {
    fn from(value: Seed<u32>) -> Self {
        value.0
    }
}

impl From<u32> for Seed<u32> {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct WorldGeometry<T> {
    size: Vec2Usize,
    map: Vec<T>,
}

pub trait AroundCenter<Item> {
    fn around(&self, center: IVec2) -> Around<Option<Item>>;
}

#[derive(Debug)]
pub struct Around<T>((T, T, T), (T, T, T), (T, T, T));

impl<T> Around<T> {
    pub fn map<R>(&self, map_fn: impl Fn(&T) -> R) -> Around<R> {
        Around(
            (map_fn(&self.0 .0), map_fn(&self.0 .1), map_fn(&self.0 .2)),
            (map_fn(&self.1 .0), map_fn(&self.1 .1), map_fn(&self.1 .2)),
            (map_fn(&self.2 .0), map_fn(&self.2 .1), map_fn(&self.2 .2)),
        )
    }
}

impl Around<IVec2> {
    pub fn center(c: IVec2) -> Self {
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

impl<T> WorldGeometry<T>
where
    T: Default + Clone,
{
    pub fn new(size: Vec2Usize) -> Self {
        Self {
            size,
            map: vec![T::default(); size.0 * size.1],
        }
    }

    pub fn set(&mut self, c: Vec2Usize, value: T) {
        let index = self.coordinates_to_index(c);
        self.map[index] = value;
    }

    pub fn get(&self, c: Vec2Usize) -> Option<&T> {
        let index = self.coordinates_to_index(c);
        if index < self.map.len() {
            Some(&self.map[index])
        } else {
            None
        }
    }

    pub fn outline(&mut self, (x0, y0): Vec2Usize, (x1, y1): Vec2Usize, value: T) {
        for x in x0..(x1 + 1) {
            self.set((x, y0), value.clone());
            self.set((x, y1), value.clone());
        }
        for y in (y0 + 1)..y1 {
            self.set((x0, y), value.clone());
            self.set((x1, y), value.clone());
        }
    }

    pub fn layout(&self) -> Vec<(Vec2Usize, Vec2, &T)> {
        self.map
            .iter()
            .enumerate()
            .map(|(index, value)| {
                (
                    self.index_to_grid(index),
                    self.index_to_coordindates(index),
                    value,
                )
            })
            .collect()
    }

    pub fn around(&self, c: Vec2Usize) -> Around<Option<&T>> {
        Around::center(IVec2::new(c.0 as i32, c.1 as i32))
            .map(|c| self.get((c.x as usize, c.y as usize)))
    }

    pub fn grid_position(&self, grid: Vec2Usize) -> Vec2 {
        self.index_to_coordindates(self.coordinates_to_index(grid))
    }

    fn index_to_grid(&self, index: usize) -> Vec2Usize {
        (index % self.size.0, index / self.size.1)
    }

    fn index_to_coordindates(&self, index: usize) -> Vec2 {
        let c = self.index_to_grid(index);
        let x: f32 = (c.0 as f32 - (self.size.0 / 2) as f32) * TILE_SIZE + (TILE_SIZE / 2.);
        let y: f32 = (c.1 as f32 - (self.size.1 / 2) as f32) * TILE_SIZE + (TILE_SIZE / 2.);
        Vec2::new(x, y)
    }

    fn coordinates_to_index(&self, c: Vec2Usize) -> usize {
        c.1 * self.size.1 + (c.0)
    }
}

#[derive(Component, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Coordinates(Vec2Usize);

impl Coordinates {
    pub fn new(vec: Vec2Usize) -> Self {
        Self(vec)
    }
}

impl From<Coordinates> for Vec2Usize {
    fn from(value: Coordinates) -> Self {
        value.0
    }
}

#[derive(Component, Clone, Default, Debug, PartialEq, Eq, Hash)]
pub enum Player {
    #[default]
    One,
    Two,
}

impl Player {
    pub fn next(&self) -> Self {
        match self {
            Player::One => Player::Two,
            Player::Two => Player::One,
        }
    }
}

#[derive(Resource, Default)]
pub struct ActivePlayer(Player);

impl ActivePlayer {
    pub fn new(player: Player) -> Self {
        Self(player)
    }

    pub fn player(&self) -> &Player {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, States)]
pub enum Phase {
    Fortify(Player),
    Arm(Player),
    Target(Player),
}

impl Default for Phase {
    fn default() -> Self {
        Phase::Fortify(Player::default())
    }
}

impl Phase {
    pub fn next(&self) -> Self {
        match self {
            Self::Fortify(Player::One) => Self::Arm(Player::One),
            Self::Arm(Player::One) => Self::Fortify(Player::Two),
            Self::Fortify(Player::Two) => Self::Arm(Player::Two),
            Self::Arm(Player::Two) => Self::Target(Player::One),
            Self::Target(Player::One) => Self::Target(Player::Two),
            Self::Target(Player::Two) => Self::Fortify(Player::One),
        }
    }

    pub fn player(&self) -> Player {
        match self {
            Self::Fortify(player) => player.clone(),
            Self::Arm(player) => player.clone(),
            Self::Target(player) => player.clone(),
        }
    }
}

pub trait Projectile {}

#[derive(Component, Clone, Debug)]
pub struct RoundShot {}

impl Projectile for RoundShot {}

#[derive(Clone, Debug)]
pub struct ConstructionEvent(Coordinates, Structure);

impl ConstructionEvent {
    pub fn new(coordinates: Coordinates, structure: Structure) -> Self {
        Self(coordinates, structure)
    }

    pub fn coordinates(&self) -> &Coordinates {
        &self.0
    }

    pub fn structure(&self) -> &Structure {
        &self.1
    }
}

impl Event for ConstructionEvent {}

#[derive(Component, Clone, Debug)]
pub enum Ground {
    Dirt,
}

impl Default for Ground {
    fn default() -> Self {
        Self::Dirt
    }
}

#[derive(Component, Clone, Debug)]
pub struct Wall {
    pub player: Player,
    pub entity: Option<Entity>,
}

#[derive(Component, Clone, Debug)]
pub struct Cannon {
    pub player: Player,
    pub entity: Option<Entity>,
}

#[derive(Clone, Debug)]
pub enum Structure {
    Wall(Wall),
    Cannon(Cannon),
}

#[derive(Debug)]
pub enum ConnectingWall {
    // Isolated,
    NorthSouth,
    EastWest,
    Corner(u32),
    Unknown,
}

impl<T> From<&Around<Option<&Option<T>>>> for ConnectingWall {
    fn from(value: &Around<Option<&Option<T>>>) -> Self {
        match value {
            Around((_, _, _), (_, _, Some(Some(_))), (_, Some(Some(_)), _)) => Self::Corner(0), // Bottom Right
            Around((_, _, _), (Some(Some(_)), _, _), (_, Some(Some(_)), _)) => Self::Corner(90), // Bottom Left
            Around((_, Some(Some(_)), _), (Some(Some(_)), _, _), (_, _, _)) => Self::Corner(180), // Top Left
            Around((_, Some(Some(_)), _), (_, _, Some(Some(_))), (_, _, _)) => Self::Corner(270), // Top Right
            Around(_, (Some(Some(_)), _, Some(Some(_))), _) => Self::EastWest,
            Around((_, Some(Some(_)), _), (_, _, _), (_, Some(Some(_)), _)) => Self::NorthSouth,
            Around((_, _, _), (_, _, _), (_, _, _)) => Self::Unknown,
        }
    }
}

#[derive(Resource)]
pub struct StructureLayers {
    pub(crate) structure_layer: WorldGeometry<Option<Structure>>,
}

impl StructureLayers {
    pub fn new(size: Vec2Usize) -> Self {
        Self {
            structure_layer: WorldGeometry::new(size),
        }
    }

    pub fn create_castle(&mut self, center: Vec2Usize, size: Vec2Usize, player: Player) {
        let (x0, y0) = (center.0 - size.0 / 2, center.1 - size.1 / 2);
        let (x1, y1) = (center.0 + size.0 / 2, center.1 + size.1 / 2);

        self.structure_layer.outline(
            (x0, y0),
            (x1, y1),
            Some(Structure::Wall(Wall {
                player: player.clone(),
                entity: None,
            })),
        );

        self.structure_layer.set(
            center,
            Some(Structure::Cannon(Cannon {
                player,
                entity: None,
            })),
        );
    }
}

impl FromWorld for StructureLayers {
    fn from_world(_world: &mut World) -> Self {
        let mut structure_layers = StructureLayers::new((32, 32));
        structure_layers.create_castle((4, 4), (4, 4), Player::One);
        structure_layers.create_castle((26, 26), (4, 4), Player::Two);
        structure_layers
    }
}
