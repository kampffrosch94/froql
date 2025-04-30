use std::{
    cell::{Ref, RefMut},
    ops::Deref,
};

use crate::{entity_store::Entity, world::World};

pub struct EntityViewMut<'a> {
    pub entity: Entity,
    pub world: &'a mut World,
}

impl Deref for EntityViewMut<'_> {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.entity
    }
}

impl EntityViewMut<'_> {
    pub fn get<T: 'static>(&self) -> Ref<'_, T> {
        self.world.get_component::<T>(self.entity)
    }

    pub fn get_mut<T: 'static>(&self) -> RefMut<'_, T> {
        self.world.get_component_mut::<T>(self.entity)
    }

    pub fn add<T: 'static>(self, val: T) -> Self {
        self.world.add_component(self.entity, val);
        self
    }

    pub fn relate_to<T: 'static>(self, to: Entity) -> Self {
        self.world.add_relation::<T>(self.entity, to);
        self
    }

    pub fn relate_from<T: 'static>(self, from: Entity) -> Self {
        self.world.add_relation::<T>(from, self.entity);
        self
    }

    pub fn is_related_to<T: 'static>(&self, to: Entity) -> bool {
        self.world.has_relation::<T>(self.entity, to)
    }

    pub fn is_related_from<T: 'static>(&self, from: Entity) -> bool {
        self.world.has_relation::<T>(from, self.entity)
    }

    pub fn unrelate_to<T: 'static>(self, to: Entity) -> Self {
        self.world.remove_relation::<T>(self.entity, to);
        self
    }

    pub fn unrelate_from<T: 'static>(self, from: Entity) -> Self {
        self.world.remove_relation::<T>(from, self.entity);
        self
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

    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        self.world.remove_component(self.id)
    }
    */
}

#[cfg(test)]
mod test {
    use crate::world::World;

    #[test]
    fn basics() {
        let mut world = World::new();
        let _e = world.create();
    }
}
