use bevy::prelude::*;
use bevy_snap::*;

#[derive(Default)]
pub struct DummyGamePlugin;

#[derive(Component, Debug, PartialEq, Clone, Copy, Reflect, Default)]
#[reflect(Component, PartialEq)]
pub struct Player;

#[derive(Component, Default, Reflect)]
#[reflect(Component, Resource)]
pub struct Steps(f32);

impl Plugin for DummyGamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::BLACK))
            .init_resource::<Steps>()
            .add_startup_system(startup)
            .add_system(player_movement);
    }
}

fn startup(mut commands: Commands, mut rollback_id_provider: ResMut<RollbackIdProvider>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 1.0),
                custom_size: Some(Vec2::new(50., 50.)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Rollback::new(rollback_id_provider.next_id()))
        .insert(Player);
}

fn player_movement(
    mut query: Query<&mut Transform, With<Player>>,
    mut steps: ResMut<Steps>,
    keys: Res<Input<KeyCode>>,
) {
    let mut direction = Vec2::ZERO;
    if keys.pressed(KeyCode::Right) {
        direction += Vec2::X;
    }
    if keys.pressed(KeyCode::Left) {
        direction -= Vec2::X;
    }
    if keys.pressed(KeyCode::Up) {
        direction += Vec2::Y;
    }
    if keys.pressed(KeyCode::Down) {
        direction -= Vec2::Y;
    }
    if direction != Vec2::ZERO {
        for mut transform in query.iter_mut() {
            transform.translation += direction.extend(0.);
        }
        steps.0 += direction.length();
        info!("Distance traveled: {}", steps.0)
    }
}
