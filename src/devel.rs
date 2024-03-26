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
        app.add_systems(
            Update,
            manual_camera.run_if(in_state(CameraMode::AllTopDown)),
        )
        .add_systems(Update, developer_keyboard)
        .add_systems(Update, standard_gizmos);
    }
}

fn standard_gizmos(mut gizmos: Gizmos, lights: Query<(&PointLight, &GlobalTransform)>) {
    for (_light, transform) in lights.iter() {
        gizmos.sphere(transform.translation(), Quat::IDENTITY, 0.5, Color::RED);
    }
}

fn manual_camera(keys: Res<ButtonInput<KeyCode>>, mut camera: Query<(&Camera, &mut Transform)>) {
    let mut delta = Vec3::ZERO;

    if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::Numpad8) {
        delta += -Vec3::Z;
    }
    if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::Numpad2) {
        delta += Vec3::Z;
    }
    if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::Numpad4) {
        delta += -Vec3::X;
    }
    if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::Numpad6) {
        delta += Vec3::X;
    }
    if keys.pressed(KeyCode::Numpad7) {
        delta += -Vec3::Y;
    }
    if keys.pressed(KeyCode::Numpad1) {
        delta += Vec3::Y;
    }

    for (_, mut transform) in camera.iter_mut() {
        transform.translation += delta;
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
    mut config_store: ResMut<GizmoConfigStore>,
) {
    if keys.just_pressed(KeyCode::Space) {
        info!("{:?}", KeyCode::Space);
    }
    if keys.just_pressed(KeyCode::KeyE) {
        let setting = match expiration_control.get() {
            ExpirationControl::Running => ExpirationControl::Paused,
            ExpirationControl::Paused => ExpirationControl::Running,
        };
        info!("expirations-toggled: {:?}", setting);
        new_expiration_control.set(setting);
    }
    if keys.just_pressed(KeyCode::Digit1) {
        let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
        config.enabled = !config.enabled;
        info!("gizmo-config: {:?}", config.enabled);
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
