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

    pub fn register_component<T: 'static>(&mut self) {
        let tid = TypeId::of::<T>();
        if self.bookkeeping.component_map.contains_key(&tid) {
            return;
        }
        let id = ComponentId(self.bookkeeping.components.len() as u32);
        self.bookkeeping.components.push(Component::new::<T>(id));
        self.bookkeeping.component_map.insert(tid, id);
    }

    pub fn create(&mut self) -> Entity {
        self.bookkeeping.create()
    }

    // TODO wrap in Refcell
    pub fn add_component<T: 'static>(&mut self, e: Entity, val: T) {
        let tid = TypeId::of::<T>();
        let cid = self.bookkeeping.get_component_id(tid).unwrap();
        let ptr = self.bookkeeping.add_component(e, cid);
        unsafe {
            let dst = mem::transmute::<*mut u8, *mut T>(ptr);
            std::ptr::write(dst, val);
        }
    }
}
