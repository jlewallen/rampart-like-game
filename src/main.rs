use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
};
use bevy_hanabi::prelude::*;
use bevy_mod_picking::prelude::*;
use bevy_rapier3d::prelude::*;

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, window::WindowResolution};

mod building;
mod camera;
mod devel;
mod firing;
mod helpers;
mod model;
mod resources;
mod terrain;
mod ui;

use model::*;
use terrain::*;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(bevy::log::LogPlugin {
                    filter: "castle=debug,wgpu=error,naga=warn,bevy_hanabi=warn,bevy_winit=warn,bevy_window=warn"
                        .to_string(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Castle".to_string(),
                        resolution: WindowResolution::new(1024. + 256. + 32., 768.0),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(HanabiPlugin)
        .add_plugins(DefaultPickingPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugins(RapierDebugRenderPlugin::default())
        // .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(camera::CameraPlugin)
        .add_plugins(helpers::HelpersPlugin)
        .add_plugins(devel::DeveloperPlugin)
        .add_plugins(building::BuildingPlugin)
        .add_plugins(firing::FiringPlugin)
        .add_plugins(WireframePlugin)
        .add_plugins(TerrainPlugin)
        .add_systems(PreStartup, resources::load_structures)
        .add_systems(Update, progress_game)
        .add_systems(Update, (check_collisions.run_if(should_check_collisions),))
        .add_systems(PostUpdate, bevy::window::close_on_esc)
        .insert_resource(ClearColor(Color::hex("152238").unwrap()))
        // Wireframes can be configured with this resource. This can be changed at runtime.
        .insert_resource(WireframeConfig {
            // The global wireframe config enables drawing of wireframes on every mesh,
            // except those with `NoWireframe`. Meshes with `Wireframe` will always have a wireframe,
            // regardless of the global configuration.
            global: true,
            // Controls the default color of all wireframes. Used as the default color for global wireframes.
            // Can be changed per mesh using the `WireframeColor` component.
            default_color: Color::WHITE,
        })
        .insert_state(Phase::default())
        .add_event::<ConstructionEvent>()
        .init_resource::<ActivePlayer>()
        .run();
}

fn should_check_collisions(state: Res<State<Phase>>) -> bool {
    match &state.get() {
        Phase::Fortify(_) => false,
        Phase::Arm(_) => false,
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
                        helpers::Expires::after(5.),
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
                            helpers::Expires::after(0.05),
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
    targets: Query<(&Transform, &Name, Option<&Coordinates>), Without<Cannon>>,
) -> Option<PickedCoordinates> {
    for event in events.read() {
        info!("{:#?}", event);

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

        if let Some(coordinates) = coordinates {
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
    phase: Res<State<Phase>>,
    mut next_phase: ResMut<NextState<Phase>>,
    mut player: ResMut<ActivePlayer>,
    mut modified: EventReader<ConstructionEvent>,
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
