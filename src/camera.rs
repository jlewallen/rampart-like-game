use bevy::prelude::*;
use bevy_rts_camera::{RtsCamera, RtsCameraControls, RtsCameraPlugin};

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, States)]
pub enum CameraMode {
    #[default]
    Normal,
    AllTopDown,
    AllAngled,
}

fn setup_camera(
    mut commands: Commands,
    existing: Query<(Entity, &Camera)>,
    mode: Res<State<CameraMode>>,
) {
    info!("setup-camera");

    for (existing, _) in existing.iter() {
        commands.entity(existing).despawn_recursive();
    }

    match mode.get() {
        CameraMode::Normal => commands.spawn((
            Camera3dBundle::default(),
            RtsCamera::default(),
            RtsCameraControls::default(),
        )),
        CameraMode::AllTopDown => commands.spawn((Camera3dBundle {
            transform: Transform::from_xyz(0., 84., 0.).looking_at(Vec3::new(0., 0., 0.), -Vec3::Z),
            ..default()
        },)),
        CameraMode::AllAngled => commands.spawn((Camera3dBundle {
            transform: Transform::from_xyz(0., 64., 32.).looking_at(Vec3::new(0., 0., 6.), Vec3::Y),
            ..default()
        },)),
    };
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RtsCameraPlugin)
            .insert_state(CameraMode::Normal)
            .add_systems(OnEnter(CameraMode::Normal), setup_camera)
            .add_systems(OnEnter(CameraMode::AllTopDown), setup_camera)
            .add_systems(OnEnter(CameraMode::AllAngled), setup_camera);
    }
}
