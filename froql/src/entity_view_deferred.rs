use std::{
    any::TypeId,
    cell::{Ref, RefMut},
    ops::Deref,
};

use std::fmt::Debug;

use crate::{
    entity_store::{Entity, EntityId},
    relation::Relation,
    world::World,
};

pub struct EntityViewDeferred<'a> {
    pub id: Entity,
    pub world: &'a World,
}

pub enum DeferredOperation {
    DeleteEntity(Entity),
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

impl<'a> Deref for EntityViewDeferred<'a> {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl<'a> Debug for EntityViewDeferred<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityViewDeferred")
            .field("gen", &self.id.gen)
            .field("id", &self.id.id)
            .finish()
    }
}

impl<'me> EntityViewDeferred<'me> {
    pub fn from_id_unchecked(world: &'me World, id: EntityId) -> Self {
        let entity = world.bookkeeping.entities.get_from_id(id);
        Self { id: entity, world }
    }

    pub fn get<'a, T: 'static>(&'a self) -> Ref<'a, T> {
        self.world.get_component::<T>(self.id)
    }

    pub fn get_mut<'a, T: 'static>(&'a self) -> RefMut<'a, T> {
        self.world.get_component_mut::<T>(self.id)
    }

    pub fn add<T: 'static>(self, _val: T) -> Self {
        todo!("Deferred")
    }

    pub fn relate_to<T: 'static>(self, to: Entity) -> Self {
        let tid = TypeId::of::<Relation<T>>();
        self.world
            .deferred_queue
            .borrow_mut()
            .push(D::AddRelation(tid, self.id, to));
        self
    }

    pub fn relate_from<T: 'static>(self, from: Entity) -> Self {
        let tid = TypeId::of::<Relation<T>>();
        self.world
            .deferred_queue
            .borrow_mut()
            .push(D::AddRelation(tid, from, self.id));
        self
    }

    pub fn is_related_to<T: 'static>(&self, to: Entity) -> bool {
        self.world.has_relation::<T>(self.id, to)
    }

    pub fn is_related_from<T: 'static>(&self, from: Entity) -> bool {
        self.world.has_relation::<T>(from, self.id)
    }

    pub fn has<T: 'static>(&self) -> bool {
        self.world.has_component::<T>(self.id)
    }

    // TODO optional
    /*
    pub fn get_opt<'a, T: 'static>(&'a self) -> Option<Ref<'a, T>> {
        self.world.get_component_opt::<T>(self.id)
    }

    pub fn get_mut_opt<'a, T: 'static>(&'a self) -> Option<RefMut<'a, T>> {
        self.world.get_component_mut_opt::<T>(self.id)
    }

    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        self.world.remove_component(self.id)
    }
    */
}

impl<'a> Into<Entity> for EntityViewDeferred<'a> {
    fn into(self) -> Entity {
        self.id
    }
}
