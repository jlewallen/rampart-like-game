#[allow(unused_imports)]
use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin, math::primitives, prelude::*, window::WindowResolution,
};
use bevy_hanabi::prelude::*;
use bevy_mod_picking::{
    events::{Click, Pointer},
    DefaultPickingPlugins, PickableBundle,
};
use bevy_rapier3d::prelude::*;
use bevy_rts_camera::{RtsCamera, RtsCameraControls, RtsCameraPlugin};
use std::f32::consts::*;

mod model;
mod resources;
mod ui;

use model::*;
use resources::Structures;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Castle".to_string(),
                resolution: WindowResolution::new(1024. + 256. + 32., 768.0),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(HanabiPlugin)
        .add_plugins(
            DefaultPickingPlugins, /*
                                   .set(HighlightPlugin::<StandardMaterial> {
                                       highlighting_default: |mut assets| GlobalHighlight {
                                           hovered: assets.add(Color::rgb(0.35, 0.35, 0.35).into()),
                                           pressed: assets.add(Color::rgb(0.35, 0.75, 0.35).into()),
                                           selected: assets.add(Color::rgb(0.35, 0.35, 0.75).into()),
                                       },
                                   })
                                   .set(HighlightPlugin::<ColorMaterial> {
                                       highlighting_default: |mut assets| GlobalHighlight {
                                           hovered: assets.add(Color::rgb(0.35, 0.35, 0.35).into()),
                                           pressed: assets.add(Color::rgb(0.35, 0.75, 0.35).into()),
                                           selected: assets.add(Color::rgb(0.35, 0.35, 0.75).into()),
                                       },
                                   }),
                                   */
        )
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugins(RapierDebugRenderPlugin::default())
        // .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(RtsCameraPlugin)
        .add_systems(PreStartup, resources::load_structures)
        .add_systems(Startup, setup)
        .add_systems(Update, progress_game)
        .add_systems(Update, refresh_terrain)
        .add_systems(Update, (check_collisions.run_if(should_check_collisions),))
        // Resources for these won't exist until later.
        .add_systems(Update, (place_wall.run_if(should_place_wall),))
        .add_systems(Update, (place_cannon.run_if(should_place_cannon),))
        .add_systems(Update, (pick_target.run_if(should_pick_target),))
        .add_systems(PostUpdate, expirations)
        .add_systems(PostUpdate, expanding)
        .add_systems(Update, bevy::window::close_on_esc)
        .insert_state(Phase::default())
        .add_event::<TerrainModifiedEvent>()
        .insert_resource(ClearColor(Color::hex("152238").unwrap()))
        .init_resource::<Terrain>()
        .init_resource::<ActivePlayer>()
        .init_resource::<EntityLayer>()
        .run();
}

fn should_place_wall(state: Res<State<Phase>>) -> bool {
    matches!(state.get(), Phase::Fortify(_))
}

fn should_place_cannon(state: Res<State<Phase>>) -> bool {
    matches!(state.get(), Phase::Arm(_))
}

fn should_pick_target(state: Res<State<Phase>>) -> bool {
    matches!(state.get(), Phase::Target(_))
}

