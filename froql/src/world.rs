#![deny(missing_docs)]
//! contains the `World` type and its methods
//! This module intended for direct use by the library user.

use std::{
    any::{TypeId, type_name},
    cell::{Ref, RefCell, RefMut},
    fmt::{self, Debug},
    mem::MaybeUninit,
};

use crate::{
    bookkeeping::{Bookkeeping, EnsureComponentResult},
    component::{Component, ComponentId, RELATION},
    entity_store::{Entity, EntityId},
    entity_view_deferred::{DeferredOperation, EntityViewDeferred},
    entity_view_mut::EntityViewMut,
    relation::Relation,
    util::short_type_name,
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
    fn register_component_inner<T: 'static>(
        &mut self,
        flags: u32,
        short_name: &str,
    ) -> ComponentId {
        let tid = TypeId::of::<T>();
        if let Some(cid) = self.bookkeeping.get_component_id(tid) {
            return cid;
        }
        let mut cid = ComponentId::from_usize(self.bookkeeping.components.len());
        cid = cid.set_flags(flags);
        self.bookkeeping
            .components
            .push(Component::new::<T>(cid, short_name));
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

    /// Registers a debug formatter for ComponentTypes that implement the Debug trait
    pub fn register_debug<T: 'static + Debug>(&mut self) {
        let cid = self.register_component::<T>();
        assert!(
            !cid.is_relation(),
            "Relations don't contain values and therefore can't be formatted with the Debug trait."
        );
        let debug_fn_wrapped = |ptr: *const u8, formatter: &mut fmt::Formatter<'_>| {
            let debug_fn = <T as Debug>::fmt;
            let ptr = ptr as *const RefCell<T>;
            // SAFETY: we know its a component (not a relation) of the right type here
            let val: &RefCell<T> = unsafe { &*ptr };
            debug_fn(&val.borrow(), formatter)
        };
        self.bookkeeping.components[cid.as_index()].debug_fn = Some(debug_fn_wrapped);
    }

    /// Convenience method for getting a Component of the singleton entity.
    ///
    /// The singleton entity is meant to be used for things that only exist once.
    pub fn singleton<T: 'static>(&self) -> Ref<T> {
        self.get_component::<T>(self.singleton)
    }

    /// Convenience method for getting an optional Component of the singleton entity.
    ///
    /// The singleton entity is meant to be used for things that only exist once.
    pub fn singleton_opt<T: 'static>(&self) -> Option<Ref<T>> {
        self.get_component_opt::<T>(self.singleton)
    }

    /// Convenience method for getting a mutable ref to a Component of the singleton entity.
    ///
    /// The singleton entity is meant to be used for things that only exist once.
    pub fn singleton_mut<T: 'static>(&self) -> RefMut<T> {
        self.get_component_mut::<T>(self.singleton)
    }

    /// Convenience method for getting an optional mutable ref to a Component of
    /// the singleton entity.
    ///
    /// The singleton entity is meant to be used for things that only exist once.
    pub fn singleton_mut_opt<T: 'static>(&self) -> Option<RefMut<T>> {
        self.get_component_mut_opt::<T>(self.singleton)
    }

    /// Convenience method for setting a Component of the singleton entity.
    ///
    /// The singleton entity is meant to be used for things that only exist once.
    pub fn singleton_add<T: 'static>(&mut self, val: T) {
        self.add_component(self.singleton, val)
    }

    /// Returns if singleton has the component of type T.
    ///
    /// The singleton entity is meant to be used for things that only exist once.
    pub fn singleton_has<T: 'static>(&self) -> bool {
        self.has_component::<T>(self.singleton)
    }

    /// Convenience method for removing a Component of the singleton entity.
    ///
    /// The singleton entity is meant to be used for things that only exist once.
    pub fn singleton_remove<T: 'static>(&mut self) {
        self.remove_component::<T>(self.singleton)
    }

    /// Registers component type for later use.
    pub fn register_component<T: 'static>(&mut self) -> ComponentId {
        self.register_component_inner::<RefCell<T>>(0, short_type_name::<T>())
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
            .unwrap_or_else(|| panic!("ComponentType '{}' is not registered.", type_name::<T>()))
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

    /// Wraps an existing Entity in an `EntityViewDeferred`.
    pub fn view_deferred(&self, e: Entity) -> EntityViewDeferred {
        EntityViewDeferred {
            entity: e,
            world: self,
        }
    }

    /// Creates an Entity and immediately wraps it in a `EntityViewDeferred`.
    /// Useful when you only have shared reference to `World`.
    ///
    /// The wrapped Entity can be accessed as `.entity` member on the view.
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
    /// Panics if component type is not registered if the feature `manual_registration` is enabled.
    pub fn add_component<T: 'static>(&mut self, e: Entity, mut val: T) {
        let cid = if cfg!(feature = "manual_registration") {
            self.get_component_id::<T>()
        } else {
            self.register_component::<T>()
        };

        match self.bookkeeping.ensure_component(e, cid) {
            EnsureComponentResult::NewComponent(ptr) => {
                let val = RefCell::new(val);
                let dst = ptr as *mut RefCell<T>;
                unsafe {
                    std::ptr::write(dst, val);
                }
            }
            EnsureComponentResult::OldComponent(ptr) => {
                let ptr = ptr as *const RefCell<T>;
                let mut old = unsafe { &*ptr }.borrow_mut();
                // this drops the old component too, how neat
                std::mem::swap::<T>(&mut val, &mut old);
            }
        }
    }

    /// Returns an immutable Ref to the component.
    ///
    /// Panics if `Entity` is not alive or does not have the component.
    /// Panics if component type is not registered.
    pub fn get_component<T: 'static>(&self, e: Entity) -> Ref<T> {
        let cid = self.get_component_id::<T>();
        let ptr = self.bookkeeping.get_component(e, cid) as *const RefCell<T>;
        let cell = unsafe { &*ptr };
        cell.borrow()
    }

    /// Returns an immutable Ref to the component.
    /// Returns `None` if the Entity does not have the component.
    ///
    /// Panics if `Entity` is not alive.
    /// Panics if component type is not registered.
    pub fn get_component_opt<T: 'static>(&self, e: Entity) -> Option<Ref<T>> {
        let cid = self.get_component_id::<T>();
        let Some(ptr) = self.bookkeeping.get_component_opt(e, cid) else {
            return None;
        };
        let ptr = ptr as *const RefCell<T>;
        let cell = unsafe { &*ptr };
        Some(cell.borrow())
    }

    /// Returns a immutable Ref to the component of the Entity with the given `EntityId`.
    ///
    /// Useful if you don't have a generation for whatever reason.
    ///
    /// Panics if `Entity` is not alive or does not have the component.
    pub fn get_component_by_entityid<T: 'static>(&self, id: EntityId) -> Ref<T> {
        let cid = self.get_component_id::<T>();
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
        let cid = self.get_component_id::<T>();
        let ptr = self.bookkeeping.get_component(e, cid) as *const RefCell<T>;
        let cell = unsafe { &*ptr };
        cell.borrow_mut()
    }

    /// Returns a mutable RefMut to the component.
    /// Returns `None` if the Entity does not have the component.
    ///
    /// Panics if `Entity` is not alive.
    /// Panics if component type is not registered.
    pub fn get_component_mut_opt<T: 'static>(&self, e: Entity) -> Option<RefMut<T>> {
        let cid = self.get_component_id::<T>();
        let Some(ptr) = self.bookkeeping.get_component_opt(e, cid) else {
            return None;
        };
        let ptr = ptr as *const RefCell<T>;
        let cell = unsafe { &*ptr };
        Some(cell.borrow_mut())
    }

    /// Returns true, if the Entity has the component.
    ///
    /// Panics if component type is not registered.
    pub fn has_component<T: 'static>(&self, e: Entity) -> bool {
        let cid = self.get_component_id::<T>();
        self.bookkeeping.has_component(e, cid)
    }

    /// Removes component of type `T` from Entity.
    /// This operation is idempotent.
    ///
    /// Panics if component type is not registered if the feature `manual_registration` is enabled.
    pub fn remove_component<T: 'static>(&mut self, e: Entity) {
        let cid = if cfg!(feature = "manual_registration") {
            self.get_component_id::<T>()
        } else {
            self.register_component::<T>()
        };
        self.bookkeeping.remove_component(e, cid, None);
    }

    /// Removes component of type `T` from Entity and returns it if it was present.
    ///
    /// Panics if component type is not registered if the feature `manual_registration` is enabled.
    pub fn take_component<T: 'static>(&mut self, e: Entity) -> Option<T> {
        let cid = if cfg!(feature = "manual_registration") {
            self.get_component_id::<T>()
        } else {
            self.register_component::<T>()
        };

        let mut sink: MaybeUninit<RefCell<T>> = MaybeUninit::uninit();
        if self
            .bookkeeping
            .remove_component(e, cid, Some(sink.as_mut_ptr() as *mut u8))
        {
            let r: RefCell<T> = unsafe { sink.assume_init() };
            Some(r.into_inner())
        } else {
            None
        }
    }

    /// Makes entity not alive.
    /// All components of the entity are dropped (and their drop functions executed).
    pub fn destroy(&mut self, e: Entity) {
        self.bookkeeping.destroy(e);
    }

    /// Defers execution of closure until next World::process()
    /// Useful when borrows get tricky.
    pub fn defer_closure<F>(&self, f: F)
    where
        F: FnOnce(&mut World) + 'static,
    {
        self.deferred_queue
            .borrow_mut()
            .operations
            .push(DeferredOperation::Closure(Box::new(f)));
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
                    self.bookkeeping.remove_component(e, cid, None);
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
                DeferredOperation::Closure(func) => {
                    func(self);
                }
            }
        }
    }
}

