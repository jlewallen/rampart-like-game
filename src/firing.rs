use bevy::math::primitives;
use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use bevy_hanabi::{EffectAsset, Gradient};
use bevy_mod_picking::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::helpers::GamePlayLifetime;
use crate::terrain::Terrain;
use crate::{building::Cannon, helpers};

use super::model::*;

pub struct FiringPlugin;

impl Plugin for FiringPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ExplosionEvent>()
            .add_systems(Startup, setup)
            .add_systems(Update, pick_target.run_if(in_state(Activity::Firing)))
            .add_systems(Update, check_collisions.run_if(in_state(Activity::Firing)));
    }
}

pub trait Projectile {}

#[derive(Component, Clone, Debug)]
pub struct RoundShot {
    target: Vec3,
}

impl Projectile for RoundShot {}

#[derive(Debug, Clone)]
struct PickedCoordinates {
    transform: Transform,
}

fn get_picked_coordinates(mut events: EventReader<Pointer<Click>>) -> Option<PickedCoordinates> {
    for event in events.read() {
        if let Some(position) = event.event.hit.position {
            return Some(PickedCoordinates {
                transform: Transform::from_translation(position),
            });
        }
    }

    None
}

#[derive(Bundle)]
struct MuzzleFlashBundle {
    name: Name,
    expiration: helpers::Expires,
    light: PointLightBundle,
}

impl MuzzleFlashBundle {
    fn new(position: Vec3) -> Self {
        Self {
            name: Name::new("Muzzle:Flash"),
            expiration: helpers::Expires::after(0.05),
            light: PointLightBundle {
                transform: Transform::from_translation(position + Vec3::new(0., 1., 0.)),
                point_light: PointLight {
                    // 1,000,000 lumens is a very large "cinema light" capable of registering brightly at Bevy's
                    // default "very overcast day" exposure level. For "indoor lighting" with a lower exposure,
                    // this would be way too bright.
                    intensity: 1_000_000.0,
                    shadows_enabled: true,
                    ..default()
                },
                ..default()
            },
        }
    }
}

#[derive(Bundle)]
struct RoundShotBundle {
    name: Name,
    pbr: PbrBundle,
    mass: ColliderMassProperties,
    body: RigidBody,
    lifetime: GamePlayLifetime,
    active_events: ActiveEvents,
    projectile: RoundShot,
    player: Player,
    collider: Collider,
    velocity: Velocity,
}

impl RoundShotBundle {
    fn new(
        position: Vec3,
        target: Vec3,
        velocity: Vec3,
        mass: f32,
        player: Player,
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
    ) -> Self {
        Self {
            name: Name::new("Projectile:RoundShot"),
            pbr: PbrBundle {
                mesh,
                material,
                transform: Transform::from_translation(position).with_scale(Vec3::new(
                    ROUND_SHOT_DIAMETER,
                    ROUND_SHOT_DIAMETER,
                    ROUND_SHOT_DIAMETER,
                )),
                ..default()
            },
            mass: ColliderMassProperties::Mass(mass),
            body: RigidBody::Dynamic,
            lifetime: GamePlayLifetime,
            active_events: ActiveEvents::COLLISION_EVENTS,
            projectile: RoundShot { target },
            player,
            collider: Collider::ball(ROUND_SHOT_DIAMETER / 2.),
            velocity: Velocity {
                linvel: velocity,
                angvel: Vec3::ZERO,
            },
        }
    }
}

fn pick_target(
    events: EventReader<Pointer<Click>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cannons: Query<(Entity, &mut Transform, &Player), With<Cannon>>,
) {
    let picked: Option<PickedCoordinates> = get_picked_coordinates(events);
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

    let target = picked.transform.translation;

    match cannons.iter_mut().next() {
        Some((_e, mut cannon, player)) => {
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

            let vertical_offset =
                Vec3::new(0., (WALL_HEIGHT / 2.0) + (ROUND_SHOT_DIAMETER / 2.0), 0.);
            let initial = cannon.translation + vertical_offset;

            info!(%distance, %velocity, %initial, ?player, "firing");

            commands.spawn(MuzzleFlashBundle::new(initial));

            commands.spawn(RoundShotBundle::new(
                initial,
                target,
                velocity,
                mass,
                player.clone(),
                mesh,
                black,
            ));
        }
        None => warn!("no cannons"),
    }
}

#[derive(Resource)]
struct ExplosionResources {
    effect: Handle<EffectAsset>,
}