fn should_check_collisions(state: Res<State<Phase>>) -> bool {
    match &state.get() {
        Phase::Fortify(_) => true,
        Phase::Arm(_) => true,
        Phase::Target(_) => true,
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
    for collision_event in collision_events.read() {
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

                // Create a new expression module
                let mut module = Module::default();
                let position = SetPositionSphereModifier {
                    dimension: ShapeDimension::Volume,
                    center: module.lit(Vec3::ZERO),
                    radius: module.lit(0.25),
                };

                let lifetime = module.lit(0.3);
                let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

                let accel = module.lit(Vec3::new(0., -8., 0.));
                let update_accel = AccelModifier::new(accel);

                let update_drag = LinearDragModifier::new(module.lit(5.));

                // TODO Leaking?
                let effect = effects.add(
                    EffectAsset::new(32768, Spawner::once(500.0.into(), true), module)
                        .init(position)
                        .init(init_lifetime)
                        .update(update_drag)
                        .update(update_accel)
                        .render(ColorOverLifetimeModifier { gradient: colors })
                        .render(SizeOverLifetimeModifier {
                            gradient: sizes,
                            screen_space_size: true,
                        }),
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
            CollisionEvent::Stopped(_, _, _) => debug!("collision(stopped): {:?}", collision_event),
        }
    }

    for contact_force_event in contact_force_events.read() {
        info!("contact force: {:?}", contact_force_event);
    }
}

#[derive(Debug, Clone)]
pub struct PickedCoordinates {
    name: String,
    coordinates: Coordinates,
    transform: Transform,
}

fn pick_coordinates(
    mut events: EventReader<Pointer<Click>>,
    targets: Query<(&Transform, &Name, &Coordinates), Without<Cannon>>,
) -> Option<PickedCoordinates> {
    for event in events.read() {
        let target = targets.get(event.target).ok();

        let Some((transform, target_name, coordinates)) = target else {
            info!("pick-coordinate no target");
            return None;
        };

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

    None
}

pub fn progress_game(
    phase: Res<State<Phase>>,
    mut next_phase: ResMut<NextState<Phase>>,
    mut player: ResMut<ActivePlayer>,
    mut modified: EventReader<TerrainModifiedEvent>,
    mut _commands: Commands,
) {
    for event in modified.read() {
        println!("{:?}", event);
        println!("{:?}", phase);
        let before = &phase.get();
        let after = before.next();
        info!("{:?} -> {:?}", before, after);
        *player = ActivePlayer::new(after.player());
        next_phase.set(after);
    }
}

fn refresh_terrain(
    mut commands: Commands,
    mut modified: EventReader<TerrainModifiedEvent>,
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

pub fn place_wall(
    player: Res<ActivePlayer>,
    events: EventReader<Pointer<Click>>,
    targets: Query<(&Transform, &Name, &Coordinates), Without<Cannon>>,
    mut modified: EventWriter<TerrainModifiedEvent>,
) {
    let picked = pick_coordinates(events, targets);
    if picked.is_none() {
        return;
    }

    let picked = picked.expect("No picked");

    info!("place-wall p={:?}", &picked);

    modified.send(TerrainModifiedEvent::new(
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
    targets: Query<(&Transform, &Name, &Coordinates), Without<Cannon>>,
    mut modified: EventWriter<TerrainModifiedEvent>,
) {
    let picked = pick_coordinates(events, targets);
    if picked.is_none() {
        return;
    }

    let picked = picked.expect("No picked");

    info!("place-cannon p={:?}", &picked);

    modified.send(TerrainModifiedEvent::new(
        picked.coordinates,
        Structure::Cannon(Cannon {
            player: player.player().clone(),
            entity: None,
        }),
    ));
}

pub fn pick_target(
    events: EventReader<Pointer<Click>>,
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

    let mesh: Handle<Mesh> = meshes.add(primitives::Sphere::default());

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

#[allow(dead_code)]
enum CameraMode {
    Rts,
    Normal,
    TopDown,
    CloseSide,
}

impl Default for CameraMode {
    fn default() -> Self {
        CameraMode::Rts
    }
}

impl CameraMode {}

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

            info!("create-structure {:?} {:?}", grid, connecting);

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
                ))
                .with_children(|parent| match connecting {
                    ConnectingWall::Isolated => {
                        parent.spawn(PbrBundle {
                            mesh: structures.unknown.clone(),
                            material: structures.simple.clone(),
                            ..default()
                        });
                    }
                    ConnectingWall::Vertical => {
                        parent.spawn(PbrBundle {
                            mesh: structures.v.clone(),
                            material: structures.simple.clone(),
                            ..default()
                        });
                    }
                    ConnectingWall::Horizontal => {
                        parent.spawn(PbrBundle {
                            mesh: structures.h.clone(),
                            material: structures.simple.clone(),
                            ..default()
                        });
                    }
                    ConnectingWall::Corner(angle) => {
                        parent.spawn(SceneBundle {
                            scene: structures.corner.clone(),
                            transform: Transform::from_rotation(Quat::from_rotation_y(
                                -(angle as f32 * PI / 180.),
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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut entities: ResMut<EntityLayer>,
    terrain: Res<Terrain>,
    structures: Res<Structures>,
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

    match CameraMode::default() {
        CameraMode::Rts => commands.spawn((
            Camera3dBundle::default(),
            RtsCamera::default(),
            RtsCameraControls::default(),
        )),
        CameraMode::Normal => commands.spawn((Camera3dBundle {
            transform: Transform::from_xyz(0.0, 18.0, -32.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },)),
        CameraMode::TopDown => commands.spawn((Camera3dBundle {
            transform: Transform::from_xyz(-12., 12., -12.)
                .looking_at(Vec3::new(-12., 1., -12.), Vec3::Z),
            ..default()
        },)),
        CameraMode::CloseSide => commands.spawn((Camera3dBundle {
            transform: Transform::from_xyz(-10., 1., -18.)
                .looking_at(Vec3::new(-10., 1., -8.), Vec3::Y),
            ..default()
        },)),
    };

    // Rigid body ground
    commands.spawn((
        Name::new("Ground"),
        TransformBundle::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        CollisionGroups::new(Group::all(), Group::all()),
        Collider::cuboid(20., 0.1, 20.),
        bevy_rts_camera::Ground,
    ));

    let ground = meshes.add(Mesh::from(primitives::Cuboid::new(
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
            Coordinates::new(grid),
        ));
    }

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
                    info!("expiring '{:?}'", name);
                    commands.entity(entity).despawn_recursive();
                }
            }
            None => {
                expires.expiration = Some(timer.elapsed_seconds() + expires.lifetime);
            }
        }
    }
}

#[derive(Resource)]
pub struct EntityLayer(WorldGeometry<Option<Vec<Entity>>>);

impl EntityLayer {
    pub fn new(size: Vec2Usize) -> Self {
        Self(WorldGeometry::new(size))
    }
}

impl FromWorld for EntityLayer {
    fn from_world(_world: &mut World) -> Self {
        Self::new((32, 32))
    }
}

impl FromWorld for Terrain {
    fn from_world(_world: &mut World) -> Self {
        load_terrain((32, 32))
    }
}
