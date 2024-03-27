use bevy::window::WindowResolution;
use bevy::{
    input::common_conditions::input_toggle_active,
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
};
use bevy_hanabi::prelude::*;
use bevy_mod_picking::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_tweening::TweeningPlugin;
use clap::Parser;
use model::Settings;

mod building;
mod camera;
mod devel;
mod firing;
mod helpers;
mod model;
mod terrain;
mod ui;

#[derive(Parser, Resource)]
struct Options {
    #[arg(long)]
    seed: Option<u32>,
    #[arg(long, default_value_t = 64)]
    size: u32,
}

impl Options {
    fn seed(&self) -> Option<model::Seed<u32>> {
        self.seed.map(model::Seed::new)
    }

    fn settings(self) -> Settings {
        Settings {
            seed: self.seed().unwrap_or_else(|| model::Seed::system_time()),
            size: UVec2::new(self.size, self.size),
            ..default()
        }
    }
}

fn main() {
    let options = Options::parse();

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
                        present_mode: bevy::window::PresentMode::AutoNoVsync,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(HanabiPlugin)
        .add_plugins(DefaultPickingPlugins)
        .add_plugins(TweeningPlugin)
        .add_plugins(WireframePlugin)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::KeyI)))
        .add_plugins(helpers::HelpersPlugin)
        .add_plugins(AppStatePlugin)
        .add_plugins(camera::CameraPlugin)
        .add_plugins(devel::DeveloperPlugin)
        .add_plugins(building::BuildingPlugin)
        .add_plugins(firing::FiringPlugin)
        .add_plugins(terrain::TerrainPlugin)
        .add_systems(Update, progress_game)
        .add_systems(PostUpdate, bevy::window::close_on_esc)
        .insert_resource(ClearColor(Color::hex("152238").unwrap()))
        .insert_resource(WireframeConfig::default())
        .insert_resource(options.settings())
        .insert_state(model::Phase::default())
        .run();
}

pub struct AppStatePlugin;

impl Plugin for AppStatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(model::AppState::default())
            .insert_state(model::Activity::default())
            .add_systems(Startup, enter_game)
            .add_systems(OnEnter(model::AppState::Menu), enter_game);
    }
}

fn enter_game(
    mut commands: Commands,
    mut app_state: ResMut<NextState<model::AppState>>,
    mut activity: ResMut<NextState<model::Activity>>,
) {
    commands.insert_resource(model::Settings::default());
    app_state.set(model::AppState::Game);
    activity.set(model::Activity::Observing);
    commands.spawn(iyes_perf_ui::PerfUiCompleteBundle::default());
}

fn progress_game(
    phase: Res<State<model::Phase>>,
    mut next_phase: ResMut<NextState<model::Phase>>,
    mut modified: EventReader<building::ConstructionEvent>,
) {
    for event in modified.read() {
        println!("{:?}", event);
        println!("{:?}", phase);
        let before = &phase.get();
        let after = before.next();
        info!("{:?} -> {:?}", before, after);
        next_phase.set(after);
    }
}
