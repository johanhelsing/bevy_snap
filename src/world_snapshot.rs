use bevy::{
    prelude::*,
    reflect::{Reflect, TypeRegistry},
    utils::HashMap,
};
use std::{fmt::Debug, marker::PhantomData};

use crate::{reflect_resource::ReflectResource, SnapType};

/// Add this component to all entities you want to be loaded/saved in snapshots.
/// The `id` has to be unique. Consider using the `SnapshotIdProvider` resource.
#[derive(Component)]
pub struct SnapshotId<T: SnapType> {
    id: u32,
    t: PhantomData<T>,
}

impl<T: SnapType> SnapshotId<T> {
    pub fn new(id: u32) -> Self {
        Self { id, t: default() }
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

/// Maps snapshot_ids to entity id+generation. Necessary to track entities over time.
fn snapshot_id_map<T: SnapType>(world: &mut World) -> HashMap<u32, Entity> {
    let mut rid_map = HashMap::default();
    let mut query = world.query::<(Entity, &SnapshotId<T>)>();
    for (entity, snapshot_id) in query.iter(world) {
        assert!(!rid_map.contains_key(&snapshot_id.id));
        rid_map.insert(snapshot_id.id, entity);
    }
    rid_map
}

struct SnapshotEntity {
    pub entity: Entity,
    pub snapshot_id: u32,
    pub components: Vec<Box<dyn Reflect>>,
}

impl Clone for SnapshotEntity {
    fn clone(&self) -> Self {
        let components = self
            .components
            .iter()
            .map(|c| c.clone_value())
            .collect::<Vec<_>>();

        Self {
            entity: self.entity,
            snapshot_id: self.snapshot_id,
            components,
        }
    }
}

impl Default for SnapshotEntity {
    fn default() -> Self {
        Self {
            entity: Entity::from_raw(0),
            ..default()
        }
    }
}

impl Debug for SnapshotEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnapshotEntity")
            .field("id", &self.entity.id())
            .field("generation", &self.entity.generation())
            .field("snapshot_id", &self.snapshot_id)
            .finish()
    }
}

/// Holds registered components of `SnapshotId` tagged entities, as well as registered resources to save and load from/to the real bevy world.
/// The `checksum` is the sum of hash-values from all hashable objects. It is a sum for the checksum to be order insensitive. This of course
/// is not the best checksum to ever exist, but it is a starting point.
#[derive(Default, Debug)]
pub struct WorldSnapshot<T: SnapType> {
    entities: Vec<SnapshotEntity>,
    pub resources: Vec<Box<dyn Reflect>>,
    pub checksum: u64,
    t: PhantomData<T>,
}

impl<T: SnapType> Clone for WorldSnapshot<T> {
    fn clone(&self) -> Self {
        let resources = self
            .resources
            .iter()
            .map(|r| r.clone_value())
            .collect::<Vec<_>>();

        Self {
            entities: self.entities.clone(),
            resources,
            checksum: self.checksum.clone(),
            t: default(),
        }
    }
}

impl<T: SnapType> WorldSnapshot<T> {
    pub fn from_world(world: &World, type_registry: &TypeRegistry) -> Self {
        let mut snapshot = WorldSnapshot::default();
        let type_registry = type_registry.read();

        // create a snapshot entity for every entity tagged with SnapshotId
        for archetype in world.archetypes().iter() {
            let entities_offset = snapshot.entities.len();
            for entity in archetype.entities() {
                if let Some(snapshot_id) = world.get::<SnapshotId<T>>(*entity) {
                    snapshot.entities.push(SnapshotEntity {
                        entity: *entity,
                        snapshot_id: snapshot_id.id,
                        components: Vec::new(),
                    });
                }
            }

            // fill the component vectors of snapshot entities
            for component_id in archetype.components() {
                let reflect_component = world
                    .components()
                    .get_info(component_id)
                    .and_then(|info| type_registry.get(info.type_id().unwrap()))
                    .and_then(|registration| registration.data::<ReflectComponent>());
                if let Some(reflect_component) = reflect_component {
                    for (i, entity) in archetype
                        .entities()
                        .iter()
                        .filter(|&&entity| world.get::<SnapshotId<T>>(entity).is_some())
                        .enumerate()
                    {
                        if let Some(component) = reflect_component.reflect_component(world, *entity)
                        {
                            assert_eq!(*entity, snapshot.entities[entities_offset + i].entity);
                            // add the hash value of that component to the shapshot checksum, if that component supports hashing
                            if let Some(hash) = component.reflect_hash() {
                                snapshot.checksum += hash;
                            }
                            // add the component to the shapshot
                            snapshot.entities[entities_offset + i]
                                .components
                                .push(component.clone_value());
                        }
                    }
                }
            }
        }

        // go through all resources and clone those that are registered
        for component_id in world.archetypes().resource().unique_components().indices() {
            let reflect_component = world
                .components()
                .get_info(component_id)
                .and_then(|info| type_registry.get(info.type_id().unwrap()))
                .and_then(|registration| registration.data::<ReflectResource>());
            if let Some(reflect_resource) = reflect_component {
                if let Some(resource) = reflect_resource.reflect_resource(world) {
                    // add the hash value of that resource to the shapshot checksum, if that resource supports hashing
                    if let Some(hash) = resource.reflect_hash() {
                        snapshot.checksum += hash;
                    }
                    // add the resource to the shapshot
                    snapshot.resources.push(resource.clone_value());
                }
            }
        }

        snapshot
    }

