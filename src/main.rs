use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*, window::PresentMode};
use bevy_hanabi::prelude::*;
use bevy_mod_picking::{
    CustomHighlightPlugin, DefaultHighlighting, DefaultPickingPlugins, PickableBundle,
    PickingCameraBundle, PickingEvent,
};
use bevy_rapier3d::prelude::*;
use iyes_loopless::prelude::*;
use std::f32::consts::*;

pub type Vec2Usize = (usize, usize);

const STRUCTURE_HEIGHT: f32 = 0.6;
const GROUND_DEPTH: f32 = 0.2;
const WALL_HEIGHT: f32 = 0.6;
const WALL_WIDTH: f32 = 0.4;
const TILE_SIZE: f32 = 1.0;
const ROUND_SHOT_SIZE: f32 = 0.25;
const BRICK_COLOR: &str = "E7444A";

// We base all the math on a desired time of flight that
// looks appropriate for the distance.
const MAXIMUM_HORIZONTAL_DISTANCE: f32 = 35.0;
const MINIMUM_FLIGHT_TIME: f32 = 1.0;
const GRAVITY: f32 = 9.8;

#[derive(Debug)]
pub struct WorldGeometry<T> {
    size: Vec2Usize,
    map: Vec<T>,
}

#[derive(Debug)]
pub struct Around<T>((T, T, T), (T, T, T), (T, T, T));

impl<T> Around<T> {
    pub fn map<R>(&self, map_fn: &dyn Fn(&T) -> R) -> Around<R> {
        Around(
            (map_fn(&self.0 .0), map_fn(&self.0 .1), map_fn(&self.0 .2)),
            (map_fn(&self.1 .0), map_fn(&self.1 .1), map_fn(&self.1 .2)),
            (map_fn(&self.2 .0), map_fn(&self.2 .1), map_fn(&self.2 .2)),
        )
    }
}

impl Around<Vec2Usize> {
    pub fn center(c: Vec2Usize) -> Self {
        Self(
            ((c.0 - 1, c.1 - 1), (c.0, c.1 - 1), (c.0 + 1, c.1 - 1)),
            ((c.0 - 1, c.1), (c.0, c.1), (c.0 + 1, c.1)),
            ((c.0 - 1, c.1 + 1), (c.0, c.1 + 1), (c.0 + 1, c.1 + 1)),
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
        Around::center(c).map(&|c| self.get(*c))
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

#[derive(Component, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Player {
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

impl Default for Player {
    fn default() -> Player {
        Player::One
    }
}

#[derive(Resource, Default)]
pub struct ActivePlayer(Player);

#[derive(Resource, Default)]
pub struct ActivePhase(Phase);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Component, Clone, Debug)]
pub enum Ground {
    Dirt,
    Grass,
    Water,
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
pub struct TerrainModifiedEvent(Coordinates, Structure);

#[derive(Clone, Debug)]
pub enum Structure {
    Wall(Wall),
    Cannon(Cannon),
}

#[derive(Resource)]
pub struct Terrain {
    ground_layer: WorldGeometry<Ground>,
    structure_layer: WorldGeometry<Option<Structure>>,
}

impl FromWorld for Terrain {
    fn from_world(_world: &mut World) -> Self {
        load_terrain()
    }
}

impl Terrain {
    pub fn new(size: Vec2Usize) -> Self {
        Self {
            ground_layer: WorldGeometry::new(size),
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

pub trait Projectile {}

#[derive(Component, Clone, Debug)]
pub struct RoundShot {}

impl Projectile for RoundShot {}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "Castle".to_string(),
                width: 1024. + 256. + 32.,
                height: 768.,
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            },
            ..default()
        }))
        .add_plugins(
            DefaultPickingPlugins
                .set(CustomHighlightPlugin::<StandardMaterial> {
                    highlighting_default: |mut assets| DefaultHighlighting {
                        hovered: assets.add(Color::rgb(0.35, 0.35, 0.35).into()),
                        pressed: assets.add(Color::rgb(0.35, 0.75, 0.35).into()),
                        selected: assets.add(Color::rgb(0.35, 0.35, 0.75).into()),
                    },
                })
                .set(CustomHighlightPlugin::<ColorMaterial> {
                    highlighting_default: |mut assets| DefaultHighlighting {
                        hovered: assets.add(Color::rgb(0.35, 0.35, 0.35).into()),
                        pressed: assets.add(Color::rgb(0.35, 0.75, 0.35).into()),
                        selected: assets.add(Color::rgb(0.35, 0.35, 0.75).into()),
                    },
                }),
        )
        .add_plugin(HanabiPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugin(RapierDebugRenderPlugin::default())
        // .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_startup_system(setup)
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .label("game")
                .with_system(progress_game)
                .with_system(refresh_terrain)
                .with_system(check_collisions.run_if(should_check_collisions))
                .with_system(place_wall.run_if(should_place_wall))
                .with_system(place_cannon.run_if(should_place_cannon))
                .with_system(pick_target.run_if(should_pick_target)),
        )
        .add_system_to_stage(CoreStage::PostUpdate, expirations)
        .add_system_to_stage(CoreStage::PostUpdate, expanding)
        .add_system(bevy::window::close_on_esc)
        .add_loopless_state(Phase::default())
        .add_event::<TerrainModifiedEvent>()
        .insert_resource(ClearColor(Color::hex("152238").unwrap()))
        .init_resource::<Terrain>()
        .init_resource::<ActivePlayer>()
        .init_resource::<ActivePhase>()
        .run();
}

