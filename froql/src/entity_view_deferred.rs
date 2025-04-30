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

pub struct EntityViewDeferred<'a> {
    pub entity: Entity,
    pub world: &'a World,
}

pub enum DeferredOperation {
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
        f.debug_struct("EntityViewDeferred")
            .field("generation", &self.entity.generation)
            .field("id", &self.entity.id)
            .finish()
    }
}

impl<'me> EntityViewDeferred<'me> {
    pub fn from_id_unchecked(world: &'me World, id: EntityId) -> Self {
        let entity = world.bookkeeping.entities.get_from_id(id);
        Self { entity, world }
    }

    pub fn get<'a, T: 'static>(&'a self) -> Ref<'me, T> {
        self.world.get_component::<T>(self.entity)
    }

    pub fn get_mut<'a, T: 'static>(&'a self) -> RefMut<'me, T> {
        self.world.get_component_mut::<T>(self.entity)
    }

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

    pub fn relate_to<T: 'static>(&self, to: Entity) -> &Self {
        let tid = TypeId::of::<Relation<T>>();
        self.world
            .deferred_queue
            .borrow_mut()
            .operations
            .push(D::AddRelation(tid, self.entity, to));
        self
    }

    pub fn relate_from<T: 'static>(&self, from: Entity) -> &Self {
        let tid = TypeId::of::<Relation<T>>();
        self.world
            .deferred_queue
            .borrow_mut()
            .operations
            .push(D::AddRelation(tid, from, self.entity));
        self
    }

    pub fn is_related_to<T: 'static>(&self, to: Entity) -> bool {
        self.world.has_relation::<T>(self.entity, to)
    }

    pub fn is_related_from<T: 'static>(&self, from: Entity) -> bool {
        self.world.has_relation::<T>(from, self.entity)
    }

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

    pub fn remove<T: 'static>(&self) -> &Self {
        let tid = TypeId::of::<RefCell<T>>();
        self.world
            .deferred_queue
            .borrow_mut()
            .operations
            .push(D::RemoveComponent(tid, self.entity));
        self
    }

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
