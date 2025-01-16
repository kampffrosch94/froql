use std::{
    cell::{Ref, RefMut},
    ops::Deref,
};

use crate::{
    entity_store::{Entity, EntityId},
    world::World,
};

pub struct EntityViewDeferred<'a> {
    pub id: Entity,
    pub world: &'a World,
}

impl<'a> Deref for EntityViewDeferred<'a> {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.id
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

    pub fn relate_to<T: 'static>(self, _to: Entity) -> Self {
        todo!("Deferred");
        //self.world.add_relation::<T>(self.id, to);
        //self
    }

    pub fn relate_from<T: 'static>(self, _from: Entity) -> Self {
        todo!("Deferred");
        //self.world.add_relation::<T>(from, self.id);
        //self
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
