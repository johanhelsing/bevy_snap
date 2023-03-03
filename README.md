
## deprecation notice

I'not currently working on this project or updating it to new versions of Bevy.

Much of the functionality is now covered by Bevy's built-in scene functionality.

For the things not covered by Bevy, there are other up-to-date community crates, like
[bevy_save](https://github.com/hankjordan/bevy_save).

# bevy_snap

Bevy snap is a crate for saving and loading snapshots of entity and resource
state.

Potential usages:

- Rewind game mechanic
- Undo in turn-based games
- Rollback in networked games

And if serialization is implemented:

- Save and load game state to disk
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

|bevy|bevy_pkv|
|---|---|
|0.7|0.2,main|
|0.6|0.1|

## Thanks!

The core part of this crate is based on code from
[`bevy_ggrs`](https://github.com/gschup/bevy_ggrs).

## License

All code in this repository dual-licensed under either:

- MIT License (LICENSE-MIT or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