/// Condition checking if spacebar is pressed
/*
fn spacebar_pressed(kbd: Res<Input<KeyCode>>) -> bool {
    kbd.pressed(KeyCode::Space)
}
 */

fn should_place_wall(state: Res<CurrentState<Phase>>) -> bool {
    matches!(state.0, Phase::Fortify(_))
}

fn should_place_cannon(state: Res<CurrentState<Phase>>) -> bool {
    matches!(state.0, Phase::Arm(_))
}

fn should_pick_target(state: Res<CurrentState<Phase>>) -> bool {
    matches!(state.0, Phase::Target(_))
}

fn should_check_collisions(state: Res<CurrentState<Phase>>) -> bool {
    match &state.0 {
        Phase::Fortify(_) => true,
        Phase::Arm(_) => true,
        Phase::Target(_) => true,
    }
}

fn refresh_terrain(mut modified: EventReader<TerrainModifiedEvent>, mut terrain: ResMut<Terrain>) {
    for ev in modified.iter() {
        info!("terrain-modified {:?}", ev);

        let position = ev.0 .0;
        let structure = ev.1.clone();

        // let entity = create_structure(commands, )

        terrain.structure_layer.set(position, Some(structure));
    }
}

fn check_collisions(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    mut contact_force_events: EventReader<ContactForceEvent>,
    mut effects: ResMut<Assets<EffectAsset>>,
    projectiles: Query<Option<&RoundShot>>,
    transforms: Query<&Transform>,
    names: Query<&Name>,
) {
    for collision_event in collision_events.iter() {
        match collision_event {
            CollisionEvent::Started(first, second, _) => {
                let (target, projectile) = {
                    if projectiles
                        .get(*first)
                        .expect("Projectile check failed")
                        .is_some()
                    {
                        (second, first)
                    } else {
                        (first, second)
                    }
                };

                let showtime = transforms.get(*projectile).expect("No collision entity");

                commands.entity(*projectile).despawn_recursive();

                info!(
                    "collision: target={:?} projectile={:?}",
                    names.get(*target).map(|s| s.as_str()),
                    names.get(*projectile).map(|s| s.as_str())
                );

                let mut colors = Gradient::new();
                colors.add_key(0.0, Vec4::new(4.0, 4.0, 4.0, 1.0));
                colors.add_key(0.1, Vec4::new(4.0, 4.0, 0.0, 1.0));
                colors.add_key(0.9, Vec4::new(4.0, 0.0, 0.0, 1.0));
                colors.add_key(1.0, Vec4::new(4.0, 0.0, 0.0, 0.0));

                let mut sizes = Gradient::new();
                sizes.add_key(0.0, Vec2::splat(0.1));
                sizes.add_key(0.3, Vec2::splat(0.1));
                sizes.add_key(1.0, Vec2::splat(0.0));

                // TODO Leaking?
                let effect = effects.add(
                    EffectAsset {
                        name: "Firework".to_string(),
                        capacity: 32768,
                        spawner: Spawner::once(500.0.into(), true),
                        ..Default::default()
                    }
                    .init(PositionSphereModifier {
                        dimension: ShapeDimension::Volume,
                        radius: 0.25,
                        speed: 70_f32.into(),
                        center: Vec3::ZERO,
                    })
                    .init(ParticleLifetimeModifier { lifetime: 0.3 })
                    .update(LinearDragModifier { drag: 5. })
                    .update(AccelModifier {
                        accel: Vec3::new(0., -8., 0.),
                    })
                    .render(ColorOverLifetimeModifier { gradient: colors })
                    .render(SizeOverLifetimeModifier { gradient: sizes }),
                );

                commands
                    .spawn((
                        Name::new("Explosion"),
                        Expires::after(5.),
                        SpatialBundle {
                            transform: Transform::from_translation(showtime.translation),
                            ..default()
                        },
                    ))
                    .with_children(|child_builder| {
                        child_builder.spawn((
                            Name::new("Firework"),
                            ParticleEffectBundle {
                                effect: ParticleEffect::new(effect),
                                ..Default::default()
                            },
                        ));
                        child_builder.spawn((
                            Name::new("Explosion:Light"),
                            Expires::after(0.05),
                            PointLightBundle {
                                point_light: PointLight {
                                    intensity: 15000.0,
                                    shadows_enabled: true,
                                    ..default()
                                },
                                ..default()
                            },
                        ));
                    });
            }
            CollisionEvent::Stopped(_, _, _) => debug!("collision: {:?}", collision_event),
        }
    }

    for contact_force_event in contact_force_events.iter() {
        info!("contact force: {:?}", contact_force_event);
    }
}

