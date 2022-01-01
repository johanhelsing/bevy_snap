# bevy_snap

Bevy snap is a crate for saving and loading snapshots of entity and resource
state.

Potential usages:

- Save and load game state to disk
- Rewind game mechanic
- Undo in turn-based games
- Rollback in networked games
- Sync game state across networked clients
- Easily reproduce crashes by sending saves as diagnostics

## Usage

`bevy_snap` works by defining your own snapshot type. Register the types you
want to track by implementing the `SnapType` trait:

```rust
use bevy_snap::*;

#[derive(Default)]
struct MySnap;

impl SnapType for MySnap {
    fn add_types(registry: &mut TypeRegistry) {
        // Register the types you want to be saved and loaded
        registry.write().register::<Transform>();
        registry.write().register::<Player>();

        // Resources are also supported
        registry.write().register::<Steps>();
    }
}
```

The components that are to be tracked, need to implement the `Reflect` trait,
and be marked as `Component`s:

```rust
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Player;
```

And so do resources, but they need an additional `Resource` marker as well:

```rust
#[derive(Component, Reflect, Default)]
#[reflect(Component, Resource)]
struct Score(i32);
```

When you start your app, add a `SnapPlugin` with your snap type as type
parameter:

```rust
    .add_plugin(SnapPlugin::<MySnap>::default());
```

Entities that are to be tracked, need to have a unique `SnapshotId` component.
You could do this manually with `SnapshotId::new(some_int)`, but the simplest
way is probably to use the `SnapshotIdProvider` resource that's automatically
added by the plugin:

```rust
fn startup(
    mut commands: Commands, 
    mut snapshot_id_provider: ResMut<SnapshotIdProvider<MySnap>>
) {
    commands.spawn_bundle(PlayerBundle::default())
        .insert(snapshot_id_provider.next());
}
```

Then you can generate snapshot using the `.save()` command:

```rust
fn save_key(mut commands: Commands, keys: Res<Input<KeyCode>>) {
    if keys.just_pressed(KeyCode::S) {
        commands.save::<MySnap>();
    }
}
```

This will generate an event with the snapshot in it.  You can do whatever you
want with it. Save it in a resource, to disk, or add it to a stack if you are
implementing undo.

```rust
fn store_snapshot(
    mut save_events: EventReader<SaveEvent<MySnap>>,
    mut save_slot: ResMut<SaveSlot>,
) {
    for save_event in save_events.iter() {
        save_slot.0 = save_event.snapshot.clone();
    }
}
```

When you want to load the snapshot, it's as simple as invoking the `.load()`
command with the snapshot:

```rust
fn load_key(
    mut commands: Commands,
    mut save_slot: ResMut<SaveSlot>,
    keys: Res<Input<KeyCode>>,
)
    if keys.just_pressed(KeyCode::L) {
        commands.load::<MySnap>(save_slot.0.clone())
    }
}
```

See the [`basic.rs`](./examples/basic.rs) for a complete example very similar to
the above.

## Supported bevy versions

Currently, bevy main is supported, but the plan is to stay with 0.6 once it's
released.