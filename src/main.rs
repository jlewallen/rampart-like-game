use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::PresentMode,
};
use bevy_mod_picking::{DefaultPickingPlugins, PickableBundle, PickingCameraBundle, PickingEvent};
use std::f32::consts::PI;

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

impl<T: Default + Clone> WorldGeometry<T> {
    pub fn new(size: Vec2Usize) -> Self {
        Self {
            size,
            map: vec![T::default(); size.0 * size.1],
        }
    }

    pub fn layout(&self) -> Vec<(Vec2, &T)> {
        self.map
            .iter()
            .enumerate()
            .map(|(index, value)| (self.index_to_coordindates(index), value))
            .collect()
    }

    pub fn set(&mut self, c: Vec2Usize, value: T) {
        let index = self.coordinates_to_index(c);
        self.map[index] = value;
    }

    fn index_to_coordindates(&self, index: usize) -> Vec2 {
        let x: f32 = ((index % self.size.0) as f32 - (self.size.0 / 2) as f32) * 1.0 + 0.5;
        let y: f32 = ((index / self.size.1) as f32 - (self.size.1 / 2) as f32) * 1.0 + 0.5;
        Vec2::new(x, y)
    }

    fn coordinates_to_index(&self, c: Vec2Usize) -> usize {
        c.1 * self.size.1 + (c.0)
    }
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct Wall {
    orientation: u32,
    integrity: f32,
}

#[derive(Clone, Debug)]
pub struct Cannon {
    integrity: f32,
}

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

pub struct RoundShot {}

impl Projectile for RoundShot {}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                present_mode: PresentMode::AutoNoVsync, // Reduce input latency
                ..default()
            },
            ..default()
        }))
        .add_plugins(DefaultPickingPlugins)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_startup_system(setup)
        .add_system_to_stage(CoreStage::PostUpdate, print_events)
        .run();
}

pub fn print_events(mut events: EventReader<PickingEvent>) {
    for event in events.iter() {
        match event {
            PickingEvent::Selection(e) => info!("selection: {:?}", e),
            PickingEvent::Clicked(e) => info!("clicked: {:?}", e),
            PickingEvent::Hover(_) => {}
        }
    }
}

pub fn load_terrain() -> Terrain {
    let mut terrain = Terrain::new((32, 32));
    terrain.ground_layer.set((4, 4), Ground::Grass);
    terrain
        .structure_layer
        .set((4, 5), Some(Structure::Cannon(Cannon { integrity: 1. })));
    terrain
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let terrain = load_terrain();

    let ground = meshes.add(Mesh::from(shape::Cube { size: 0.95 }));

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

    for (position, item) in terrain.ground_layer.layout() {
        println!("{:?} {:?}", position, item);

        commands.spawn((
            PbrBundle {
                mesh: ground.clone(),
                material: match item {
                    Ground::Dirt => dirt.clone(),
                    Ground::Grass => grass.clone(),
                    Ground::Water => water.clone(),
                },
                transform: Transform::from_scale(Vec3::new(1., 0.2, 1.))
                    * Transform::from_xyz(position.x, 0.0, position.y),
                ..default()
            },
            PickableBundle::default(),
        ));
    }

    let structure = meshes.add(Mesh::from(shape::Cube { size: 0.6 }));

    let wall = materials.add(StandardMaterial {
        base_color: Color::FUCHSIA,
        perceptual_roughness: 1.0,
        ..default()
    });

    let cannon = materials.add(StandardMaterial {
        base_color: Color::RED,
        perceptual_roughness: 0.3,
        ..default()
    });

    for (position, item) in terrain.structure_layer.layout() {
        if let Some(item) = item {
            println!("{:?} {:?}", position, item);

            commands.spawn((
                PbrBundle {
                    mesh: structure.clone(),
                    material: match item {
                        Structure::Wall(_) => wall.clone(),
                        Structure::Cannon(_) => cannon.clone(),
                    },
                    transform: Transform::from_scale(Vec3::new(1., 1., 1.))
                        * Transform::from_xyz(position.x, 0.4, position.y),
                    ..default()
                },
                PickableBundle::default(),
            ));
        }
    }

    /*
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 5.0, 0.0),
        ..default()
    });
    */

    const HALF_SIZE: f32 = 10.0;
    commands.spawn(DirectionalLightBundle {
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
    });

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 18.0, -32.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        PickingCameraBundle::default(),
    ));
}
