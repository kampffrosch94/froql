use std::{any::TypeId, mem};

use crate::{
    bookkeeping::Bookkeeping,
    component::{Component, ComponentId},
    entity_store::Entity,
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

    pub fn register_component<T: 'static>(&mut self) -> ComponentId {
        let tid = TypeId::of::<T>();
        if let Some(cid) = self.bookkeeping.get_component_id(tid) {
            return cid;
        }
        let cid = ComponentId(self.bookkeeping.components.len() as u32);
        self.bookkeeping.components.push(Component::new::<T>(cid));
        self.bookkeeping.component_map.insert(tid, cid);
        return cid;
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

    // TODO wrap T in Refcell
    #[track_caller]
    pub fn remove_component<T: 'static>(&mut self, e: Entity) {
        let tid = TypeId::of::<T>();
        let cid = self.bookkeeping.get_component_id(tid).unwrap(); // TODO error msg
        self.bookkeeping.remove_component(e, cid);
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
    fn create_delete_get() {
        struct Pos(i32, i32);
        struct Name(String);

        let mut world = World::new();
        let e = world.create();
        world.add_component(e, Pos(4, 2));
        world.add_component(e, Name("Player".to_string()));
        let other = world.create();
        world.add_component(other, Pos(5, 4));
        world.add_component(other, Name("Other".to_string()));

        world.remove_component::<Pos>(e);
        world.remove_component::<Name>(e);

        let pos = world.get_component::<Pos>(other);
        let name = world.get_component::<Name>(other);
        assert_eq!(pos.0, 5);
        assert_eq!(pos.1, 4);
        assert_eq!(name.0, "Other");
    }
}
