use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*, window::PresentMode};
use bevy_hanabi::prelude::*;
use bevy_mod_picking::{
    CustomHighlightPlugin, DefaultHighlighting, DefaultPickingPlugins, PickableBundle,
    PickingCameraBundle, PickingEvent,
};
use bevy_rapier3d::prelude::*;
use std::f32::consts::*;

pub type Vec2Usize = (usize, usize);

pub struct Player(u32);

pub enum Phase {
    Extend(Player),
    Arm(Player),
    Target(Player),
}

#[derive(Debug)]
pub struct WorldGeometry<T> {
    size: Vec2Usize,
    map: Vec<T>,
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

    pub fn around(&self, c: Vec2Usize) {
        todo!()
    }

    fn index_to_grid(&self, index: usize) -> Vec2Usize {
        (index % self.size.0, index / self.size.1)
    }

    fn index_to_coordindates(&self, index: usize) -> Vec2 {
        let c = self.index_to_grid(index);
        let x: f32 = (c.0 as f32 - (self.size.0 / 2) as f32) * 1.0 + 0.5;
        let y: f32 = (c.1 as f32 - (self.size.1 / 2) as f32) * 1.0 + 0.5;
        Vec2::new(x, y)
    }

    fn coordinates_to_index(&self, c: Vec2Usize) -> usize {
        c.1 * self.size.1 + (c.0)
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
pub struct Wall {}

#[derive(Component, Clone, Debug)]
pub struct Cannon {}

#[derive(Clone, Debug)]
pub enum Structure {
    Wall(Wall),
    Cannon(Cannon),
}

#[derive(Debug)]
pub struct Terrain {
    ground_layer: WorldGeometry<Ground>,
    structure_layer: WorldGeometry<Option<Structure>>,
}

impl Terrain {
    pub fn new(size: Vec2Usize) -> Self {
        Self {
            ground_layer: WorldGeometry::new(size),
            structure_layer: WorldGeometry::new(size),
        }
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
        .add_system_to_stage(CoreStage::PostUpdate, check_collisions)
        .add_system_to_stage(CoreStage::PostUpdate, process_picking)
        .add_system_to_stage(CoreStage::PostUpdate, expirations)
        .add_system_to_stage(CoreStage::PostUpdate, expanding)
        .add_system(bevy::window::close_on_esc)
        .insert_resource(ClearColor(Color::hex("152238").unwrap()))
        .run();
}

fn check_collisions(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    mut contact_force_events: EventReader<ContactForceEvent>,
    mut effects: ResMut<Assets<EffectAsset>>,
    transforms: Query<&Transform>,
) {
    for collision_event in collision_events.iter() {
        info!("collision: {:?}", collision_event);

        match collision_event {
            CollisionEvent::Started(_structure, projectile, _) => {
                let showtime = transforms.get(*projectile).expect("No collision entity");

                commands.entity(*projectile).despawn();

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
            CollisionEvent::Stopped(_, _, _) => {}
        }
    }

    for contact_force_event in contact_force_events.iter() {
        info!("contact force: {:?}", contact_force_event);
    }
}

pub fn process_picking(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventReader<PickingEvent>,
    targets: Query<(&Transform, &Name)>,
    cannons: Query<(Entity, &Transform, &Name), With<Cannon>>,
) {
    let mesh: Handle<Mesh> = meshes.add(shape::Icosphere::default().into());

    let black = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        perceptual_roughness: 0.3,
        ..default()
    });

    for event in events.iter() {
        match event {
            PickingEvent::Selection(e) => info!("selection: {:?}", e),
            PickingEvent::Clicked(e) => {
                let (target, target_name) = targets.get(*e).expect("Clicked entity not found?");
                let target = target.translation;

                match cannons.iter().next() {
                    Some((_e, cannon, cannon_name)) => {
                        let zero_y = Vec3::new(1., 0., 1.);
                        let direction = (target - cannon.translation) * zero_y;
                        let distance = direction.length();
                        let direction = direction.normalize();

                        // We base all the math on a desired time of flight that
                        // looks appropriate for the distance.
                        const MAXIMUM_HORIZONTAL_DISTANCE: f32 = 35.0;
                        const MINIMUM_FLIGHT_TIME: f32 = 1.0;
                        const GRAVITY: f32 = 9.8;

                        let desired_time_of_flight =
                            (distance / MAXIMUM_HORIZONTAL_DISTANCE) + MINIMUM_FLIGHT_TIME;
                        // Vertical velocity to reach apex half way through.
                        let vertical_velocity = GRAVITY * (desired_time_of_flight / 2.0);
                        // Gotta go `distance` so however long that will take.
                        let horizontal_velocity = distance / desired_time_of_flight;

                        let mass = 20.0;

                        // Final velocity is horizontal plus vertical.
                        let velocity = (direction * horizontal_velocity)
                            + Vec3::new(0., vertical_velocity, 0.);

                        info!(
                            %distance, %velocity,
                            "firing {} -> {}", cannon_name.as_str(), target_name.as_str(),
                        );

                        let size = 0.25;

                        commands.spawn((
                            Name::new("Projectile"),
                            PbrBundle {
                                mesh: mesh.clone(),
                                material: black.clone(),
                                transform: Transform::from_translation(cannon.translation)
                                    .with_scale(Vec3::new(size, size, size)),
                                ..default()
                            },
                            ColliderMassProperties::Mass(mass),
                            RigidBody::Dynamic,
                            ActiveEvents::COLLISION_EVENTS,
                            RoundShot {},
                            Collider::ball(size / 2.),
                            Velocity {
                                linvel: velocity,
                                angvel: Vec3::ZERO,
                            },
                        ));
                    }
                    None => warn!("no cannons"),
                }
            }
            PickingEvent::Hover(_) => {}
        }
    }
}

pub fn load_terrain() -> Terrain {
    let mut terrain = Terrain::new((32, 32));
    terrain.ground_layer.set((4, 4), Ground::Grass);

    terrain
        .structure_layer
        .outline((2, 2), (6, 6), Some(Structure::Wall(Wall {})));

    terrain
        .structure_layer
        .set((4, 4), Some(Structure::Cannon(Cannon {})));

    terrain
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut _effects: ResMut<Assets<EffectAsset>>,
) {
    const HALF_SIZE: f32 = 10.0;
    commands.spawn((
        Name::new("Sun"),
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                illuminance: 5000.,
                shadow_projection: OrthographicProjection {
                    left: -HALF_SIZE,
                    right: HALF_SIZE,
                    bottom: -HALF_SIZE,
                    top: HALF_SIZE,
                    near: -10.0 * HALF_SIZE,
                    far: 10.0 * HALF_SIZE,
                    ..default()
                },
                shadows_enabled: true,
                ..default()
            },
            transform: Transform {
                translation: Vec3::new(0.0, 2.0, 0.0),
                rotation: Quat::from_rotation_x(-PI / 4.),
                ..default()
            },
            ..default()
        },
    ));

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 18.0, -32.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        PickingCameraBundle::default(),
    ));

    let terrain = load_terrain();

    // Rigid body ground
    commands.spawn((
        TransformBundle::from(Transform::from_xyz(0.0, 0.0, 0.0)),
        Collider::cuboid(20., 0.1, 20.),
    ));

    let ground = meshes.add(Mesh::from(shape::Box::new(0.95, 0.2, 0.95)));

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
        ));
    }

    let structure = meshes.add(Mesh::from(shape::Box::new(0.6, 0.6, 0.6)));

    let wall_simple = materials.add(StandardMaterial {
        base_color: Color::FUCHSIA,
        perceptual_roughness: 1.0,
        ..default()
    });

    let cannon_simple = materials.add(StandardMaterial {
        base_color: Color::RED,
        perceptual_roughness: 0.3,
        ..default()
    });

    for (grid, position, item) in terrain.structure_layer.layout() {
        if let Some(item) = item {
            match item {
                Structure::Wall(wall) => {
                    commands.spawn((
                        Name::new(format!("Wall{:?}", &grid)),
                        PbrBundle {
                            mesh: structure.clone(),
                            material: wall_simple.clone(),
                            transform: Transform::from_xyz(position.x, 0.4, position.y),
                            ..default()
                        },
                        PickableBundle::default(),
                        // We need to be able to exclude this from colliding with its own projectiles.
                        // Collider::cuboid(0.3, 0.3, 0.3),
                        wall.clone(),
                    ));
                }
                Structure::Cannon(cannon) => {
                    commands.spawn((
                        Name::new(format!("Cannon{:?}", &grid)),
                        PbrBundle {
                            mesh: structure.clone(),
                            material: cannon_simple.clone(),
                            transform: Transform::from_xyz(position.x, 0.4, position.y),
                            ..default()
                        },
                        PickableBundle::default(),
                        // We need to be able to exclude this from colliding with its own projectiles.
                        // Collider::cuboid(0.3, 0.3, 0.3),
                        cannon.clone(),
                    ));
                }
            }
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
                    info!("expiring {:?} '{:?}'", entity, name.map(|n| n.as_str()));
                    commands.entity(entity).despawn_recursive();
                }
            }
            None => {
                expires.expiration = Some(timer.elapsed_seconds() + expires.lifetime);
            }
        }
    }
}
