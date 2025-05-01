#![deny(missing_docs)]
//! contains the `World` type and its methods
//! This module intended for direct use by the library user.

use std::{
    any::{TypeId, type_name},
    cell::{Ref, RefCell, RefMut},
};

use crate::{
    bookkeeping::Bookkeeping,
    component::{Component, ComponentId, RELATION},
    entity_store::{Entity, EntityId},
    entity_view_deferred::{DeferredOperation, EntityViewDeferred},
    entity_view_mut::EntityViewMut,
    relation::Relation,
};

/// The `World` is the central datastructure in froql that holds all state.
pub struct World {
    /// internal state management
    /// Bookkeeping is public because some queries need to interact with it.
    #[doc(hidden)]
    pub bookkeeping: Bookkeeping,
    pub(crate) deferred_queue: RefCell<DeferredQueue>,
    // TODO move into query or something
    singleton: Entity,
}

/// This is a queue of operations that will be executed during `world.process()`
pub(crate) struct DeferredQueue {
    pub operations: Vec<DeferredOperation>,
}

impl World {
    /// Creates a new `World`
    ///
    /// Don't forget to register your Components and Relationships
    /// with `register_component::<T>()` and `register_relation::<T>()`.
    pub fn new() -> Self {
        let mut bookkeeping = Bookkeeping::new();
        let singleton = bookkeeping.create();
        World {
            bookkeeping,
            deferred_queue: RefCell::new(DeferredQueue {
                operations: Vec::new(),
            }),
            singleton,
        }
    }

