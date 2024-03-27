use bevy::{math::primitives, prelude::*};
use bevy_mod_picking::prelude::*;

use crate::model::*;

#[derive(Resource)]
pub struct BuildingResources {
    pub simple: Handle<StandardMaterial>,
    pub unknown: Handle<Mesh>,
    pub east_west: Handle<Mesh>,
    pub north_south: Handle<Mesh>,
    pub corner: Handle<Scene>,
    pub cannon: Handle<Scene>,
}

pub fn load(
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

    commands.insert_resource(BuildingResources {
        simple,
        unknown,
        east_west,
        north_south,
        corner: asset_server.load("corner.glb#Scene0"),
        cannon: asset_server.load("cannon.glb#Scene0"),
    })
}

#[allow(dead_code)]
pub const HIGHLIGHT_TINT: Highlight<StandardMaterial> = Highlight {
    hovered: Some(HighlightKind::new_dynamic(|matl| StandardMaterial {
        // base_color: matl.base_color + Color::rgba(-0.2, -0.2, 0.4, 0.0),
        base_color: Color::rgb(0.35, 0.35, 0.35),
        ..matl.to_owned()
    })),
    pressed: Some(HighlightKind::new_dynamic(|matl| StandardMaterial {
        // base_color: matl.base_color + Color::rgba(-0.3, -0.3, 0.5, 0.0),
        base_color: Color::rgb(0.35, 0.75, 0.35),
        ..matl.to_owned()
    })),
    selected: Some(HighlightKind::new_dynamic(|matl| StandardMaterial {
        // base_color: matl.base_color + Color::rgba(-0.3, 0.2, -0.3, 0.0),
        base_color: Color::rgb(0.35, 0.35, 0.75),
        ..matl.to_owned()
    })),
};