#[derive(Debug, Clone)]
pub struct PickedCoordinates {
    name: String,
    coordinates: Coordinates,
    transform: Transform,
}

pub fn pick_coordinates(
    mut events: EventReader<PickingEvent>,
    targets: Query<(&Transform, &Name, &Coordinates), Without<Cannon>>,
) -> Option<PickedCoordinates> {
    for event in events.iter() {
        if let PickingEvent::Clicked(e) = event {
            let (transform, target_name, coordinates) =
                targets.get(*e).expect("Clicked entity not found?");

            info!(
                "pick-coordinate {:?} p={:?}",
                target_name.as_str(),
                &coordinates
            );

            return Some(PickedCoordinates {
                name: target_name.to_string(),
                coordinates: coordinates.clone(),
                transform: *transform,
            });
        }
    }

    None
}

pub fn progress_game(
    phase: Res<CurrentState<Phase>>,
    mut player: ResMut<ActivePlayer>,
    mut modified: EventReader<TerrainModifiedEvent>,
    mut commands: Commands,
) {
    for _event in modified.iter() {
        let before = &phase.0;
        let after = before.next();
        info!("{:?} -> {:?}", before, after);
        *player = ActivePlayer(after.player());
        commands.insert_resource(NextState(after));
    }
}

pub fn place_wall(
    player: Res<ActivePlayer>,
    events: EventReader<PickingEvent>,
    targets: Query<(&Transform, &Name, &Coordinates), Without<Cannon>>,
    mut modified: EventWriter<TerrainModifiedEvent>,
) {
    let picked = pick_coordinates(events, targets);
    if picked.is_none() {
        return;
    }

    let picked = picked.expect("No picked");

    info!("place-wall p={:?}", &picked);

    modified.send(TerrainModifiedEvent(
        picked.coordinates,
        Structure::Wall(Wall {
            player: player.0.clone(),
            entity: None,
        }),
    ))
}

