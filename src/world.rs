use std::{
    any::TypeId,
    cell::{Ref, RefCell, RefMut},
};

use crate::{
    bookkeeping::Bookkeeping,
    component::{Component, ComponentId, RELATION},
    entity_store::Entity,
    relation::Relation,
};

pub struct World {
    pub bookkeeping: Bookkeeping,
}

impl World {
    pub fn new() -> Self {
        World {
            bookkeeping: Bookkeeping::new(),
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

    pub fn create(&mut self) -> Entity {
        self.bookkeeping.create()
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
            .relation_targets(origin_cid, from)
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
            .relation_targets(target_cid, to)
            .into_iter()
            .flat_map(|it| it)
    }
}

#[cfg(test)]
mod test {
    use crate::component::{CASCADING_DESTRUCT, EXCLUSIVE, SYMMETRIC};

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
    fn relation_simple() {
        enum Rel {}

        let mut world = World::new();
        world.register_relation::<Rel>();
        let a = world.create();
        let b = world.create();
        assert!(!world.has_relation::<Rel>(a, b));
        world.add_relation::<Rel>(a, b);
        assert!(world.has_relation::<Rel>(a, b));
        world.remove_relation::<Rel>(a, b);
        assert!(!world.has_relation::<Rel>(a, b));

        // removing multiple times is no problem
        world.remove_relation::<Rel>(a, b);
        world.remove_relation::<Rel>(a, b);
        world.remove_relation::<Rel>(a, b);
        assert!(!world.has_relation::<Rel>(a, b));
    }

    #[test]
    fn relation_entity_destroy() {
        enum Rel {}

        let mut world = World::new();
        world.register_relation::<Rel>();
        let a = world.create();
        let b = world.create();
        assert!(!world.has_relation::<Rel>(a, b));
        world.add_relation::<Rel>(a, b);
        assert!(world.has_relation::<Rel>(a, b));
        assert_eq!(1, world.relation_origins::<Rel>(b).count());
        world.destroy(a);
        assert_eq!(0, world.relation_origins::<Rel>(b).count());
        assert!(!world.has_relation::<Rel>(a, b));
    }

    #[test]
    fn relation_exlusive() {
        enum Rel {}

        let mut world = World::new();
        world.register_relation_flags::<Rel>(EXCLUSIVE);
        let a = world.create();
        let b = world.create();
        let c = world.create();
        assert!(!world.has_relation::<Rel>(a, b));
        assert!(!world.has_relation::<Rel>(a, c));
        world.add_relation::<Rel>(a, b);
        assert!(world.has_relation::<Rel>(a, b));
        assert!(!world.has_relation::<Rel>(a, c));
        world.add_relation::<Rel>(a, c);
        assert!(world.has_relation::<Rel>(a, c));
        assert!(!world.has_relation::<Rel>(a, b));
    }

    #[test]
    fn relation_asymmetric() {
        enum Rel {}
        let mut world = World::new();
        world.register_relation_flags::<Rel>(0);
        let a = world.create();
        let b = world.create();
        world.add_relation::<Rel>(a, b);
        assert!(world.has_relation::<Rel>(a, b));
        assert!(!world.has_relation::<Rel>(b, a));
    }

    #[test]
    fn relation_symmetric() {
        enum Rel {}

        let mut world = World::new();
        world.register_relation_flags::<Rel>(SYMMETRIC);
        let a = world.create();
        let b = world.create();
        assert!(!world.has_relation::<Rel>(a, b));
        assert!(!world.has_relation::<Rel>(b, a));
        world.add_relation::<Rel>(a, b);
        assert!(world.has_relation::<Rel>(a, b));
        assert!(world.has_relation::<Rel>(b, a));
    }

    #[test]
    fn relation_cascading() {
        enum Rel {}

        let mut world = World::new();
        world.register_relation_flags::<Rel>(CASCADING_DESTRUCT);
        let a = world.create();
        let b = world.create();
        world.add_relation::<Rel>(a, b);

        assert!(world.has_relation::<Rel>(a, b));
        assert!(world.is_alive(a));
        assert!(world.is_alive(b));

        world.destroy(a);
        assert!(!world.is_alive(a));
        assert!(!world.is_alive(b));
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
