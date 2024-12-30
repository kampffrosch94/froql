use std::{any::TypeId, mem};

use crate::{
    bookkeeping::Bookkeeping,
    component::{Component, ComponentId, IS_RELATION},
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

    // TODO wrap in refcell
    pub fn register_component<T: 'static>(&mut self) -> ComponentId {
        self.register_component_inner::<T>(0)
    }

    pub fn create(&mut self) -> Entity {
        self.bookkeeping.create()
    }

    // TODO wrap T in Refcell
    pub fn add_component<T: 'static>(&mut self, e: Entity, val: T) {
        let cid = self.register_component::<T>();
        let ptr = self.bookkeeping.add_component(e, cid);
        unsafe {
            let dst = mem::transmute::<*mut u8, *mut T>(ptr);
            std::ptr::write(dst, val);
        }
    }

    // TODO wrap T in Refcell
    #[track_caller]
    pub fn get_component<T: 'static>(&self, e: Entity) -> &T {
        let tid = TypeId::of::<T>();
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        let ptr = self.bookkeeping.get_component(e, cid);
        unsafe {
            let dst = mem::transmute::<*mut u8, *const T>(ptr);
            &*dst
        }
    }

    #[track_caller]
    pub fn has_component<T: 'static>(&self, e: Entity) -> bool {
        let tid = TypeId::of::<T>();
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        self.bookkeeping.has_component(e, cid)
    }

    // TODO wrap T in Refcell
    #[track_caller]
    pub fn remove_component<T: 'static>(&mut self, e: Entity) {
        let tid = TypeId::of::<T>();
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
        self.register_component_inner::<Relation<T>>(IS_RELATION);
    }

    pub fn register_relation_flags<T: 'static>(&mut self, flags: u32) {
        // TODO: error if component is already registered
        self.register_component_inner::<Relation<T>>(flags | IS_RELATION);
    }

    pub fn add_relation<T: 'static>(&mut self, from: Entity, to: Entity) {
        let origin_cid = self.register_component_inner::<Relation<T>>(IS_RELATION);
        self.bookkeeping.add_relation(origin_cid, from, to);
    }

    pub fn has_relation<T: 'static>(&self, from: Entity, to: Entity) -> bool {
        let o_tid = TypeId::of::<Relation<T>>();
        let origin_cid = self.bookkeeping.get_component_id(o_tid).unwrap(); // TODO error msg
        self.bookkeeping.has_relation(origin_cid, from, to)
    }

    pub fn remove_relation<T: 'static>(&mut self, from: Entity, to: Entity) {
        let cid = self.register_component_inner::<Relation<T>>(IS_RELATION);
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
}
