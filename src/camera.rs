use bevy::prelude::*;
use bevy_rts_camera::{RtsCamera, RtsCameraControls, RtsCameraPlugin};

#[allow(dead_code)]
#[derive(Default)]
enum CameraMode {
    #[default]
    Rts,
    Normal,
    TopDown,
    CloseSide,
}

impl CameraMode {}

fn setup_camera(mut commands: Commands) {
    match CameraMode::default() {
        CameraMode::Rts => commands.spawn((
            Camera3dBundle::default(),
            RtsCamera::default(),
            RtsCameraControls::default(),
        )),
        CameraMode::Normal => commands.spawn((Camera3dBundle {
            transform: Transform::from_xyz(0.0, 22.0, -32.0).looking_at(Vec3::ZERO, Vec3::Y),
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
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RtsCameraPlugin)
            .add_systems(Startup, setup_camera);
    }
}
