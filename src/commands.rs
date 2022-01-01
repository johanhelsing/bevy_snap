use bevy::{app::Events, ecs::system::Command, prelude::*};

use crate::*;

#[derive(Default)]
pub struct SaveCommand<T: SnapType> {
    t: PhantomData<T>,
}

#[derive(Default)]
pub struct LoadCommand<T: SnapType> {
    snapshot: WorldSnapshot,
    t: PhantomData<T>,
}

pub trait SaveCommandExt {
    fn save<T: SnapType>(&mut self);
    fn load<T: SnapType>(&mut self, snapshot: WorldSnapshot);
}

impl SaveCommandExt for Commands<'_, '_> {
    fn save<T: SnapType>(&mut self) {
        self.add(SaveCommand::<T>::default())
    }

    fn load<T: SnapType>(&mut self, snapshot: WorldSnapshot) {
        self.add(LoadCommand::<T> {
            snapshot,
            ..Default::default()
        })
    }
}

impl<T: SnapType> Command for SaveCommand<T> {
    fn write(self, world: &mut World) {
        let registry = world
            .get_resource::<SnapRegistry<T>>()
            .expect("No type registry found, did you forget to initialize the save plugin?");

        let snapshot = WorldSnapshot::from_world(world, &registry.type_registry);
        let mut save_events = world.get_resource_mut::<Events<SaveEvent<T>>>().unwrap();
        save_events.send(snapshot.into());
    }
}

impl<T: SnapType> Command for LoadCommand<T> {
    fn write(self, world: &mut World) {
        let registry = world
            .get_resource::<SnapRegistry<T>>()
            .expect("No type registry found, did you forget to initialize the save plugin?")
            .type_registry
            .clone();

        info!("restoring save {:?}", registry);

        self.snapshot.write_to_world(world, registry);
    }
}
