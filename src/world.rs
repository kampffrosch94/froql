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
    }
}
