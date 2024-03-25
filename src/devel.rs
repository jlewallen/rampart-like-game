use bevy::{pbr::wireframe::WireframeConfig, prelude::*};

use crate::{
    camera::CameraMode,
    helpers::ExpirationControl,
    model::{Activity, AppState},
};

// .add_plugins(RapierDebugRenderPlugin::default())
// .add_plugins(LogDiagnosticsPlugin::default())

pub struct DeveloperPlugin;

impl Plugin for DeveloperPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, developer_keyboard);
    }
}

fn developer_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    camera_mode: Res<State<CameraMode>>,
    expiration_control: Res<State<ExpirationControl>>,
    mut app_state: ResMut<NextState<AppState>>,
    mut new_camera_mode: ResMut<NextState<CameraMode>>,
    mut activity: ResMut<NextState<Activity>>,
    mut wireframe_config: ResMut<WireframeConfig>,
    mut new_expiration_control: ResMut<NextState<ExpirationControl>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        info!("{:?}", KeyCode::Space);
    }
    if keys.just_pressed(KeyCode::KeyE) {
        match expiration_control.get() {
            ExpirationControl::Running => new_expiration_control.set(ExpirationControl::Paused),
            ExpirationControl::Paused => new_expiration_control.set(ExpirationControl::Running),
        }
    }
    if keys.just_pressed(KeyCode::KeyR) {
        info!("resetting");
        app_state.set(AppState::Menu);
    }
    if keys.just_pressed(KeyCode::KeyO) {
        info!("observing");
        activity.set(Activity::Observing);
    }
    if keys.just_pressed(KeyCode::KeyF) {
        info!("firing");
        activity.set(Activity::Firing);
    }
    if keys.just_pressed(KeyCode::KeyB) {
        info!("building");
        activity.set(Activity::Building);
    }
    if keys.just_pressed(KeyCode::KeyC) {
        match camera_mode.get() {
            CameraMode::Normal => new_camera_mode.set(CameraMode::AllTopDown),
            CameraMode::AllTopDown => new_camera_mode.set(CameraMode::AllAngled),
            CameraMode::AllAngled => new_camera_mode.set(CameraMode::Normal),
        }
    }
    if keys.just_pressed(KeyCode::KeyW) {
        info!("toggle-wireframe");
        wireframe_config.global = !wireframe_config.global;
    }
}
