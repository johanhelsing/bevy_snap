use bevy::{prelude::*, reflect::TypeRegistry};
use bevy_snap::*;

// This library works by defining your own types of snapshots
// that can be saved and restored using bevy commands

#[derive(Default)]
struct MySnap;

// Define types by creating a struct that implements the SnapType trait
impl SnapType for MySnap {
    fn add_types(registry: &mut TypeRegistry) {
        // Register the types you want to be saved and loaded
        registry.write().register::<Transform>();
        registry.write().register::<Player>();

        // Resources are also supported
        registry.write().register::<Steps>();
    }
}

// Components that are to be restored have to implement the Reflect trait
#[derive(Component, Reflect, Default)]
// And be marked as components
#[reflect(Component)]
struct Player;

// Resources also need to implement the Reflect and Component traits trait
#[derive(Component, Reflect, Default)]
// Resources also (at least at the moment) need to be marked as Components, as well as Resources
#[reflect(Component, Resource)]
struct Steps(f32);

// Actual save data is contained in the WorldSnapshot type,
// which is generic over your type of snapshot.
#[derive(Default)]
struct SaveSlot(WorldSnapshot<MySnap>);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<SaveSlot>()
        .init_resource::<Steps>()
        .add_plugins(DefaultPlugins)
        // Add the SnapPlugin with your SnapType
        .add_plugin(SnapPlugin::<MySnap>::default())
        .add_startup_system(startup)
        .add_system(save_keys)
        .add_system(store_snapshot)
        .add_system(player_movement)
        .run();
}

fn startup(mut commands: Commands, mut rip: ResMut<SnapshotIdProvider>) {
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
        // Entities that are to be saved and loaded have to be "tagged", by adding a special component
        .insert(SnapshotId::<MySnap>::new(rip.next_id()))
        .insert(Player);
}

fn save_keys(mut commands: Commands, keys: Res<Input<KeyCode>>, save_slot: ResMut<SaveSlot>) {
    if keys.just_pressed(KeyCode::S) {
        info!("Making snapshot");
        // This triggers saving the world the next time commands are processed.
        // The snapshot is then sent as an event so it can be picked up by other systems.
        commands.save::<MySnap>();
    } else if keys.just_pressed(KeyCode::L) {
        info!("Restoring snapshot");
        commands.load::<MySnap>(save_slot.0.clone())
    }
}

fn store_snapshot(
    mut save_events: EventReader<SaveEvent<MySnap>>,
    mut save_slot: ResMut<SaveSlot>,
) {
    for save_event in save_events.iter() {
        info!("Writing snapshot to save slot resource");

        // Save the snapshot in a resource so we can restore it later
        save_slot.0 = save_event.snapshot.clone();
    }
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
