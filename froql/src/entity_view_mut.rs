#![deny(missing_docs)]
//! contains the `EntityViewMut` type and its methods
//! This module intended for direct use by the library user.
use std::cell::{Ref, RefMut};
use std::fmt::Debug;
use std::ops::Deref;

use crate::debug::debug_view;
use crate::{entity_store::Entity, world::World};

/// This is a convenience wrapper for mutating the components and relationships of an `Entity`.
pub struct EntityViewMut<'a> {
    /// The wrapped entity
    pub entity: Entity,
    /// Mutable reference to the World.
    /// Because mutable access is unique, the `EntityViewMut` needs to be shortlived,
    /// so it does not block other operations.
    pub world: &'a mut World,
}

impl Debug for EntityViewMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        debug_view(f, &self.world, self.entity, "EntityViewMut")
    }
}

impl Deref for EntityViewMut<'_> {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.entity
    }
}

impl EntityViewMut<'_> {
    /// Returns an immutable Ref to the component.
    ///
    /// Panics if `Entity` is not alive or does not have the component.
    /// Panics if component type is not registered.
    pub fn get<T: 'static>(&self) -> Ref<'_, T> {
        self.world.get_component::<T>(self.entity)
    }

    /// Returns a mutable RefMut to the component.
    ///
    /// Panics if `Entity` is not alive or does not have the component.
    /// Panics if component type is not registered.
    pub fn get_mut<T: 'static>(&self) -> RefMut<'_, T> {
        self.world.get_component_mut::<T>(self.entity)
    }

    /// Adds a component to the entity.
    ///
    /// Panics if `Entity` is not alive.
    #[allow(clippy::should_implement_trait)]
    pub fn add<T: 'static>(self, val: T) -> Self {
        self.world.add_component(self.entity, val);
        self
    }

    /// Adds a relationship between two entities.
    /// The wrapped entity is the relationship origin.
    /// Registers the relationship type if it is not already.
    pub fn relate_to<T: 'static>(self, to: Entity) -> Self {
        self.world.add_relation::<T>(self.entity, to);
        self
    }

    /// Adds a relationship between two entities.
    /// The wrapped entity is the relationship target.
    /// Registers the relationship type if it is not already.
    pub fn relate_from<T: 'static>(self, from: Entity) -> Self {
        self.world.add_relation::<T>(from, self.entity);
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

    /// Removes a relationship between two entities.
    /// The wrapped entity is the relationship origin.
    /// Registers the relationship type if it is not already.
    pub fn unrelate_to<T: 'static>(self, to: Entity) -> Self {
        self.world.remove_relation::<T>(self.entity, to);
        self
    }

    /// Removes a relationship between two entities.
    /// The wrapped entity is the relationship target.
    /// Registers the relationship type if it is not already.
    pub fn unrelate_from<T: 'static>(self, from: Entity) -> Self {
        self.world.remove_relation::<T>(from, self.entity);
        self
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
    /// Panics if component type is not registered.
    pub fn remove<T: 'static>(&mut self) {
        // TODO option?
        self.world.remove_component::<T>(self.entity)
    }

    /// Makes entity not alive.
    /// All components of the entity are dropped (and their drop functions executed).
    pub fn destroy(&mut self) {
        self.world.destroy(self.entity);
    }
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