    /// Used internally to register both components and relations
    /// because Relations are a special kind of component
    /// and Components are meant to be wrapped in `RefCell`
    fn register_component_inner<T: 'static>(&mut self, flags: u32) -> ComponentId {
        let tid = TypeId::of::<T>();
        if let Some(cid) = self.bookkeeping.get_component_id(tid) {
            return cid;
        }
        let mut cid = ComponentId::from_usize(self.bookkeeping.components.len());
        cid = cid.set_flags(flags);
        self.bookkeeping.components.push(Component::new::<T>(cid));
        self.bookkeeping.component_map.insert(tid, cid);
        let tname = type_name::<T>().to_string();
        let old = self.bookkeeping.component_name_map.insert(tname, tid);
        assert_eq!(None, old, "Typename was already registered.");
        return cid;
    }

    /// Counterpart of register_component_inner for hotreloading purposes
    unsafe fn reload_component_inner<T: 'static>(
        &mut self,
        cid: ComponentId,
    ) -> Result<(), ReregisterError> {
        let component = &mut self.bookkeeping.components[cid.as_index()];
        unsafe {
            component.update_type::<T>()?;
        }
        for aid in component.get_archetypes() {
            let arch = &mut self.bookkeeping.archetypes[aid.as_index()];
            let col = arch.find_column_mut(cid);
            unsafe {
                col.change_drop_function(component.drop_fn.clone());
            }
        }
        Ok(())
    }

    /// Convenience method for getting an EntityViewDeferred of the singleton entity.
    ///
    /// The singleton entity is meant to be used for things that only exist once.
    /// Don't delete it. It is not particularly special otherwise.
    pub fn singleton(&self) -> EntityViewDeferred {
        EntityViewDeferred {
            entity: self.singleton,
            world: self,
        }
    }

    /// Registers component type for later use.
    pub fn register_component<T: 'static>(&mut self) -> ComponentId {
        self.register_component_inner::<RefCell<T>>(0)
    }

    /// This allows reusing the same world in hotreloading scenarios.
    /// This is not only unsafe, its straight up undefined behavior.
    /// Very useful for development purposes though.
    ///
    /// DO NOT ship code that calls this method in production.
    /// Use feature flags to accomplish this.
    ///
    /// # SAFETY
    /// This finds the type by using its typename.
    /// If you have two components/relations with the exact same name you are in trouble.
    ///
    /// If a types layout changed you get an error. But not all changes to a struct result
    /// in a change to its layout.
    pub unsafe fn re_register_component<T: 'static>(&mut self) -> Result<(), ReregisterError> {
        let tid = TypeId::of::<RefCell<T>>();
        let name = type_name::<RefCell<T>>();
        let old_tid = self
            .bookkeeping
            .component_name_map
            .get(name)
            .expect("Type {name} was not registered as component.");
        let cid = self.bookkeeping.component_map.remove(old_tid).unwrap();
        self.bookkeeping.component_map.insert(tid, cid);
        self.bookkeeping
            .component_name_map
            .insert(name.to_string(), tid);
        unsafe { self.reload_component_inner::<RefCell<T>>(cid) }
    }

    /// This allows reusing the same world in hotreloading scenarios.
    /// This is not only unsafe, its straight up undefined behavior.
    /// Very useful for development purposes though.
    ///
    /// DO NOT ship code that calls this method in production.
    /// Use feature flags to accomplish this.
    ///
    /// # SAFETY
    /// This finds the type by using its typename.
    /// If you have two components/relations with the exact same name you are in trouble.
    ///
    /// If a types layout changed you get an error. But not all changes to a struct result
    /// in a change to its layout.
    pub unsafe fn re_register_relation<T: 'static>(&mut self) -> Result<(), ReregisterError> {
        let tid = TypeId::of::<Relation<T>>();
        let name = type_name::<Relation<T>>();
        let old_tid = self
            .bookkeeping
            .component_name_map
            .get(name)
            .expect("Type {name} was not registered as component.");
        let cid = self.bookkeeping.component_map.remove(old_tid).unwrap();
        self.bookkeeping.component_map.insert(tid, cid);
        self.bookkeeping
            .component_name_map
            .insert(name.to_string(), tid);
        unsafe { self.reload_component_inner::<Relation<T>>(cid) }
    }

    /// mostly there for use in query
    #[doc(hidden)]
    pub fn get_component_id<T: 'static>(&self) -> ComponentId {
        let tid = TypeId::of::<RefCell<T>>();
        self.bookkeeping
            .get_component_id(tid)
            // TODO general error msg handler for T
            .unwrap_or_else(|| panic!("ComponentType is not registered."))
    }

    /// Creates an Entity and returns it.
    ///
    /// This Entity is not wrapped in a view, so it doesn't carry a lifetime.
    pub fn create_entity(&mut self) -> Entity {
        self.bookkeeping.create()
    }

    /// Turns an `EntityId` into an `Entity`.
    /// If the `Entity` was not alive before it will be made alive.
    ///
    /// This is useful when building deserialization.
    pub fn ensure_alive(&mut self, id: EntityId) -> Entity {
        self.bookkeeping.ensure_alive(id)
    }

    /// Creates an Entity and immediately wraps it in a `EntityViewMut`.
    /// Useful for convenience.
    ///
    /// The wrapped Entity can be accessed as `.entity` member on the view.
    pub fn create(&mut self) -> EntityViewMut {
        EntityViewMut {
            entity: self.bookkeeping.create(),
            world: self,
        }
    }

    /// Wraps an existing Entity in an `EntityViewMut`.
    pub fn view_mut(&mut self, e: Entity) -> EntityViewMut {
        EntityViewMut {
            entity: e,
            world: self,
        }
    }

    /// Creates an Entity and immediately wraps it in a `EntityViewDeferred`.
    /// Useful when you only have shared reference to `World`.
    ///
    /// The wrapped Entity can be accessed as `.entity` member on the view.
    ///
    /// Don't
    pub fn create_deferred(&self) -> EntityViewDeferred {
        EntityViewDeferred {
            entity: self.bookkeeping.create_deferred(),
            world: self,
        }
    }

    /// Checks if the `Entity` is alive by using its generation.
    pub fn is_alive(&self, e: Entity) -> bool {
        self.bookkeeping.is_alive(e)
    }

    /// Adds a component to the entity.
    ///
    /// Panics if `Entity` is not alive.
    pub fn add_component<T: 'static>(&mut self, e: Entity, val: T) {
        let cid = self.register_component::<T>();
        let val = RefCell::new(val);
        let dst = self.bookkeeping.add_component(e, cid) as *mut RefCell<T>;
        unsafe {
            std::ptr::write(dst, val);
        }
    }

    /// Returns an immutable Ref to the component.
    ///
    /// Panics if `Entity` is not alive or does not have the component.
    /// Panics if component type is not registered.
    pub fn get_component<T: 'static>(&self, e: Entity) -> Ref<T> {
        let tid = TypeId::of::<RefCell<T>>();
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        let ptr = self.bookkeeping.get_component(e, cid) as *const RefCell<T>;
        let cell = unsafe { &*ptr };
        cell.borrow()
    }

    /// Returns a immutable Ref to the component of the Entity with the given `EntityId`.
    ///
    /// Useful if you don't have a generation for whatever reason.
    ///
    /// Panics if `Entity` is not alive or does not have the component.
    pub fn get_component_by_entityid<T: 'static>(&self, id: EntityId) -> Ref<T> {
        let tid = TypeId::of::<RefCell<T>>();
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        let ptr = self
            .bookkeeping
            .get_component_opt_unchecked(id, cid)
            .unwrap() as *const RefCell<T>;
        let cell = unsafe { &*ptr };
        cell.borrow()
    }

    /// Returns a mutable RefMut to the component.
    ///
    /// Panics if `Entity` is not alive or does not have the component.
    /// Panics if component type is not registered.
    pub fn get_component_mut<T: 'static>(&self, e: Entity) -> RefMut<T> {
        let tid = TypeId::of::<RefCell<T>>();
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        let ptr = self.bookkeeping.get_component(e, cid) as *const RefCell<T>;
        let cell = unsafe { &*ptr };
        cell.borrow_mut()
    }

    /// Returns true, if the Entity has the component.
    ///
    /// Panics if component type is not registered.
    pub fn has_component<T: 'static>(&self, e: Entity) -> bool {
        let tid = TypeId::of::<RefCell<T>>();
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        self.bookkeeping.has_component(e, cid)
    }

    /// Removes component type from Entity.
    /// This operation is idempotent.
    ///
    /// Panics if component type is not registered.
    pub fn remove_component<T: 'static>(&mut self, e: Entity) {
        let tid = TypeId::of::<RefCell<T>>();
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        self.bookkeeping.remove_component(e, cid);
    }

    /// Makes entity not alive.
    /// All components of the entity are dropped (and their drop functions executed).
    pub fn destroy(&mut self, e: Entity) {
        self.bookkeeping.destroy(e);
    }

    /// Executes all queued deferred operations.
    pub fn process(&mut self) {
        self.bookkeeping.realize_deferred();

        let mut tmp = Vec::new();
        let queue = self.deferred_queue.get_mut();
        let ops = &mut queue.operations;
        std::mem::swap(&mut tmp, ops); // too lazy to work around partial borrows here atm
        for command in tmp {
            match command {
                DeferredOperation::DestroyEntity(e) => {
                    self.destroy(e);
                }
                DeferredOperation::AddComponent(func) => {
                    func(self);
                }
                DeferredOperation::RemoveComponent(tid, e) => {
                    let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
                    self.bookkeeping.remove_component(e, cid);
                }
                DeferredOperation::AddRelation(tid, from, to) => {
                    let Some(cid) = self.bookkeeping.get_component_id(tid) else {
                        panic!("Can't register relation in deferred context.");
                    };
                    self.bookkeeping.add_relation(cid, from, to);
                }
                DeferredOperation::RemoveRelation(tid, from, to) => {
                    let Some(cid) = self.bookkeeping.get_component_id(tid) else {
                        panic!("Can't register relation in deferred context.");
                    };
                    self.bookkeeping.remove_relation(cid, from, to);
                }
            }
        }
    }
}

