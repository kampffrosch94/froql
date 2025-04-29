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

pub struct World {
    pub bookkeeping: Bookkeeping,
    pub(crate) deferred_queue: RefCell<DeferredQueue>,
    // TODO move into query or something
    singleton: Entity,
}

/// This is a queue of operations that will be executed during `world.process()`
pub(crate) struct DeferredQueue {
    pub operations: Vec<DeferredOperation>,
    /// Entitis which are not yet created.
    deferred_creations: Vec<Option<Entity>>,
}

impl World {
    pub fn new() -> Self {
        let mut bookkeeping = Bookkeeping::new();
        let singleton = bookkeeping.create();
        World {
            bookkeeping,
            deferred_queue: RefCell::new(DeferredQueue {
                operations: Vec::new(),
                deferred_creations: Vec::new(),
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
    fn reload_component_inner<T: 'static>(&mut self, cid: ComponentId) -> Result<(), ()> {
        let component = &mut self.bookkeeping.components[cid.as_index()];
        component.update_type::<T>()?;
        for aid in component.get_archetypes() {
            let arch = &mut self.bookkeeping.archetypes[aid.as_index()];
            let col = arch.find_column_mut(cid);
            col.change_drop_function(component.drop_fn.clone());
        }
        Ok(())
    }

    pub fn singleton(&self) -> EntityViewDeferred {
        EntityViewDeferred {
            entity: self.singleton,
            world: self,
        }
    }

    pub fn register_component<T: 'static>(&mut self) -> ComponentId {
        self.register_component_inner::<RefCell<T>>(0)
    }

    pub unsafe fn re_register_component<T: 'static>(&mut self) -> Result<(), ()> {
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
        self.reload_component_inner::<RefCell<T>>(cid)
    }

    pub unsafe fn re_register_relation<T: 'static>(&mut self) -> Result<(), ()> {
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
        self.reload_component_inner::<Relation<T>>(cid)
    }

    // mostly there for use in query
    #[doc(hidden)]
    pub fn get_component_id<T: 'static>(&self) -> ComponentId {
        let tid = TypeId::of::<RefCell<T>>();
        self.bookkeeping
            .get_component_id(tid)
            // TODO general error msg handler for T
            .unwrap_or_else(|| panic!("ComponentType is not registered."))
    }

    pub fn create_entity(&mut self) -> Entity {
        self.bookkeeping.create()
    }

    pub fn ensure_alive(&mut self, id: EntityId) -> Entity {
        self.bookkeeping.ensure_alive(id)
    }

    pub fn create(&mut self) -> EntityViewMut {
        EntityViewMut {
            entity: self.bookkeeping.create(),
            world: self,
        }
    }

    pub fn view_mut(&mut self, e: Entity) -> EntityViewMut {
        EntityViewMut {
            entity: e,
            world: self,
        }
    }

    pub fn create_deferred(&self) -> EntityViewDeferred {
        EntityViewDeferred {
            entity: self.bookkeeping.create_deferred(),
            world: self,
        }
    }

    pub fn is_alive(&self, e: Entity) -> bool {
        self.bookkeeping.is_alive(e)
    }

    pub fn add_component<T: 'static>(&mut self, e: Entity, val: T) {
        let cid = self.register_component::<T>();
        let val = RefCell::new(val);
        let dst = self.bookkeeping.add_component(e, cid) as *mut RefCell<T>;
        unsafe {
            std::ptr::write(dst, val);
        }
    }

    pub fn get_component<T: 'static>(&self, e: Entity) -> Ref<T> {
        let tid = TypeId::of::<RefCell<T>>();
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        let ptr = self.bookkeeping.get_component(e, cid) as *const RefCell<T>;
        let cell = unsafe { &*ptr };
        cell.borrow()
    }

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

    pub fn get_component_mut<T: 'static>(&self, e: Entity) -> RefMut<T> {
        let tid = TypeId::of::<RefCell<T>>();
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        let ptr = self.bookkeeping.get_component(e, cid) as *const RefCell<T>;
        let cell = unsafe { &*ptr };
        cell.borrow_mut()
    }

    pub fn has_component<T: 'static>(&self, e: Entity) -> bool {
        let tid = TypeId::of::<RefCell<T>>();
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        self.bookkeeping.has_component(e, cid)
    }

    pub fn remove_component<T: 'static>(&mut self, e: Entity) {
        let tid = TypeId::of::<RefCell<T>>();
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        self.bookkeeping.remove_component(e, cid);
    }

    pub fn destroy(&mut self, e: Entity) {
        self.bookkeeping.destroy(e);
    }

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
    pub fn register_relation<T: 'static>(&mut self) {
        self.register_component_inner::<Relation<T>>(RELATION);
    }

    pub fn register_relation_flags<T: 'static>(&mut self, flags: u32) {
        // TODO: error if component is already registered
        self.register_component_inner::<Relation<T>>(flags | RELATION);
    }

    pub fn add_relation<T: 'static>(&mut self, from: Entity, to: Entity) {
        let origin_cid = self.register_component_inner::<Relation<T>>(RELATION);
        self.bookkeeping.add_relation(origin_cid, from, to);
    }

    pub fn has_relation<T: 'static>(&self, from: Entity, to: Entity) -> bool {
        let o_tid = TypeId::of::<Relation<T>>();
        let origin_cid = self.bookkeeping.get_component_id(o_tid).unwrap(); // TODO error msg
        self.bookkeeping.has_relation(origin_cid, from, to)
    }

    pub fn remove_relation<T: 'static>(&mut self, from: Entity, to: Entity) {
        let cid = self.register_component_inner::<Relation<T>>(RELATION);
        self.bookkeeping.remove_relation(cid, from, to);
    }

    /// Returns all directly related targets
    /// DOES NOT follow transitive relations
    pub fn relation_targets<'a, T: 'static>(
        &'a self,
        from: Entity,
    ) -> impl Iterator<Item = Entity> + use<'a, T> {
        let o_tid = TypeId::of::<Relation<T>>();
        let origin_cid = self.bookkeeping.get_component_id(o_tid).unwrap(); // TODO error msg
        self.bookkeeping
            .relation_partners(origin_cid, from)
            .into_iter()
            .flat_map(|it| it)
    }

    /// Returns all directly related origins
    /// DOES NOT follow transitive relations
    pub fn relation_origins<'a, T: 'static>(
        &'a self,
        to: Entity,
    ) -> impl Iterator<Item = Entity> + use<'a, T> {
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
            .flat_map(|it| it)
    }

    /// Returns all directly related pairs
    /// DOES NOT follow transitive relations
    pub fn relation_pairs<T: 'static>(&self) -> Vec<(Entity, Entity)> {
        let o_tid = TypeId::of::<Relation<T>>();
        self.bookkeeping.relation_pairs(o_tid)
    }
}

impl DeferredQueue {}

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
