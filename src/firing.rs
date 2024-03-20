use bevy::{math::primitives, prelude::*};
use bevy_mod_picking::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{helpers, pick_coordinates};

use super::model::*;

pub struct FiringPlugin;

impl Plugin for FiringPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (pick_target.run_if(should_pick_target),));
    }
}

fn should_pick_target(state: Res<State<Phase>>) -> bool {
    matches!(state.get(), Phase::Target(_))
}

fn pick_target(
    events: EventReader<Pointer<Click>>,
    targets: Query<(&Transform, &Name, Option<&Coordinates>), Without<Cannon>>,
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
                helpers::Expires::after(0.05),
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
