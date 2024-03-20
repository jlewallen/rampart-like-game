use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use bevy_rapier3d::prelude::*;

use super::model::*;

use crate::{
    pick_coordinates,
    resources::{self, Structures},
    ActivePlayer, Cannon, ConnectingWall, ConstructionEvent, Coordinates, EntityLayer, Phase,
    Structure, Terrain, Vec2Usize, Wall, GROUND_DEPTH, WALL_HEIGHT,
};

pub struct BuildingPlugin;

impl Plugin for BuildingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_structures)
            .add_systems(Update, (place_wall.run_if(should_place_wall),))
            .add_systems(Update, (place_cannon.run_if(should_place_cannon),))
            .add_systems(Update, refresh_terrain);
    }
}

fn should_place_wall(state: Res<State<Phase>>) -> bool {
    matches!(state.get(), Phase::Fortify(_))
}

fn should_place_cannon(state: Res<State<Phase>>) -> bool {
    matches!(state.get(), Phase::Arm(_))
}

pub fn place_wall(
    player: Res<ActivePlayer>,
    events: EventReader<Pointer<Click>>,
    targets: Query<(&Transform, &Name, Option<&Coordinates>), Without<Cannon>>,
    mut modified: EventWriter<ConstructionEvent>,
) {
    let picked = pick_coordinates(events, targets);
    if picked.is_none() {
        return;
    }

    let picked = picked.expect("No picked");

    info!("place-wall p={:?}", &picked);

    modified.send(ConstructionEvent::new(
        picked.coordinates,
        Structure::Wall(Wall {
            player: player.player().clone(),
            entity: None,
        }),
    ));
}

pub fn place_cannon(
    player: Res<ActivePlayer>,
    events: EventReader<Pointer<Click>>,
    targets: Query<(&Transform, &Name, Option<&Coordinates>), Without<Cannon>>,
    mut modified: EventWriter<ConstructionEvent>,
) {
    let picked = pick_coordinates(events, targets);
    if picked.is_none() {
        return;
    }

    let picked = picked.expect("No picked");

    info!("place-cannon p={:?}", &picked);

    modified.send(ConstructionEvent::new(
        picked.coordinates,
        Structure::Cannon(Cannon {
            player: player.player().clone(),
            entity: None,
        }),
    ));
}

fn create_structure(
    commands: &mut Commands,
    terrain: &Terrain,
    grid: Vec2Usize,
    position: &Vec2,
    item: &Structure,
    structures: &Res<Structures>,
    _entities: &mut ResMut<EntityLayer>,
) {
    match item {
        Structure::Wall(wall) => {
            let around = &terrain.structure_layer.around(grid);

            let connecting: ConnectingWall = around.into();

            // info!("create-structure {:?} {:?}", grid, connecting);

            commands
                .spawn((
                    Name::new(format!("Wall{:?}", &grid)),
                    SpatialBundle {
                        transform: Transform::from_xyz(
                            position.x,
                            (WALL_HEIGHT / 2.) + (GROUND_DEPTH / 2.),
                            position.y,
                        ),
                        ..default()
                    },
                    PickableBundle::default(),
                    Collider::cuboid(TILE_SIZE / 2., STRUCTURE_HEIGHT / 2., TILE_SIZE / 2.),
                    CollisionGroups::new(Group::all(), Group::all()),
                    Coordinates::new(grid),
                    wall.player.clone(),
                    wall.clone(),
                    resources::HIGHLIGHT_TINT,
                ))
                .with_children(|parent| match connecting {
                    ConnectingWall::Isolated => {
                        parent.spawn(PbrBundle {
                            mesh: structures.unknown.clone(),
                            material: structures.simple.clone(),
                            ..default()
                        });
                    }
                    ConnectingWall::NorthSouth => {
                        parent.spawn(PbrBundle {
                            mesh: structures.north_south.clone(),
                            material: structures.simple.clone(),
                            ..default()
                        });
                    }
                    ConnectingWall::EastWest => {
                        parent.spawn(PbrBundle {
                            mesh: structures.east_west.clone(),
                            material: structures.simple.clone(),
                            ..default()
                        });
                    }
                    ConnectingWall::Corner(angle) => {
                        parent.spawn(SceneBundle {
                            scene: structures.corner.clone(),
                            transform: Transform::from_rotation(Quat::from_rotation_y(
                                -(angle as f32 * std::f32::consts::PI / 180.),
                            )),
                            ..default()
                        });
                    }
                    _ => {
                        parent.spawn(PbrBundle {
                            mesh: structures.unknown.clone(),
                            ..default()
                        });
                    }
                });
        }
        Structure::Cannon(cannon) => {
            commands
                .spawn((
                    Name::new(format!("Cannon{:?}", &grid)),
                    SpatialBundle {
                        transform: Transform::from_xyz(
                            position.x,
                            STRUCTURE_HEIGHT / 2.0,
                            position.y,
                        ),
                        ..default()
                    },
                    PickableBundle::default(),
                    CollisionGroups::new(Group::all(), Group::all()),
                    Collider::cuboid(TILE_SIZE / 2., STRUCTURE_HEIGHT / 2., TILE_SIZE / 2.),
                    Coordinates::new(grid),
                    cannon.player.clone(),
                    cannon.clone(),
                    resources::HIGHLIGHT_TINT,
                ))
                .with_children(|parent| {
                    parent.spawn(SceneBundle {
                        scene: structures.cannon.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_y(0.)),
                        ..default()
                    });
                });
        }
    }
}

fn setup_structures(
    mut commands: Commands,
    mut entities: ResMut<EntityLayer>,
    terrain: Res<Terrain>,
    structures: Res<Structures>,
) {
    for (grid, position, item) in terrain.structure_layer.layout() {
        if let Some(item) = item {
            create_structure(
                &mut commands,
                &terrain,
                grid,
                &position,
                item,
                &structures,
                &mut entities,
            )
        }
    }
}

fn refresh_terrain(
    mut commands: Commands,
    mut modified: EventReader<ConstructionEvent>,
    mut terrain: ResMut<Terrain>,
    mut entities: ResMut<EntityLayer>,
    structures: Res<Structures>,
) {
    for ev in modified.read() {
        info!("terrain-modified {:?}", ev);

        let grid = ev.coordinates().clone().into();
        let structure = ev.structure().clone();
        let position = terrain.structure_layer.grid_position(grid);
        let existing = terrain.structure_layer.get(grid).expect("Out of bounds");
        if existing.is_some() {
            return;
        }

        let _around = terrain.structure_layer.around(grid);

        terrain.structure_layer.set(grid, Some(structure.clone()));

        create_structure(
            &mut commands,
            &terrain,
            grid,
            &position,
            &structure,
            &structures,
            &mut entities,
        );
    }
}