// relation stuff in separate impl block
impl World {
    /// shorthand
    fn get_relation_id<T: 'static>(&self) -> ComponentId {
        let tid = TypeId::of::<Relation<T>>();
        self.bookkeeping
            .get_component_id(tid)
            .unwrap_or_else(|| panic!("RelationType '{}' is not registered.", type_name::<T>()))
    }

    /// Registers a relation type.
    ///
    /// It's recommended to use an inhibited type (enum without variants)
    /// so that you don't confuse components and relations on accident.
    pub fn register_relation<T: 'static>(&mut self) -> ComponentId {
        self.register_component_inner::<Relation<T>>(RELATION, short_type_name::<T>())
    }

    /// Registers a relation type with specific flags.
    /// Flag options are: `EXCLUSIVE`, `SYMMETRIC`, `CASCADING_DESTRUCT` and `TRANSITIVE`
    ///
    /// It's recommended to use an inhibited type (enum without variants)
    /// so that you don't confuse components and relations on accident.
    pub fn register_relation_flags<T: 'static>(&mut self, flags: u32) {
        // TODO: error if component is already registered
        self.register_component_inner::<Relation<T>>(flags | RELATION, short_type_name::<T>());
    }

    /// Adds a relationship between two entities.
    /// Registers the relationship type if it is not already.
    pub fn add_relation<T: 'static>(&mut self, from: Entity, to: Entity) {
        let origin_cid = if cfg!(feature = "manual_registration") {
            self.get_relation_id::<T>()
        } else {
            self.register_relation::<T>()
        };

        self.bookkeeping.add_relation(origin_cid, from, to);
    }

    /// Checks if there is a relation between two entities.
    /// Order matters for all relations that are not `SYMMETRIC`.
    pub fn has_relation<T: 'static>(&self, from: Entity, to: Entity) -> bool {
        let origin_cid = self.get_relation_id::<T>();
        self.bookkeeping.has_relation(origin_cid, from, to)
    }

    /// Removes relation between two entities.
    /// This operation is idempotent.
    pub fn remove_relation<T: 'static>(&mut self, from: Entity, to: Entity) {
        let cid = if cfg!(feature = "manual_registration") {
            self.get_relation_id::<T>()
        } else {
            self.register_relation::<T>()
        };

        self.bookkeeping.remove_relation(cid, from, to);
    }

    /// Returns all directly related targets
    /// DOES NOT follow transitive relations
    pub fn relation_targets<T: 'static>(
        &self,
        from: Entity,
    ) -> impl Iterator<Item = Entity> + use<'_, T> {
        let origin_cid = self.get_relation_id::<T>();
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
        let target_cid = self.get_relation_id::<T>().flip_target();
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
        world.register_component::<Pos>();
        world.register_component::<Name>();

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
        world.register_component::<Pos>();
        world.register_component::<Name>();

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
        world.register_component::<Pos>();
        world.register_component::<Name>();

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
        world.register_component::<Pos>();

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

    #[test]
    fn add_component_twice() {
        #[allow(dead_code)]
        struct Comp(i32);
        use std::sync::atomic::{AtomicBool, Ordering as O};
        static WAS_DROPPED: AtomicBool = AtomicBool::new(false);
        impl Drop for Comp {
            fn drop(&mut self) {
                println!("Dropping.");
                let _ = WAS_DROPPED.compare_exchange(false, true, O::Relaxed, O::Relaxed);
            }
        }

        let mut world = World::new();
        world.register_component::<Comp>();
        let e = world.create_entity();
        world.add_component(e, Comp(42));
        world.add_component(e, Comp(50));

        let comp = world.get_component::<Comp>(e);
        assert_eq!(comp.0, 50);
        assert!(WAS_DROPPED.load(O::Relaxed));
    }

    #[cfg(not(feature = "manual_registration"))]
    #[test]
    fn automatic_registration() {
        struct Comp {}
        enum Rel {}
        let mut world = World::new();
        let e = world.create_entity();
        world.create().add(Comp {}).relate_to::<Rel>(e);
    }

    #[cfg(feature = "manual_registration")]
    #[test]
    #[should_panic]
    fn manual_registration() {
        struct Comp {}
        let mut world = World::new();
        world.create().add(Comp {});
    }

    #[cfg(feature = "manual_registration")]
    #[test]
    #[should_panic]
    fn manual_registration_relation() {
        enum Rel {}
        let mut world = World::new();
        let e = world.create_entity();
        world.create().relate_to::<Rel>(e);
    }

    #[test]
    fn relate_to_self() {
        enum Rel {}
        let mut world = World::new();
        let e = world.create_entity();
        world.add_relation::<Rel>(e, e);
        // dbg!(EntityViewDeferred::new(&world, e));
    }
}
