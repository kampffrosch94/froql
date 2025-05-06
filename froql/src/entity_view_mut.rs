#![deny(missing_docs)]
//! contains the `EntityViewMut` type and its methods
//! This module intended for direct use by the library user.
use std::cell::{Ref, RefMut};
use std::fmt::{self, Debug};
use std::ops::Deref;

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

struct DebugHelper {
    ptr: *const u8,
    debug_fn: fn(*const u8, &mut fmt::Formatter<'_>) -> Result<(), fmt::Error>,
}

impl Debug for DebugHelper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (self.debug_fn)(self.ptr, f)
    }
}

impl Debug for EntityViewMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bk = &self.world.bookkeeping;
        let (aid, _row) = bk.entities.get_archetype(self.entity);
        let a = &bk.archetypes[aid.0 as usize];

        let mut builder = f.debug_struct("EntityViewMut");
        builder
            .field("id", &self.entity.id)
            .field("generation", &self.entity.generation);
        for comp_id in &a.components {
            let comp = &bk.components[comp_id.as_index()];
            if let Some(debug_fn) = comp.debug_fn {
                let ptr = bk.get_component(self.entity, *comp_id);
                let helper = DebugHelper { ptr, debug_fn };
                builder.field("component", &helper);
            } else {
                builder.field("component", &comp.name);
            }
        }
        builder.finish()
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
