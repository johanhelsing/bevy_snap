use bevy::{prelude::*, reflect::TypeRegistry};
use bevy_snap::*;

#[path = "../dummy_game/dummy_game.rs"]
mod dummy_game;
use dummy_game::*;

#[derive(Default)]
struct MySnapType;

impl SnapType for MySnapType {
    fn add_types(registry: &mut TypeRegistry) {
        registry.write().register::<Transform>();
        registry.write().register::<dummy_game::Player>();
        registry.write().register::<dummy_game::Steps>();
    }
}

#[derive(Default)]
struct SaveSlot(WorldSnapshot<MySnapType>);

fn main() {
    App::new()
        .init_resource::<SaveSlot>()
        .add_plugins(DefaultPlugins)
        .add_plugin(SnapPlugin::<MySnapType>::default())
        .add_plugin(dummy_game::DummyGamePlugin)
        .add_system(save_keys)
        .add_system(tag_player)
        .add_system(store_snapshot)
        .run();
}

fn tag_player(
    mut commands: Commands,
    query: Query<Entity, (With<Player>, Without<Rollback<MySnapType>>)>,
    mut rollback_id_provider: ResMut<RollbackIdProvider>,
) {
    for entity in query.iter() {
        commands
            .entity(entity)
            .insert(Rollback::<MySnapType>::new(rollback_id_provider.next_id()));
    }
}

fn save_keys(mut commands: Commands, keys: Res<Input<KeyCode>>, save_slot: ResMut<SaveSlot>) {
    if keys.just_pressed(KeyCode::S) {
        info!("Making snapshot");
        commands.save::<MySnapType>();
    } else if keys.just_pressed(KeyCode::L) {
        info!("Restoring snapshot");
        commands.load::<MySnapType>(save_slot.0.clone())
    }
}

fn store_snapshot(
    mut save_events: EventReader<SaveEvent<MySnapType>>,
    mut save_slot: ResMut<SaveSlot>,
) {
    for save_event in save_events.iter() {
        info!("Writing snapshot to save slot resource");
        save_slot.0 = save_event.snapshot.clone();
    }
}
