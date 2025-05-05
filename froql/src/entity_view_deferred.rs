#![deny(missing_docs)]
//! contains the `EntityViewDeferred` type and its methods
//! This module intended for direct use by the library user.
use std::{
    any::TypeId,
    cell::{Ref, RefCell, RefMut},
    ops::Deref,
};

use std::fmt::Debug;

use crate::{
    entity_store::{Entity, EntityId},
    relation::Relation,
    world::World,
};

/// This is a wrapper of an `Entity` and immutable reference to the `World`.
/// Operations that would mutate the structure of the `World`,
/// namely adding/removing components/relationships or creating/destroying entities
/// on this wrapper are put in queue and only executed once `world.process()` is called.
pub struct EntityViewDeferred<'a> {
    /// The wrapped entity
    pub entity: Entity,
    /// Immutable reference to the World.
    pub world: &'a World,
}

pub(crate) enum DeferredOperation {
    DestroyEntity(Entity),
    /// Boxed, because we have to hide the type somehow
    AddComponent(Box<dyn FnOnce(&mut World)>),
    /// tid, entity
    RemoveComponent(TypeId, Entity),
    /// tid, from, to
    AddRelation(TypeId, Entity, Entity),
    /// tid, from, to
    RemoveRelation(TypeId, Entity, Entity),
}
use DeferredOperation as D;

impl Deref for EntityViewDeferred<'_> {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.entity
    }
}

impl Debug for EntityViewDeferred<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut builder = f.debug_struct("EntityViewDeferred");
        builder
            .field("generation", &self.entity.generation)
            .field("id", &self.entity.id);

        builder.finish()
    }
}

impl<'me> EntityViewDeferred<'me> {
    /// Creates an Entityview from an EntityId
    /// Panics if the Entity with that id is not alive.
    pub fn from_id_unchecked(world: &'me World, id: EntityId) -> Self {
        let entity = world.bookkeeping.entities.get_from_id(id);
        Self { entity, world }
    }

    /// Returns an immutable Ref to the component.
    ///
    /// Panics if `Entity` is not alive or does not have the component.
    /// Panics if component type is not registered.
    pub fn get<'a, T: 'static>(&'a self) -> Ref<'me, T> {
        self.world.get_component::<T>(self.entity)
    }

    /// Returns a mutable RefMut to the component.
    ///
    /// Panics if `Entity` is not alive or does not have the component.
    /// Panics if component type is not registered.
    pub fn get_mut<'a, T: 'static>(&'a self) -> RefMut<'me, T> {
        self.world.get_component_mut::<T>(self.entity)
    }

    /// Adds a component to the entity.
    ///
    /// This method is deferred until `world.process()` is called.
    /// Drops the value if entity is not alive at that point.
    pub fn add<T: 'static>(&self, val: T) -> &Self {
        let e = self.entity;
        self.world
            .deferred_queue
            .borrow_mut()
            .operations
            .push(D::AddComponent(Box::new(move |world| {
                if world.is_alive(e) {
                    world.add_component(e, val);
                }
            })));
        self
    }

    /// Adds a relationship between two entities.
    ///
    /// This method is deferred until `world.process()` is called.
    ///
    /// The wrapped entity is the relationship origin.
    /// Registers the relationship type if it is not already.
    pub fn relate_to<T: 'static>(&self, to: Entity) -> &Self {
        let tid = TypeId::of::<Relation<T>>();
        self.world
            .deferred_queue
            .borrow_mut()
            .operations
            .push(D::AddRelation(tid, self.entity, to));
        self
    }

    /// Adds a relationship between two entities.
    ///
    /// This method is deferred until `world.process()` is called.
    ///
    /// The wrapped entity is the relationship target.
    /// Registers the relationship type if it is not already.
    pub fn relate_from<T: 'static>(&self, from: Entity) -> &Self {
        let tid = TypeId::of::<Relation<T>>();
        self.world
            .deferred_queue
            .borrow_mut()
            .operations
            .push(D::AddRelation(tid, from, self.entity));
        self
    }

    /// Removes a relationship between two entities.
    ///
    /// This method is deferred until `world.process()` is called.
    ///
    /// The wrapped entity is the relationship origin.
    /// Registers the relationship type if it is not already.
    pub fn unrelate_to<T: 'static>(&self, to: Entity) -> &Self {
        let tid = TypeId::of::<Relation<T>>();
        self.world
            .deferred_queue
            .borrow_mut()
            .operations
            .push(D::RemoveRelation(tid, self.entity, to));
        self
    }

    /// Removes a relationship between two entities.
    ///
    /// This method is deferred until `world.process()` is called.
    ///
    /// The wrapped entity is the relationship target.
    /// Registers the relationship type if it is not already.
    pub fn unrelate_from<T: 'static>(&self, from: Entity) -> &Self {
        let tid = TypeId::of::<Relation<T>>();
        self.world
            .deferred_queue
            .borrow_mut()
            .operations
            .push(D::RemoveRelation(tid, from, self.entity));
        self
    }

    /// Checks if there is a relation between two entities.
    /// The wrapped entity is the relationship origin.
    pub fn is_related_to<T: 'static>(&self, to: Entity) -> bool {
        self.world.has_relation::<T>(self.entity, to)
    }

    /// Checks if there is a relation between two entities.
    /// The wrapped entity is the relationship target.
    pub fn is_related_from<T: 'static>(&self, from: Entity) -> bool {
        self.world.has_relation::<T>(from, self.entity)
    }

    /// Returns true, if the Entity has the component.
    ///
    /// Panics if component type is not registered.
    pub fn has<T: 'static>(&self) -> bool {
        self.world.has_component::<T>(self.entity)
    }

    // TODO optional
    /*
    pub fn get_opt<'a, T: 'static>(&'a self) -> Option<Ref<'a, T>> {
        self.world.get_component_opt::<T>(self.id)
    }

    pub fn get_mut_opt<'a, T: 'static>(&'a self) -> Option<RefMut<'a, T>> {
        self.world.get_component_mut_opt::<T>(self.id)
    }
    */

    /// Removes component of type `T` from Entity.
    /// This operation is idempotent.
    ///
    /// This method is deferred until `world.process()` is called.
    ///
    /// Panics if component type is not registered.
    pub fn remove<T: 'static>(&self) -> &Self {
        let tid = TypeId::of::<RefCell<T>>();
        self.world
            .deferred_queue
            .borrow_mut()
            .operations
            .push(D::RemoveComponent(tid, self.entity));
        self
    }

    /// Makes entity not alive.
    /// All components of the entity are dropped (and their drop functions executed).
    ///
    /// This method is deferred until `world.process()` is called.
    pub fn destroy(&self) {
        self.world
            .deferred_queue
            .borrow_mut()
            .operations
            .push(D::DestroyEntity(self.entity));
    }
}

impl From<EntityViewDeferred<'_>> for Entity {
    fn from(val: EntityViewDeferred<'_>) -> Self {
        val.entity
    }
}

impl From<&EntityViewDeferred<'_>> for Entity {
    fn from(val: &EntityViewDeferred<'_>) -> Self {
        val.entity
    }
}

impl PartialEq for EntityViewDeferred<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity && std::ptr::eq(self.world, other.world)
    }
}
