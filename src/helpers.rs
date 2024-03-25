use bevy::prelude::*;

use crate::model::AppState;

#[derive(Debug, Clone, PartialEq, Eq, Hash, States, Default)]
pub enum ExpirationControl {
    #[default]
    Running,
    Paused,
}

pub struct HelpersPlugin;

impl Plugin for HelpersPlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(ExpirationControl::default())
            .add_systems(
                PostUpdate,
                expirations.run_if(in_state(ExpirationControl::Running)),
            )
            .add_systems(OnExit(AppState::Game), destroy_lifetime::<GamePlayLifetime>)
            .add_systems(PostUpdate, expanding);
    }
}

#[derive(Component, Clone)]
pub struct Expandable {}

fn expanding(mut expandables: Query<(&mut Transform, &Expandable)>, timer: Res<Time>) {
    for (mut transform, _expandable) in &mut expandables {
        transform.scale += Vec3::splat(0.3) * timer.delta_seconds()
    }
}

#[derive(Component, Clone)]
pub struct Expires {
    lifetime: f32,
    expiration: Option<f32>,
}

impl Expires {
    pub fn after(lifetime: f32) -> Self {
        Self {
            lifetime,
            expiration: None,
        }
    }
}

fn expirations(
    mut commands: Commands,
    mut expires: Query<(Entity, &mut Expires, Option<&Name>)>,
    timer: Res<Time>,
) {
    for (entity, mut expires, name) in &mut expires {
        match expires.expiration {
            Some(expiration) => {
                if timer.elapsed_seconds() > expiration {
                    debug!("expiring '{:?}'", name);
                    commands.entity(entity).despawn_recursive();
                }
            }
            None => {
                expires.expiration = Some(timer.elapsed_seconds() + expires.lifetime);
            }
        }
    }
}

pub trait Lifetime {}

fn destroy_lifetime<T>(mut commands: Commands, removing: Query<(Entity, &T)>)
where
    T: Lifetime + Component,
{
    for (entity, _) in removing.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

#[derive(Default, Component)]
pub struct GamePlayLifetime;

impl Lifetime for GamePlayLifetime {}
