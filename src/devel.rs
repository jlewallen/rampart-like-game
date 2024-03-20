use bevy::prelude::*;

pub struct DeveloperPlugin;

impl Plugin for DeveloperPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, developer_keyboard);
    }
}

fn developer_keyboard(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Space) {
        info!("{:?}", KeyCode::Space);
    }
}