pub fn place_cannon(
    player: Res<ActivePlayer>,
    events: EventReader<PickingEvent>,
    targets: Query<(&Transform, &Name, &Coordinates), Without<Cannon>>,
    mut modified: EventWriter<TerrainModifiedEvent>,
) {
    let picked = pick_coordinates(events, targets);
    if picked.is_none() {
        return;
    }

    let picked = picked.expect("No picked");

    info!("place-cannon p={:?}", &picked);

    modified.send(TerrainModifiedEvent(
        picked.coordinates,
        Structure::Cannon(Cannon {
            player: player.0.clone(),
            entity: None,
        }),
    ))
}

pub fn pick_target(
    events: EventReader<PickingEvent>,
    targets: Query<(&Transform, &Name, &Coordinates), Without<Cannon>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cannons: Query<(Entity, &mut Transform, &Player, &Name), With<Cannon>>,
) {
    let picked = pick_coordinates(events, targets);
    if picked.is_none() {
        return;
    }

    let picked = picked.expect("No picked");

    let mesh: Handle<Mesh> = meshes.add(shape::Icosphere::default().into());

    let black = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        perceptual_roughness: 0.3,
        ..default()
    });

    let (target, target_name) = (picked.transform, picked.name);
    let target = target.translation;

    match cannons.iter_mut().next() {
        Some((_e, mut cannon, player, cannon_name)) => {
            let zero_y = Vec3::new(1., 0., 1.);
            let direction = (target - cannon.translation) * zero_y;
            let distance = direction.length();
            let direction = direction.normalize();

            if distance < 1. {
                info!(%distance, "safety engaged");
                return;
            }

            let distance = distance - TILE_SIZE / 2.;
            let desired_time_of_flight =
                (distance / MAXIMUM_HORIZONTAL_DISTANCE) + MINIMUM_FLIGHT_TIME;
            // Vertical velocity to reach apex half way through.
            let vertical_velocity = GRAVITY * (desired_time_of_flight / 2.0);
            // Gotta go `distance` so however long that will take.
            let horizontal_velocity = distance / desired_time_of_flight;

            let mass = 20.0;

            // Final velocity is horizontal plus vertical.
            let velocity = (direction * horizontal_velocity) + Vec3::new(0., vertical_velocity, 0.);

            // This may need an offset to account for the mesh.
            // TODO Animate?
            let aim_angle = direction.angle_between(Vec3::new(-1., 0., 0.));
            cannon.rotation = Quat::from_rotation_y(aim_angle);

            let vertical_offset = Vec3::new(0., (WALL_HEIGHT / 2.0) + (ROUND_SHOT_SIZE / 2.0), 0.);
            let initial = cannon.translation + vertical_offset;

            info!(
                %distance, %velocity,
                "firing ({:?}) {} -> {} (initial={})", player, cannon_name.as_str(), target_name.as_str(), initial
            );

            commands.spawn((
                Name::new("Muzzle:Light"),
                Expires::after(0.05),
                PointLightBundle {
                    transform: Transform::from_translation(initial + Vec3::new(0., 1., 0.)),
                    point_light: PointLight {
                        intensity: 100.0,
                        shadows_enabled: true,
                        ..default()
                    },
                    ..default()
                },
            ));

            commands.spawn((
                Name::new("Projectile"),
                PbrBundle {
                    mesh,
                    material: black,
                    transform: Transform::from_translation(initial).with_scale(Vec3::new(
                        ROUND_SHOT_SIZE,
                        ROUND_SHOT_SIZE,
                        ROUND_SHOT_SIZE,
                    )),
                    ..default()
                },
                ColliderMassProperties::Mass(mass),
                RigidBody::Dynamic,
                ActiveEvents::COLLISION_EVENTS,
                RoundShot {},
                player.clone(),
                Collider::ball(ROUND_SHOT_SIZE / 2.),
                Velocity {
                    linvel: velocity,
                    angvel: Vec3::ZERO,
                },
            ));
        }
        None => warn!("no cannons"),
    }
}

