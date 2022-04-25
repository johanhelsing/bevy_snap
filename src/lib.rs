use std::marker::PhantomData;

use bevy::{prelude::*, reflect::TypeRegistry};

mod commands;
mod reflect_resource;
mod snapshot_id_provider;
mod world_snapshot;

pub use commands::*;
pub use reflect_resource::ReflectResource;
pub use snapshot_id_provider::*;
pub use world_snapshot::*;

pub trait SnapType: 'static + Send + Sync + Default {
    fn add_types(registry: &mut TypeRegistry);
}

#[derive(Default)]
pub struct SnapPlugin<T>
where
    T: SnapType,
{
    t: PhantomData<T>,
}

impl<T: 'static + SnapType> Plugin for SnapPlugin<T> {
    fn build(&self, app: &mut App) {
        app.init_resource::<SnapRegistry<T>>();
        app.init_resource::<SnapshotIdProvider<T>>();
        app.add_event::<SaveEvent<T>>();
    }
}

pub struct SaveEvent<T: SnapType> {
    pub snapshot: WorldSnapshot<T>,
}

impl<T: SnapType> From<WorldSnapshot<T>> for SaveEvent<T> {
    fn from(snapshot: WorldSnapshot<T>) -> Self {
        Self { snapshot }
    }
}

struct SnapRegistry<T: SnapType> {
    type_registry: TypeRegistry,
    t: PhantomData<T>,
}

impl<T: SnapType> Default for SnapRegistry<T> {
    fn default() -> Self {
        let mut type_registry = TypeRegistry::default();
        T::add_types(&mut type_registry);
        Self {
            type_registry,
            t: default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    fn single<'s, T>(app: &'s mut App) -> &'s T
    where
        T: Component,
    {
        app.world.query::<&T>().iter(&app.world).next().unwrap()
    }

    #[derive(Component)]
    struct TestComponent {
        value: i32,
    }

    #[test]
    fn it_works() {
        fn startup(mut commands: Commands) {
            commands.spawn().insert(TestComponent { value: 0 });
        }

        fn increment(mut query: Query<&mut TestComponent>) {
            let mut test_component = query.single_mut();
            test_component.value += 1;
        }

        App::new()
            .add_startup_system(startup)
            .add_system(increment)
            .set_runner(|mut app| {
                app.update();
                assert_eq!(single::<TestComponent>(&mut app).value, 1);
                app.update();
                assert_eq!(single::<TestComponent>(&mut app).value, 2);
            })
            .run();
    }
}
