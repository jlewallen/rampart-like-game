use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use bevy_rapier3d::prelude::*;

use resources::BuildingResources;

use super::model::*;

mod resources;

use crate::{
    helpers::GamePlayLifetime,
    model::{Coordinates, GROUND_DEPTH, WALL_HEIGHT},
    terrain::{SurveyedCell, Terrain},
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

fn setup_structures(
    mut commands: Commands,
    resources: Res<BuildingResources>,
    settings: Res<Settings>,
) {
    let mut structures = StructureLayers::new(settings.size());
    structures.create_castle(IVec2::new(4, 4), IVec2::new(4, 4), Player::One);
    structures.create_castle(IVec2::new(26, 26), IVec2::new(4, 4), Player::Two);
    structures.refresh_entities(&mut commands, &resources);

    commands.insert_resource(structures);
}

fn refresh_terrain(
    mut commands: Commands,
    mut modified: EventReader<ConstructionEvent>,
    mut structures: ResMut<StructureLayers>,
    resources: Res<BuildingResources>,
) {
    for ev in modified.read() {
        info!("terrain-modified {:?}", ev);

        let grid = ev.coordinates().clone().into();
        structures.set(grid, ev.structure().clone());
        structures.refresh_entities(&mut commands, &resources);
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

fn stop_placing(mut commands: Commands, placing: Query<(Entity, &Placing)>) {
    if let Ok((entity, _)) = placing.get_single() {
        commands.entity(entity).despawn_recursive();
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

#[derive(Default, Clone)]
pub enum StructureEntity {
    #[default]
    Empty,
    New(Structure),
    Affected(Structure, Entity),
    Current(Structure, Entity),
}

impl StructureEntity {
    fn affected(&self) -> Self {
        match self {
            StructureEntity::Empty => StructureEntity::Empty,
            StructureEntity::New(s) => StructureEntity::New(s.clone()),
            StructureEntity::Affected(s, e) => StructureEntity::Affected(s.clone(), e.clone()),
            StructureEntity::Current(s, e) => StructureEntity::Affected(s.clone(), e.clone()),
        }
    }

    fn structure(self) -> Option<Structure> {
        match self {
            StructureEntity::Empty => None,
            StructureEntity::New(s) => Some(s),
            StructureEntity::Affected(s, _) => Some(s),
            StructureEntity::Current(s, _) => Some(s),
        }
    }
}

#[derive(Default, Resource)]
pub struct StructureLayers {
    entities: SquareGrid<StructureEntity>,
}

impl StructureLayers {
    pub fn new(size: UVec2) -> Self {
        Self {
            entities: SquareGrid::new_flat(size),
        }
    }

    pub fn create_castle(&mut self, center: IVec2, size: IVec2, player: Player) {
        let (x0, y0) = (center.x - size.x / 2, center.y - size.y / 2);
        let (x1, y1) = (center.x + size.x / 2, center.y + size.y / 2);

        self.entities.outline(
            IVec2::new(x0 as i32, y0 as i32),
            IVec2::new(x1 as i32, y1 as i32),
            StructureEntity::New(Structure::Wall(Wall {
                player: player.clone(),
            })),
        );

        self.entities.set(
            IVec2::new(center.x as i32, center.y as i32),
            StructureEntity::New(Structure::Cannon(Cannon { player })),
        );
    }

    fn set(&mut self, grid: IVec2, structure: Structure) {
        self.entities.set(grid, StructureEntity::New(structure));

        for v in Around::centered(grid).to_vec().into_iter() {
            if let Some(e) = self.entities.get(v) {
                self.entities.set(v, e.affected());
            }
        }
    }

    fn refresh_entities(&mut self, commands: &mut Commands, resources: &Res<BuildingResources>) {
        let mut refreshing = Vec::default();

        for (grid, position, item) in self.entities.layout() {
            match item {
                StructureEntity::New(item) => {
                    let entity = self.create_entity(commands, grid, position, item, resources);
                    refreshing.push((grid, StructureEntity::Current(item.clone(), entity)))
                }
                StructureEntity::Affected(item, e) => match item {
                    Structure::Wall(_) => {
                        commands.entity(e.clone()).despawn_recursive();
                        let entity = self.create_entity(commands, grid, position, item, resources);
                        refreshing.push((grid, StructureEntity::Current(item.clone(), entity)))
                    }
                    Structure::Cannon(_) => {
                        refreshing.push((grid, StructureEntity::Current(item.clone(), e.clone())))
                    }
                },
                StructureEntity::Current(_, _) => {}
                StructureEntity::Empty => {}
            }
        }

        for (grid, update) in refreshing.into_iter() {
            self.entities.set(grid, update);
        }
    }

    fn create_entity(
        &self,
        commands: &mut Commands,
        grid: IVec2,
        position: Vec3,
        item: &Structure,
        resources: &Res<BuildingResources>,
    ) -> Entity {
        match item {
            Structure::Wall(wall) => {
                let around = self.entities.around(grid);

                let connecting: ConnectingWall = around.map(simplify).into();

                let offset = Vec3::Y * ((WALL_HEIGHT / 2.) + (GROUND_DEPTH / 2.));

                let position = position + offset;

                info!(%grid, %position, %offset, "create-structure");

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
                        ConnectingWall::Isolated => {
                            parent.spawn(PbrBundle {
                                mesh: resources.unknown.clone(),
                                material: resources.simple.clone(),
                                ..default()
                            });
                        }
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
                    })
                    .id()
            }
            Structure::Cannon(cannon) => {
                let offset = Vec3::Y * (STRUCTURE_HEIGHT / 2.0);
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
                    })
                    .id()
            }
        }
    }
}

#[derive(Component, Clone, Debug)]
pub struct Wall {
    player: Player,
}

#[derive(Component, Clone, Debug)]
pub struct Cannon {
    player: Player,
}

#[derive(Clone, Debug)]
pub enum Structure {
    Wall(Wall),
    Cannon(Cannon),
}

impl Structure {
    fn as_wall(self) -> Option<Structure> {
        match self {
            Structure::Wall(w) => Some(Structure::Wall(w)),
            Structure::Cannon(_) => None,
        }
    }
}

#[derive(Debug)]
pub enum ConnectingWall {
    Isolated,
    NorthSouth,
    EastWest,
    Corner(u32),
    Unknown,
}

fn simplify(v: Option<StructureEntity>) -> Option<Structure> {
    v.and_then(|v| v.structure()).and_then(|v| v.as_wall())
}

impl From<Around<Option<Structure>>> for ConnectingWall {
    fn from(value: Around<Option<Structure>>) -> Self {
        match value {
            Around((None, None, None), (None, _, Some(_)), (None, Some(_), None)) => {
                Self::Corner(0)
            } // Bottom Right
            Around((None, None, None), (Some(_), _, None), (None, Some(_), None)) => {
                Self::Corner(90)
            } // Bottom Left
            Around((None, Some(_), None), (Some(_), _, None), (None, None, None)) => {
                Self::Corner(180)
            } // Top Left
            Around((None, Some(_), None), (None, _, Some(_)), (None, None, None)) => {
                Self::Corner(270)
            } // Top Right
            Around((None, None, None), (Some(_), _, Some(_)), (None, None, None)) => Self::EastWest,
            Around((None, Some(_), None), (None, _, None), (None, Some(_), None)) => {
                Self::NorthSouth
            }
            Around((None, None, None), (None, Some(_), None), (None, None, None)) => Self::Isolated,
            Around((_, _, _), (_, _, _), (_, _, _)) => Self::Unknown,
        }
    }
}