fn setup(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    asset_server: ResMut<AssetServer>,
) {
    let mut colors = Gradient::new();
    colors.add_key(0.0, Vec4::new(4.0, 4.0, 4.0, 1.0));
    colors.add_key(0.1, Vec4::new(4.0, 4.0, 0.0, 1.0));
    colors.add_key(0.9, Vec4::new(4.0, 0.0, 0.0, 1.0));
    colors.add_key(1.0, Vec4::new(4.0, 0.0, 0.0, 0.0));

    let mut sizes = Gradient::new();
    sizes.add_key(0.0, Vec2::splat(0.15));
    sizes.add_key(0.3, Vec2::splat(0.1));
    sizes.add_key(0.8, Vec2::splat(0.01));
    sizes.add_key(1.0, Vec2::splat(0.0));

    let mut module = Module::default();
    let init_position = SetPositionSphereModifier {
        dimension: ShapeDimension::Volume,
        center: module.lit(Vec3::ZERO),
        radius: module.lit(0.5),
    };
    let init_velocity = SetVelocitySphereModifier {
        center: module.lit(Vec3::ZERO),
        speed: module.lit(20.),
    };
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, module.lit(2.0));
    let update_accel = AccelModifier::new(module.lit(Vec3::new(0., -9.8, 0.)));
    let update_drag = LinearDragModifier::new(module.lit(1.5));

    let circle: Handle<Image> = asset_server.load("circle.png");

    let particle_texture_modifier = ParticleTextureModifier {
        texture: circle,
        sample_mapping: ImageSampleMapping::Modulate,
    };

    let effect = effects.add(
        EffectAsset::new(256, Spawner::once(256.0.into(), true), module)
            .init(init_position)
            .init(init_velocity)
            .init(init_lifetime)
            .update(update_drag)
            .update(update_accel)
            .render(ColorOverLifetimeModifier { gradient: colors })
            .render(SizeOverLifetimeModifier {
                gradient: sizes,
                screen_space_size: false,
            })
            .render(particle_texture_modifier),
    );

    // Hack! For some reason the first one of these spawned never does anything.
    // I believe there's something going on with `Spawner::once` and the
    // initialization of the effect. Either way, this fixes things and will be
    // easy to delete if I can ever figure out what's going on. I spawn well
    // outside of any visible area.
    commands.spawn((
        Name::new("Explosion:Burst"),
        ParticleEffectBundle {
            effect: ParticleEffect::new(effect.clone())
                // You can actually change this so nothing is emitted and it'll still fix the issue.
                .with_spawner(Spawner::once(256.0.into(), true)),
            transform: Transform::from_translation(Vec3::Y * 1000.0),
            ..Default::default()
        },
    ));

    commands.insert_resource(ExplosionResources { effect });

    info!("explosions-ready");
}

fn check_collisions(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    mut contact_force_events: EventReader<ContactForceEvent>,
    mut explosions: EventWriter<ExplosionEvent>,
    _terrain: Query<&Terrain>,
    projectiles: Query<Option<&RoundShot>>,
    transforms: Query<&Transform>,
    names: Query<&Name>,
    resources: Res<ExplosionResources>,
) {
    for collision_event in collision_events.read() {
        match collision_event {
            CollisionEvent::Started(first, second, _) => {
                let (target, projectile, round_shot) = match (
                    projectiles.get(*first).expect("Projectile check failed"),
                    projectiles.get(*second).expect("Projectile check failed"),
                ) {
                    (None, Some(projectile)) => (first, second, projectile),
                    (Some(projectile), None) => (second, first, projectile),
                    (Some(_), Some(_)) => todo!(),
                    (None, None) => todo!(),
                };

                let showtime = transforms.get(*projectile).expect("No collision entity");
                let collision_at = showtime.translation;
                let explosion_at = round_shot.target;

                explosions.send(ExplosionEvent::new(explosion_at));

                commands.entity(*projectile).despawn_recursive();

                info!(
                    %collision_at,
                    %explosion_at,
                    "collision: target={:?} projectile={:?} ({:?})",
                    names.get(*target).ok().map(|s| s.as_str()),
                    names.get(*projectile).ok().map(|s| s.as_str()),
                    explosion_at - collision_at
                );

                commands
                    .spawn((
                        Name::new("Explosion"),
                        helpers::Expires::after(2.5),
                        SpatialBundle {
                            transform: Transform::from_translation(explosion_at),
                            ..default()
                        },
                    ))
                    .with_children(|child_builder| {
                        child_builder.spawn((
                            Name::new("Explosion:Burst"),
                            ParticleEffectBundle {
                                effect: ParticleEffect::new(resources.effect.clone()),
                                transform: Transform::IDENTITY,
                                ..Default::default()
                            },
                        ));
                        child_builder.spawn((
                            Name::new("Explosion:Light"),
                            helpers::Expires::after(0.05),
                            PointLightBundle {
                                transform: Transform::from_translation(Vec3::Y * 1.),
                                point_light: PointLight {
                                    // 1,000,000 lumens is a very large "cinema light" capable of registering brightly at Bevy's
                                    // default "very overcast day" exposure level. For "indoor lighting" with a lower exposure,
                                    // this would be way too bright.
                                    intensity: 1_000_000.0,
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

#[derive(Clone, Debug)]
pub struct ExplosionEvent {
    #[allow(dead_code)]
    world: Vec3,
}

impl Event for ExplosionEvent {}

impl ExplosionEvent {
    pub fn new(world: Vec3) -> Self {
        Self { world }
    }

    #[allow(dead_code)]
    pub fn world(&self) -> Vec3 {
        self.world
    }
}