// relation stuff in separate impl block
impl World {
    /// Registers a relation type.
    ///
    /// It's recommended to use an inhibited type (enum without variants)
    /// so that you don't confuse components and relations on accident.
    pub fn register_relation<T: 'static>(&mut self) {
        self.register_component_inner::<Relation<T>>(RELATION);
    }

    /// Registers a relation type with specific flags.
    /// Flag options are: `EXCLUSIVE`, `SYMMETRIC`, `CASCADING_DESTRUCT` and `TRANSITIVE`
    ///
    /// It's recommended to use an inhibited type (enum without variants)
    /// so that you don't confuse components and relations on accident.
    pub fn register_relation_flags<T: 'static>(&mut self, flags: u32) {
        // TODO: error if component is already registered
        self.register_component_inner::<Relation<T>>(flags | RELATION);
    }

    /// Adds a relationship between two entities.
    /// Registers the relationship type if it is not already.
    pub fn add_relation<T: 'static>(&mut self, from: Entity, to: Entity) {
        let origin_cid = self.register_component_inner::<Relation<T>>(RELATION);
        self.bookkeeping.add_relation(origin_cid, from, to);
    }

    /// Checks if there is a relation between two entities.
    /// Order matters for all relations that are not `SYMMETRIC`.
    pub fn has_relation<T: 'static>(&self, from: Entity, to: Entity) -> bool {
        let o_tid = TypeId::of::<Relation<T>>();
        let origin_cid = self.bookkeeping.get_component_id(o_tid).unwrap(); // TODO error msg
        self.bookkeeping.has_relation(origin_cid, from, to)
    }

    /// Removes relation between two entities.
    /// This operation is idempotent.
    pub fn remove_relation<T: 'static>(&mut self, from: Entity, to: Entity) {
        let cid = self.register_component_inner::<Relation<T>>(RELATION);
        self.bookkeeping.remove_relation(cid, from, to);
    }

    /// Returns all directly related targets
    /// DOES NOT follow transitive relations
    pub fn relation_targets<T: 'static>(
        &self,
        from: Entity,
    ) -> impl Iterator<Item = Entity> + use<'_, T> {
        let o_tid = TypeId::of::<Relation<T>>();
        let origin_cid = self.bookkeeping.get_component_id(o_tid).unwrap(); // TODO error msg
        self.bookkeeping
            .relation_partners(origin_cid, from)
            .into_iter()
            .flatten()
    }

    /// Returns all directly related origins
    /// DOES NOT follow transitive relations
    pub fn relation_origins<T: 'static>(
        &self,
        to: Entity,
    ) -> impl Iterator<Item = Entity> + use<'_, T> {
        let tid = TypeId::of::<Relation<T>>();
        let target_cid = self
            .bookkeeping
            .get_component_id(tid)
            .unwrap() // TODO error msg
            .flip_target();
        self.bookkeeping
            // same logic as with target, just different parameter
            .relation_partners(target_cid, to)
            .into_iter()
            .flatten()
    }

    /// Returns all directly related pairs
    /// DOES NOT follow transitive relations
    pub fn relation_pairs<T: 'static>(&self) -> Vec<(Entity, Entity)> {
        let o_tid = TypeId::of::<Relation<T>>();
        self.bookkeeping.relation_pairs(o_tid)
    }
}

