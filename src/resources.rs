#[allow(unused_imports)]
use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::{math::primitives, prelude::*};

use super::model::*;

#[derive(Resource)]
pub struct Structures {
    pub simple: Handle<StandardMaterial>,
    pub unknown: Handle<Mesh>,
    pub east_west: Handle<Mesh>,
    pub north_south: Handle<Mesh>,
    pub corner: Handle<Scene>,
    pub cannon: Handle<Scene>,
}

pub fn load_structures(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let simple = materials.add(StandardMaterial {
        base_color: Color::hex(BRICK_COLOR).expect("BRICK_COLOR"),
        perceptual_roughness: 1.0,
        ..default()
    });
    let unknown = meshes.add(Mesh::from(primitives::Cuboid::new(
        TILE_SIZE, TILE_SIZE, TILE_SIZE,
    )));
    let north_south = meshes.add(Mesh::from(primitives::Cuboid::new(
        WALL_WIDTH,
        WALL_HEIGHT,
        TILE_SIZE,
    )));
    let east_west = meshes.add(Mesh::from(primitives::Cuboid::new(
        TILE_SIZE,
        WALL_HEIGHT,
        WALL_WIDTH,
    )));

    commands.insert_resource(Structures {
        simple,
        unknown,
        east_west,
        north_south,
        corner: asset_server.load("corner.glb#Scene0"),
        cannon: asset_server.load("cannon.glb#Scene0"),
    })
}
