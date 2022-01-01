use bevy::{
    prelude::*,
    reflect::{Reflect, TypeRegistry},
    utils::HashMap,
};
use std::fmt::Debug;

use crate::reflect_resource::ReflectResource;

/// Add this component to all entities you want to be loaded/saved on rollback.
/// The `id` has to be unique. Consider using the `RollbackIdProvider` resource.
/// TODO: rename?
#[derive(Component)]
pub struct Rollback {
    id: u32,
}

impl Rollback {
    /// Creates a new rollback tag with the given id.
    pub fn new(id: u32) -> Self {
        Self { id }
    }

    // /// Returns the rollback id.
    //TODO: Is this actually needed ?
    // pub const fn id(&self) -> u32 {
    //     self.id
    // }
}

/// Maps rollback_ids to entity id+generation. Necessary to track entities over time.
fn rollback_id_map(world: &mut World) -> HashMap<u32, Entity> {
    let mut rid_map = HashMap::default();
    let mut query = world.query::<(Entity, &Rollback)>();
    for (entity, rollback) in query.iter(world) {
        assert!(!rid_map.contains_key(&rollback.id));
        rid_map.insert(rollback.id, entity);
    }
    rid_map
}

struct RollbackEntity {
    pub entity: Entity,
    pub rollback_id: u32,
    pub components: Vec<Box<dyn Reflect>>,
}

impl Clone for RollbackEntity {
    fn clone(&self) -> Self {
        let components = self
            .components
            .iter()
            .map(|c| c.clone_value())
            .collect::<Vec<_>>();

        Self {
            entity: self.entity.clone(),
            rollback_id: self.rollback_id.clone(),
            components,
        }
    }
}

impl Default for RollbackEntity {
    fn default() -> Self {
        Self {
            entity: Entity::from_raw(0),
            ..Default::default()
        }
    }
}

impl Debug for RollbackEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RollbackEntity")
            .field("id", &self.entity.id())
            .field("generation", &self.entity.generation())
            .field("rollback_id", &self.rollback_id)
            .finish()
    }
}

/// Holds registered components of `Rollback` tagged entities, as well as registered resources to save and load from/to the real bevy world.
/// The `checksum` is the sum of hash-values from all hashable objects. It is a sum for the checksum to be order insensitive. This of course
/// is not the best checksum to ever exist, but it is a starting point.
#[derive(Default, Debug)]
pub struct WorldSnapshot {
    entities: Vec<RollbackEntity>,
    pub resources: Vec<Box<dyn Reflect>>,
    pub checksum: u64,
}

impl Clone for WorldSnapshot {
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
        }
    }
}

impl WorldSnapshot {
    // TODO: return to crate visibility?
    pub fn from_world(world: &World, type_registry: &TypeRegistry) -> Self {
        let mut snapshot = WorldSnapshot::default();
        let type_registry = type_registry.read();

        // create a rollback entity for every entity tagged with rollback
        for archetype in world.archetypes().iter() {
            let entities_offset = snapshot.entities.len();
            for entity in archetype.entities() {
                if let Some(rollback) = world.get::<Rollback>(*entity) {
                    snapshot.entities.push(RollbackEntity {
                        entity: *entity,
                        rollback_id: rollback.id,
                        components: Vec::new(),
                    });
                }
            }

            // fill the component vectors of rollback entities
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
                        .filter(|&&entity| world.get::<Rollback>(entity).is_some())
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
        let mut rid_map = rollback_id_map(world);

        // first, we write all entities
        for rollback_entity in self.entities.iter() {
            // find the corresponding current entity or create new entity, if it doesn't exist
            let entity = *rid_map
                .entry(rollback_entity.rollback_id)
                .or_insert_with(|| {
                    world
                        .spawn()
                        .insert(Rollback {
                            id: rollback_entity.rollback_id,
                        })
                        .id()
                });

            // for each registered type, check what we need to do
            for registration in type_registry.iter() {
                let type_id = registration.type_id();
                let reflect_component = registration.data::<ReflectComponent>().expect(&format!(
                    "Unregistered type in snapshot type registry: {:?}",
                    registration.name()
                ));

                if world.entity(entity).contains_type_id(type_id) {
                    // the entity in the world has such a component
                    match rollback_entity
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
                    if let Some(component) = rollback_entity
                        .components
                        .iter()
                        .find(|comp| comp.type_name() == registration.name())
                    {
                        // if we have data saved in the snapshot, add the component to the entity
                        reflect_component.add_component(world, entity, &**component);
                    }
                    // if both the snapshot and the world does not have the registered component, we don't need to to anything
                }
            }

            // afterwards, remove the pair from the map (leftover entities will need to be despawned)
            rid_map.remove(&rollback_entity.rollback_id);
        }

        // despawn entities which have a rollback component but where not present in the snapshot
        for (_, v) in rid_map.iter() {
            world.despawn(*v);
        }

        // then, we write all resources
        for registration in type_registry.iter() {
            let reflect_resource = match registration.data::<ReflectResource>() {
                Some(res) => res,
                None => {
                    println!("DIDNT WORK {}", registration.name());
                    continue;
                }
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
