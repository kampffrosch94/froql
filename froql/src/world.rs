use std::{
    any::TypeId,
    cell::{Ref, RefCell, RefMut},
};

use crate::{
    bookkeeping::Bookkeeping,
    component::{Component, ComponentId, RELATION},
    entity_store::Entity,
    entity_view_deferred::DeferredOperation,
    entity_view_mut::EntityViewMut,
    relation::Relation,
};

pub struct World {
    pub bookkeeping: Bookkeeping,
    pub(crate) deferred_queue: RefCell<Vec<DeferredOperation>>,
}

impl World {
    pub fn new() -> Self {
        World {
            bookkeeping: Bookkeeping::new(),
            deferred_queue: RefCell::new(Vec::new()),
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
        return cid;
    }

    pub fn register_component<T: 'static>(&mut self) -> ComponentId {
        if size_of::<T>() > 0 {
            self.register_component_inner::<RefCell<T>>(0)
        } else {
            self.register_component_inner::<T>(0)
        }
    }

    // mostly there for use in query
    #[doc(hidden)]
    pub fn get_component_id<T: 'static>(&self) -> ComponentId {
        let tid = if size_of::<T>() > 0 {
            TypeId::of::<RefCell<T>>()
        } else {
            TypeId::of::<T>()
        };
        self.bookkeeping
            .get_component_id(tid)
            // TODO general error msg handler for T
            .unwrap_or_else(|| panic!("ComponentType is not registered."))
    }

    pub fn create(&mut self) -> Entity {
        self.bookkeeping.create()
    }

    pub fn create_mut(&mut self) -> EntityViewMut {
        EntityViewMut {
            id: self.bookkeeping.create(),
            world: self,
        }
    }

    pub fn is_alive(&self, e: Entity) -> bool {
        self.bookkeeping.is_alive(e)
    }

    pub fn add_component<T: 'static>(&mut self, e: Entity, val: T) {
        let cid = self.register_component::<T>();

        if size_of::<T>() > 0 {
            let val = RefCell::new(val);
            let dst = self.bookkeeping.add_component(e, cid) as *mut RefCell<T>;
            unsafe {
                std::ptr::write(dst, val);
            }
        } else {
            self.bookkeeping.add_component_zst(e, cid);
        }
    }

    #[track_caller]
    pub fn get_component<T: 'static>(&self, e: Entity) -> Ref<T> {
        if size_of::<T>() > 0 {
            let tid = TypeId::of::<RefCell<T>>();
            let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
            let ptr = self.bookkeeping.get_component(e, cid) as *const RefCell<T>;
            let cell = unsafe { &*ptr };
            cell.borrow()
        } else {
            // if we don't panic here we can't return a Ref<T> in the other branch
            // with `generic_const_exprs` this could be a compile time error
            panic!("Can't get reference to ZST component.")
        }
    }

    #[track_caller]
    pub fn get_component_mut<T: 'static>(&self, e: Entity) -> RefMut<T> {
        if size_of::<T>() > 0 {
            let tid = TypeId::of::<RefCell<T>>();
            let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
            let ptr = self.bookkeeping.get_component(e, cid) as *const RefCell<T>;
            let cell = unsafe { &*ptr };
            cell.borrow_mut()
        } else {
            // if we don't panic here we can't return a Ref<T> in the other branch
            // with `generic_const_exprs` this could be a compile time error
            panic!("Can't get reference to ZST component.")
        }
    }

    #[track_caller]
    pub fn has_component<T: 'static>(&self, e: Entity) -> bool {
        let tid: TypeId = if size_of::<T>() > 0 {
            TypeId::of::<RefCell<T>>()
        } else {
            TypeId::of::<T>()
        };
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        self.bookkeeping.has_component(e, cid)
    }

    #[track_caller]
    pub fn remove_component<T: 'static>(&mut self, e: Entity) {
        let tid: TypeId = if size_of::<T>() > 0 {
            TypeId::of::<RefCell<T>>()
        } else {
            TypeId::of::<T>()
        };
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        self.bookkeeping.remove_component(e, cid);
    }

    pub fn destroy(&mut self, e: Entity) {
        self.bookkeeping.destroy(e);
    }

    pub fn process(&mut self) {
        let mut tmp = Vec::new();
        let queue = self.deferred_queue.get_mut();
        std::mem::swap(&mut tmp, queue); // too lazy to work around partial borrows here atm
        for command in tmp {
            match command {
                DeferredOperation::DeleteEntity(e) => {
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_and_get() {
        struct Pos(i32, i32);
        struct Name(String);

        let mut world = World::new();
        let e = world.create();
        world.add_component(e, Pos(4, 2));
        world.add_component(e, Name("Player".to_string()));
        let other = world.create();
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
        let e = world.create();
        world.add_component(e, Pos(4, 2));
        world.add_component(e, Name("Player".to_string()));
        assert!(world.has_component::<Pos>(e));
        assert!(world.has_component::<Name>(e));
        let other = world.create();
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
        let e = world.create();
        world.add_component(e, Pos(4, 2));
        world.add_component(e, Name("Player".to_string()));
        let other = world.create();
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
        let e = world.create();
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
        let a = world.create();
        assert!(!world.has_component::<Comp>(a));
        world.add_component(a, Comp {});
        assert!(world.has_component::<Comp>(a));
        world.remove_component::<Comp>(a);
        assert!(!world.has_component::<Comp>(a));
    }
}