    pub(crate) fn write_to_world(&self, world: &mut World, type_registry: TypeRegistry) {
        let type_registry = type_registry.read();
        let mut rid_map = snapshot_id_map::<T>(world);

        // first, we write all entities
        for snapshot_entity in self.entities.iter() {
            // find the corresponding current entity or create new entity, if it doesn't exist
            let entity = *rid_map
                .entry(snapshot_entity.snapshot_id)
                .or_insert_with(|| {
                    world
                        .spawn()
                        .insert(SnapshotId::<T>::new(snapshot_entity.snapshot_id))
                        .id()
                });

            // for each registered type, check what we need to do
            for registration in type_registry.iter() {
                let type_id = registration.type_id();
                if let Some(reflect_component) = registration.data::<ReflectComponent>() {
                    if world.entity(entity).contains_type_id(type_id) {
                        // the entity in the world has such a component
                        match snapshot_entity
                            .components
                            .iter()
                            .find(|comp| comp.type_name() == registration.name())
                        {
                            // if we have data saved in the snapshot, overwrite the world
                            Some(component) => {
                                reflect_component.apply_component(world, entity, &**component)
                            }
                            // if we don't have any data saved, we need to remove that component from the entity
                            None => reflect_component.remove_component(world, entity),
                        }
                    } else {
                        // the entity in the world has no such component
                        if let Some(component) = snapshot_entity
                            .components
                            .iter()
                            .find(|comp| comp.type_name() == registration.name())
                        {
                            // if we have data saved in the snapshot, add the component to the entity
                            reflect_component.add_component(world, entity, &**component);
                        }
                        // if both the snapshot and the world does not have the registered component, we don't need to to anything
                    }
                } else {
                    error!(
                        "Unrecognized type in snapshot type registry: {:?}. Did you forget to add #[reflect(Component)] to it?",
                        registration.name()
                    );
                }
            }

            // afterwards, remove the pair from the map (leftover entities will need to be despawned)
            rid_map.remove(&snapshot_entity.snapshot_id);
        }

        // despawn entities which have a snapshot id component but where not present in the snapshot
        for (_, v) in rid_map.iter() {
            world.despawn(*v);
        }

        // then, we write all resources
        for registration in type_registry.iter() {
            let reflect_resource = match registration.data::<ReflectResource>() {
                Some(res) => res,
                None => continue, // likely this is a non-resource component, skip it.
            };

            match reflect_resource.reflect_resource(world) {
                // the world has such a resource
                Some(_) => {
                    // check if we have saved such a resource
                    match self
                        .resources
                        .iter()
                        .find(|res| res.type_name() == registration.name())
                    {
                        // if both the world and the snapshot has the resource, apply the values
                        Some(snapshot_res) => {
                            reflect_resource.apply_resource(world, &**snapshot_res);
                        }
                        // if only the world has the resource, but it doesn't exist in the snapshot, remove the resource
                        None => reflect_resource.remove_resource(world),
                    }
                }
                // the world does not have this resource
                None => {
                    // if we have saved that resource, add it
                    if let Some(snapshot_res) = self
                        .resources
                        .iter()
                        .find(|res| res.type_name() == registration.name())
                    {
                        reflect_resource.add_resource(world, &**snapshot_res);
                    }
                    // if both the world and the snapshot does not have this resource, do nothing
                }
            }
        }
    }
}