/// Error Type for `reregister_component`.
pub enum ReregisterError {
    /// The new type has a different layout than the old type.
    DifferingLayout,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_and_get() {
        struct Pos(i32, i32);
        struct Name(String);

        let mut world = World::new();
        let e = world.create_entity();
        world.add_component(e, Pos(4, 2));
        world.add_component(e, Name("Player".to_string()));
        let other = world.create_entity();
        world.add_component(other, Pos(5, 4));
        world.add_component(other, Name("Other".to_string()));

        let pos = world.get_component::<Pos>(e);
        let name = world.get_component::<Name>(e);
        assert_eq!(pos.0, 4);
        assert_eq!(pos.1, 2);
        assert_eq!(name.0, "Player");
        let pos = world.get_component::<Pos>(other);
        let name = world.get_component::<Name>(other);
        assert_eq!(pos.0, 5);
        assert_eq!(pos.1, 4);
        assert_eq!(name.0, "Other");
    }

    #[test]
    fn create_remove_get() {
        struct Pos(i32, i32);
        struct Name(String);

        let mut world = World::new();
        let e = world.create_entity();
        world.add_component(e, Pos(4, 2));
        world.add_component(e, Name("Player".to_string()));
        assert!(world.has_component::<Pos>(e));
        assert!(world.has_component::<Name>(e));
        let other = world.create_entity();
        world.add_component(other, Pos(5, 4));
        world.add_component(other, Name("Other".to_string()));

        world.remove_component::<Pos>(e);
        world.remove_component::<Name>(e);
        assert!(!world.has_component::<Pos>(e));
        assert!(!world.has_component::<Name>(e));

        let pos = world.get_component::<Pos>(other);
        let name = world.get_component::<Name>(other);
        assert_eq!(pos.0, 5);
        assert_eq!(pos.1, 4);
        assert_eq!(name.0, "Other");
    }

    #[test]
    fn create_destroy_get() {
        struct Pos(i32, i32);
        struct Name(String);

        let mut world = World::new();
        let e = world.create_entity();
        world.add_component(e, Pos(4, 2));
        world.add_component(e, Name("Player".to_string()));
        let other = world.create_entity();
        world.add_component(other, Pos(5, 4));
        world.add_component(other, Name("Other".to_string()));

        world.destroy(e);

        let pos = world.get_component::<Pos>(other);
        let name = world.get_component::<Name>(other);
        assert_eq!(pos.0, 5);
        assert_eq!(pos.1, 4);
        assert_eq!(name.0, "Other");
    }

    #[test]
    fn component_mut() {
        struct Pos(i32, i32);

        let mut world = World::new();
        let e = world.create_entity();
        world.add_component(e, Pos(4, 2));
        let pos = world.get_component::<Pos>(e);
        assert_eq!(pos.0, 4);
        assert_eq!(pos.1, 2);
        drop(pos); // need to release ref

        let mut pos = world.get_component_mut::<Pos>(e);
        pos.0 = 20;
        pos.1 = 30;
        drop(pos); // need to release refmut

        let pos = world.get_component::<Pos>(e);
        assert_eq!(pos.0, 20);
        assert_eq!(pos.1, 30);
    }

    #[test]
    fn zst_component() {
        struct Comp {}

        let mut world = World::new();
        world.register_component::<Comp>();
        let a = world.create_entity();
        assert!(!world.has_component::<Comp>(a));
        world.add_component(a, Comp {});
        assert!(world.has_component::<Comp>(a));

        {
            let _comp: Ref<Comp> = world.get_component::<Comp>(a);
        }

        world.remove_component::<Comp>(a);
        assert!(!world.has_component::<Comp>(a));
    }
}
