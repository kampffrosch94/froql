use std::{any::TypeId, collections::HashMap};

use crate::component::{Component, ComponentId};

#[derive(Default)]
pub struct World {
    pub component_map: HashMap<TypeId, ComponentId>,
    pub components: Vec<Component>,
}
impl World {
    pub fn register_component<T: 'static>(&mut self) {
        let tid = TypeId::of::<T>();
        if self.component_map.contains_key(&tid) {
            return;
        }
        let id = ComponentId(self.components.len() as u32);
        self.components.push(Component::new::<T>(id));
        self.component_map.insert(tid, id);
    }
}
