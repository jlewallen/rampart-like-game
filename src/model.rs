use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        event::Event,
        schedule::States,
        system::Resource,
        world::{FromWorld, World},
    },
    math::{IVec2, UVec2},
};

mod grid;

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

#[derive(Component, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Coordinates(IVec2);

impl Coordinates {
    pub fn new(vec: IVec2) -> Self {
        Self(vec)
    }
}

impl From<Coordinates> for IVec2 {
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
#[allow(dead_code)]
pub enum ConnectingWall {
    // Isolated,
    NorthSouth,
    EastWest,
    Corner(u32),
    Unknown,
}

/*
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
*/

#[derive(Resource)]
pub struct StructureLayers {
    pub(crate) structure_layer: SquareGrid<Option<Structure>>,
}

impl StructureLayers {
    pub fn new(size: UVec2) -> Self {
        Self {
            structure_layer: SquareGrid::new_flat(size),
        }
    }

    pub fn create_castle(&mut self, center: IVec2, size: IVec2, player: Player) {
        let (x0, y0) = (center.x - size.x / 2, center.y - size.y / 2);
        let (x1, y1) = (center.x + size.x / 2, center.y + size.y / 2);

        self.structure_layer.outline(
            IVec2::new(x0 as i32, y0 as i32),
            IVec2::new(x1 as i32, y1 as i32),
            Some(Structure::Wall(Wall {
                player: player.clone(),
                entity: None,
            })),
        );

        self.structure_layer.set(
            IVec2::new(center.x as i32, center.y as i32),
            Some(Structure::Cannon(Cannon {
                player,
                entity: None,
            })),
        );
    }
}

impl FromWorld for StructureLayers {
    fn from_world(_world: &mut World) -> Self {
        let mut structure_layers = StructureLayers::new(UVec2::new(64, 64));
        structure_layers.create_castle(IVec2::new(4, 4), IVec2::new(4, 4), Player::One);
        structure_layers.create_castle(IVec2::new(26, 26), IVec2::new(4, 4), Player::Two);
        structure_layers
    }
}