pub fn load_terrain() -> Terrain {
    let mut terrain = Terrain::new((32, 32));
    terrain.ground_layer.set((4, 4), Ground::Grass);
    terrain.create_castle((4, 4), (4, 4), Player::One);
    terrain.create_castle((26, 26), (4, 4), Player::Two);
    terrain
}

#[derive(Debug)]
pub enum ConnectingWall {
    Isolated,
    Vertical,
    Horizontal,
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
            Around(_, (Some(Some(_)), _, Some(Some(_))), _) => Self::Horizontal,
            Around((_, Some(Some(_)), _), (_, _, _), (_, Some(Some(_)), _)) => Self::Vertical,
            Around((_, _, _), (_, _, _), (_, _, _)) => Self::Unknown,
        }
    }
}

#[allow(dead_code)]
enum QuickCamera {
    Normal,
    TopDown,
    CloseSide,
}

const DEFAULT_QUICK_CAMERA: QuickCamera = QuickCamera::Normal;

fn create_structure(
    commands: &mut Commands,
    terrain: &Res<Terrain>,
    asset_server: &Res<AssetServer>,
    grid: Vec2Usize,
    position: &Vec2,
    item: &Structure,
    wall_simple: &Handle<StandardMaterial>,
    wall_unknown: &Handle<Mesh>,
    wall_h: &Handle<Mesh>,
    wall_v: &Handle<Mesh>,
) {
    match item {
        Structure::Wall(wall) => {
            let around = &terrain.structure_layer.around(grid);

            let connecting: ConnectingWall = around.into();

            info!("{:?} {:?}", grid, connecting);

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
                    Coordinates(grid),
                    wall.player.clone(),
                    wall.clone(),
                ))
                .with_children(|parent| match connecting {
                    ConnectingWall::Isolated => {
                        parent.spawn(PbrBundle {
                            mesh: wall_unknown.clone(),
                            material: wall_simple.clone(),
                            ..default()
                        });
                    }
                    ConnectingWall::Vertical => {
                        parent.spawn(PbrBundle {
                            mesh: wall_v.clone(),
                            material: wall_simple.clone(),
                            ..default()
                        });
                    }
                    ConnectingWall::Horizontal => {
                        parent.spawn(PbrBundle {
                            mesh: wall_h.clone(),
                            material: wall_simple.clone(),
                            ..default()
                        });
                    }
                    ConnectingWall::Corner(angle) => {
                        parent.spawn(SceneBundle {
                            scene: asset_server.load("corner.glb#Scene0"),
                            transform: Transform::from_rotation(Quat::from_rotation_y(
                                -(angle as f32 * PI / 180.),
                            )),
                            ..default()
                        });
                    }
                    _ => {
                        parent.spawn(PbrBundle {
                            mesh: wall_unknown.clone(),
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
                    Coordinates(grid),
                    cannon.player.clone(),
                    cannon.clone(),
                ))
                .with_children(|parent| {
                    parent.spawn(SceneBundle {
                        scene: asset_server.load("cannon.glb#Scene0"),
                        transform: Transform::from_rotation(Quat::from_rotation_y(0.)),
                        ..default()
                    });
                });
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    terrain: Res<Terrain>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Name::new("Sun"),
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                illuminance: 5000.,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform {
                translation: Vec3::new(-6.0, 20.0, 0.0),
                rotation: Quat::from_rotation_x(-PI / 4.),
                ..default()
            },
            ..default()
        },
    ));

    commands.spawn((
        Camera3dBundle {
            transform: match DEFAULT_QUICK_CAMERA {
                QuickCamera::Normal => {
                    Transform::from_xyz(0.0, 18.0, -32.0).looking_at(Vec3::ZERO, Vec3::Y)
                }
                QuickCamera::TopDown => Transform::from_xyz(-12., 12., -12.)
                    .looking_at(Vec3::new(-12., 1., -12.), Vec3::Z),
                QuickCamera::CloseSide => Transform::from_xyz(-10., 1., -18.)
                    .looking_at(Vec3::new(-10., 1., -8.), Vec3::Y),
            },
            ..default()
        },
        PickingCameraBundle::default(),
    ));

    // Rigid body ground
    commands.spawn((
        Name::new("Ground"),
        TransformBundle::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        CollisionGroups::new(Group::all(), Group::all()),
        Collider::cuboid(20., 0.1, 20.),
    ));

    let ground = meshes.add(Mesh::from(shape::Box::new(
        TILE_SIZE * 0.95,
        GROUND_DEPTH,
        TILE_SIZE * 0.95,
    )));

    let dirt = materials.add(StandardMaterial {
        base_color: Color::BEIGE,
        perceptual_roughness: 1.0,
        ..default()
    });

    let grass = materials.add(StandardMaterial {
        base_color: Color::GREEN,
        perceptual_roughness: 1.0,
        ..default()
    });

    let water = materials.add(StandardMaterial {
        base_color: Color::BLUE,
        perceptual_roughness: 1.0,
        ..default()
    });

    for (grid, position, item) in terrain.ground_layer.layout() {
        commands.spawn((
            Name::new(format!("Ground{:?}", &grid)),
            PbrBundle {
                mesh: ground.clone(),
                material: match item {
                    Ground::Dirt => dirt.clone(),
                    Ground::Grass => grass.clone(),
                    Ground::Water => water.clone(),
                },
                transform: Transform::from_xyz(position.x, 0.0, position.y),
                ..default()
            },
            PickableBundle::default(),
            Coordinates(grid),
        ));
    }

    let wall_simple = materials.add(StandardMaterial {
        base_color: Color::hex(BRICK_COLOR).expect("BRICK_COLOR"),
        perceptual_roughness: 1.0,
        ..default()
    });
    let wall_unknown = meshes.add(Mesh::from(shape::Box::new(TILE_SIZE, TILE_SIZE, TILE_SIZE)));
    let wall_v = meshes.add(Mesh::from(shape::Box::new(
        WALL_WIDTH,
        WALL_HEIGHT,
        TILE_SIZE,
    )));
    let wall_h = meshes.add(Mesh::from(shape::Box::new(
        TILE_SIZE,
        WALL_HEIGHT,
        WALL_WIDTH,
    )));

    for (grid, position, item) in terrain.structure_layer.layout() {
        if let Some(item) = item {
            create_structure(
                &mut commands,
                &terrain,
                &asset_server,
                grid,
                &position,
                item,
                &wall_simple,
                &wall_unknown,
                &wall_h,
                &wall_v,
            )
        }
    }
}

#[derive(Component, Clone)]
pub struct Expandable {}

pub fn expanding(mut expandables: Query<(&mut Transform, &Expandable)>, timer: Res<Time>) {
    for (mut transform, _expandable) in &mut expandables {
        transform.scale += Vec3::splat(0.3) * timer.delta_seconds()
    }
}

#[derive(Component, Clone)]
pub struct Expires {
    lifetime: f32,
    expiration: Option<f32>,
}

impl Expires {
    pub fn after(lifetime: f32) -> Self {
        Self {
            lifetime,
            expiration: None,
        }
    }
}

pub fn expirations(
    mut commands: Commands,
    mut expires: Query<(Entity, &mut Expires, Option<&Name>)>,
    timer: Res<Time>,
) {
    for (entity, mut expires, name) in &mut expires {
        match expires.expiration {
            Some(expiration) => {
                if timer.elapsed_seconds() > expiration {
                    // https://bevy-cheatbook.github.io/features/parent-child.html#known-pitfalls
                    info!("expiring '{:?}'", name.map(|n| n.as_str()));
                    commands.entity(entity).despawn_recursive();
                }
            }
            None => {
                expires.expiration = Some(timer.elapsed_seconds() + expires.lifetime);
            }
        }
    }
}
