use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use bevy_rapier3d::prelude::*;

use super::model::*;

use crate::{
    pick_coordinates,
    resources::{self, Structures},
    ActivePlayer, Cannon, ConnectingWall, ConstructionEvent, Coordinates, Phase, Structure,
    StructureLayers, Terrain, Vec2Usize, Wall, GROUND_DEPTH, WALL_HEIGHT,
};

pub struct BuildingPlugin;

impl Plugin for BuildingPlugin {
    fn build(&self, app: &mut App) {
        if false {
            app.add_systems(Startup, setup_structures)
                .add_systems(Update, (place_wall.run_if(should_place_wall),))
                .add_systems(Update, (place_cannon.run_if(should_place_cannon),))
                .add_systems(Update, refresh_terrain)
                .init_resource::<StructureLayers>();
        } else {
            app.add_systems(Update, keyboard)
                .add_systems(Update, placing)
                .add_systems(Update, try_place);
        }
    }
}

fn should_place_wall(state: Res<State<Phase>>) -> bool {
    matches!(state.get(), Phase::Fortify(_))
}

fn should_place_cannon(state: Res<State<Phase>>) -> bool {
    matches!(state.get(), Phase::Arm(_))
}

fn place_wall(
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

fn place_cannon(
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
    terrain: &StructureLayers,
    grid: Vec2Usize,
    position: &Vec2,
    item: &Structure,
    structures: &Res<Structures>,
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
                    /*
                    ConnectingWall::Isolated => {
                        parent.spawn(PbrBundle {
                            mesh: structures.unknown.clone(),
                            material: structures.simple.clone(),
                            ..default()
                        });
                    }
                    */
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
    structure_layers: Res<StructureLayers>,
    structures: Res<Structures>,
) {
    for (grid, position, item) in structure_layers.structure_layer.layout() {
        if let Some(item) = item {
            create_structure(
                &mut commands,
                &structure_layers,
                grid,
                &position,
                item,
                &structures,
            )
        }
    }
}

fn refresh_terrain(
    mut commands: Commands,
    mut modified: EventReader<ConstructionEvent>,
    mut structure_layers: ResMut<StructureLayers>,
    structures: Res<Structures>,
) {
    for ev in modified.read() {
        info!("terrain-modified {:?}", ev);

        let grid = ev.coordinates().clone().into();
        let structure = ev.structure().clone();
        let position = structure_layers.structure_layer.grid_position(grid);
        let existing = structure_layers
            .structure_layer
            .get(grid)
            .expect("Out of bounds");
        if existing.is_some() {
            return;
        }

        let _around = structure_layers.structure_layer.around(grid);

        structure_layers
            .structure_layer
            .set(grid, Some(structure.clone()));

        create_structure(
            &mut commands,
            &structure_layers,
            grid,
            &position,
            &structure,
            &structures,
        );
    }
}

fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    placing: Query<(Entity, &Placing)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if keys.just_pressed(KeyCode::KeyB) {
        info!("{:?}", KeyCode::KeyB);

        if let Ok((entity, _)) = placing.get_single() {
            commands.entity(entity).despawn_recursive();
        }

        commands.spawn((
            Name::new("Placing"),
            Pickable::IGNORE,
            Placing { allowed: true },
            PbrBundle {
                mesh: meshes.add(Cuboid::new(1., 0.2, 1.)),
                material: materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    ..default()
                }),
                transform: Transform::from_translation(Vec3::Y),
                ..default()
            },
        ));
    }
}

fn placing(
    mut events: EventReader<Pointer<Move>>,
    mut placing: Query<(&mut Placing, &mut Transform)>,
    terrain: Query<&Terrain>,
) {
    if events.is_empty() {
        return;
    }

    let Some(terrain) = terrain.get_single().ok() else {
        warn!("no terrain");
        return;
    };

    trace!("{:?}", terrain);

    for event in events.read() {
        if let Some(position) = event.event.hit.position {
            for (_, mut transform) in &mut placing {
                *transform = Transform::from_translation(position);
            }
        }
    }
}

fn try_place(
    mut events: EventReader<Pointer<Click>>,
    mut placing: Query<(&mut Placing, &mut Transform)>,
    terrain: Query<&Terrain>,
) {
    if events.is_empty() {
        return;
    }

    let Some(terrain) = terrain.get_single().ok() else {
        warn!("no terrain");
        return;
    };

    for event in events.read() {
        if let Some(position) = event.event.hit.position {
            if let Some(survey) = terrain.survey(position) {
                info!("{:#?}", survey);
            }

            for (_, mut transform) in &mut placing {
                *transform = Transform::from_translation(position);
            }
        }
    }
}

#[derive(Component, Debug)]
struct Placing {
    #[allow(dead_code)]
    allowed: bool,
}
