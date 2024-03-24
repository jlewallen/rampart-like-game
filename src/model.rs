use bevy::{
    ecs::{component::Component, schedule::States, system::Resource},
    math::{IVec2, UVec2},
};

mod grid;
#[cfg(test)]
mod tests;

pub use grid::*;

pub const STRUCTURE_HEIGHT: f32 = 0.6;
pub const GROUND_DEPTH: f32 = 0.2;
pub const WALL_HEIGHT: f32 = 0.6;
pub const WALL_WIDTH: f32 = 0.4;
pub const TILE_SIZE: f32 = 1.0;
pub const HEIGHT_SCALE: f32 = 1.0;
pub const ROUND_SHOT_DIAMETER: f32 = 0.25;
pub const BRICK_COLOR: &str = "e7444a";

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

impl<T> Seed<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl Seed<u32> {
    pub fn system_time() -> Seed<u32> {
        Seed(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time drift detected, aborting")
                .as_secs() as u32, // TODO
        )
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

#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Coordinates(IVec2);

impl Coordinates {
    pub fn new(vec: IVec2) -> Self {
        Self(vec)
    }
}

impl From<IVec2> for Coordinates {
    fn from(value: IVec2) -> Self {
        Self(value)
    }
}

impl From<Coordinates> for IVec2 {
    fn from(value: Coordinates) -> Self {
        value.0
    }
}

#[derive(Component, Copy, Clone, Default, Debug, PartialEq, Eq, Hash)]
pub enum Player {
    #[default]
    One,
    Two,
}

impl Player {
    #[allow(dead_code)]
    pub fn next(&self) -> Self {
        match self {
            Player::One => Player::Two,
            Player::Two => Player::One,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, States, Default)]
pub enum Activity {
    #[default]
    Observing,
    Building,
    Firing,
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

    #[allow(dead_code)]
    pub fn player(&self) -> Player {
        match self {
            Self::Fortify(player) => player.clone(),
            Self::Arm(player) => player.clone(),
            Self::Target(player) => player.clone(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
pub enum AppState {
    #[default]
    Menu,
    Game,
}

#[derive(Debug, Resource)]
pub struct Settings {
    pub size: UVec2,
    pub seed: Seed<u32>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            seed: Seed::system_time(),
            size: UVec2::new(64, 64),
        }
    }
}

impl Settings {
    pub fn seed(&self) -> Seed<u32> {
        self.seed
    }

    pub fn size(&self) -> UVec2 {
        self.size
    }
}
