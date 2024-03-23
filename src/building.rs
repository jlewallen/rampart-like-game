use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use bevy_rapier3d::prelude::*;

use resources::BuildingResources;

use super::model::*;

mod resources;

use crate::{
    helpers::GamePlayLifetime, model::Coordinates, model::GROUND_DEPTH, model::WALL_HEIGHT,
    terrain::SurveyedCell, terrain::Terrain,
};

pub struct BuildingPlugin;

impl Plugin for BuildingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StructureLayers>()
            .add_systems(PreStartup, resources::load)
            .add_event::<ConstructionEvent>()
            .add_systems(OnEnter(AppState::Game), setup_structures)
            .add_systems(Update, refresh_terrain.run_if(in_state(AppState::Game)))
            .add_systems(OnEnter(Activity::Building), start_placing)
            .add_systems(OnExit(Activity::Building), stop_placing)
            .add_systems(Update, placing.run_if(in_state(Activity::Building)))
            .add_systems(Update, try_place.run_if(in_state(Activity::Building)));
    }
}

fn create_structure(
    commands: &mut Commands,
    terrain: &StructureLayers,
    grid: IVec2,
    position: Vec3,
    item: &Structure,
    resources: &Res<BuildingResources>,
) {
    match item {
        Structure::Wall(wall) => {
            let around = terrain.structure_layer.around(grid);

            let connecting: ConnectingWall = around.into();

            let offset = Vec3::Y * (WALL_HEIGHT / 2.) + (GROUND_DEPTH / 2.);

            let position = position + offset;

            // info!("create-structure {:?} {:?}", grid, connecting);

            commands
                .spawn((
                    Name::new(format!("Wall{:?}", &grid)),
                    SpatialBundle {
                        transform: Transform::from_translation(position),
                        ..default()
                    },
                    GamePlayLifetime,
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
                            mesh: resources.north_south.clone(),
                            material: resources.simple.clone(),
                            ..default()
                        });
                    }
                    ConnectingWall::EastWest => {
                        parent.spawn(PbrBundle {
                            mesh: resources.east_west.clone(),
                            material: resources.simple.clone(),
                            ..default()
                        });
                    }
                    ConnectingWall::Corner(angle) => {
                        parent.spawn(SceneBundle {
                            scene: resources.corner.clone(),
                            transform: Transform::from_rotation(Quat::from_rotation_y(
                                -(angle as f32 * std::f32::consts::PI / 180.),
                            )),
                            ..default()
                        });
                    }
                    _ => {
                        parent.spawn(PbrBundle {
                            mesh: resources.unknown.clone(),
                            ..default()
                        });
                    }
                });
        }
        Structure::Cannon(cannon) => {
            let offset = Vec3::Y * STRUCTURE_HEIGHT / 2.0;
            let position = position + offset;
            commands
                .spawn((
                    Name::new(format!("Cannon{:?}", &grid)),
                    GamePlayLifetime,
                    SpatialBundle {
                        transform: Transform::from_translation(position),
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
                        scene: resources.cannon.clone(),
                        transform: Transform::from_rotation(Quat::from_rotation_y(0.)),
                        ..default()
                    });
                });
        }
    }
}

fn setup_structures(mut commands: Commands, resources: Res<BuildingResources>) {
    let mut structure_layers = StructureLayers::new(UVec2::new(64, 64));
    structure_layers.create_castle(IVec2::new(4, 4), IVec2::new(4, 4), Player::One);
    structure_layers.create_castle(IVec2::new(26, 26), IVec2::new(4, 4), Player::Two);

    for (grid, position, item) in structure_layers.structure_layer.layout() {
        if let Some(item) = item {
            create_structure(
                &mut commands,
                &structure_layers,
                grid,
                position,
                &item,
                &resources,
            )
        }
    }

    commands.insert_resource(structure_layers);
}

fn refresh_terrain(
    mut commands: Commands,
    mut modified: EventReader<ConstructionEvent>,
    mut structure_layers: ResMut<StructureLayers>,
    resources: Res<BuildingResources>,
) {
    for ev in modified.read() {
        info!("terrain-modified {:?}", ev);

        let grid = ev.coordinates().clone().into();
        let structure = ev.structure().clone();
        let position = structure_layers.structure_layer.grid_to_world(grid);
        let existing = structure_layers
            .structure_layer
            .get_xy(grid)
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
            position,
            &structure,
            &resources,
        );
    }
}

fn stop_placing(mut commands: Commands, placing: Query<(Entity, &Placing)>) {
    if let Ok((entity, _)) = placing.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}

fn start_placing(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    placing: Query<(Entity, &Placing)>,
) {
    if let Ok((entity, _)) = placing.get_single() {
        commands.entity(entity).despawn_recursive();
    }

    commands.spawn((
        Name::new("Placing"),
        Pickable::IGNORE,
        GamePlayLifetime,
        Placing { allowed: true },
        PbrBundle {
            mesh: meshes.add(Cuboid::new(TILE_SIZE, 0.2, TILE_SIZE)),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                ..default()
            }),
            transform: Transform::from_translation(Vec3::Y),
            ..default()
        },
    ));
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
        return;
    };

    for event in events.read() {
        if let Some(position) = event.event.hit.position {
            if let Some(survey) = terrain.survey(position) {
                for (_, mut transform) in &mut placing {
                    *transform = Transform::from_translation(survey.world());
                }
            }
        }
    }
}

fn try_place(
    terrain: Query<&Terrain>,
    mut events: EventReader<Pointer<Click>>,
    _placing: Query<(&mut Placing, &mut Transform)>,
    mut modified: EventWriter<ConstructionEvent>,
) {
    if events.is_empty() {
        return;
    }

    let Some(terrain) = terrain.get_single().ok() else {
        return;
    };

    for event in events.read() {
        if let Some(position) = event.event.hit.position {
            if let Some(survey) = terrain.survey(position) {
                info!("{:#?}", survey);

                match survey.cell() {
                    SurveyedCell::Ground(_cell) => {
                        modified.send(ConstructionEvent::new(
                            survey.location().into(),
                            Structure::Wall(Wall {
                                player: Player::One,
                                entity: None,
                            }),
                        ));
                    }
                    SurveyedCell::Beach => {}
                    SurveyedCell::Water => {}
                }
            }
        }
    }
}

#[derive(Component, Debug)]
struct Placing {
    #[allow(dead_code)]
    allowed: bool,
}

#[derive(Clone, Debug)]
pub struct ConstructionEvent(Coordinates, Structure);

impl Event for ConstructionEvent {}

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

#[derive(Default, Resource)]
pub struct StructureLayers {
    structure_layer: SquareGrid<Option<Structure>>,
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

#[derive(Component, Clone, Debug)]
pub struct Wall {
    player: Player,
    #[allow(dead_code)]
    entity: Option<Entity>,
}

#[derive(Component, Clone, Debug)]
pub struct Cannon {
    player: Player,
    #[allow(dead_code)]
    entity: Option<Entity>,
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

impl<T> From<Around<Option<Option<T>>>> for ConnectingWall {
    fn from(value: Around<Option<Option<T>>>) -> Self {
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
