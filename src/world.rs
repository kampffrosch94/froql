use std::{any::TypeId, collections::HashMap};

use crate::component::{Component, ComponentId};

#[derive(Default)]
pub struct World {
    component_map: HashMap<TypeId, ComponentId>,
    components: Vec<Component>,
}
impl World {
    pub fn register_component<T: 'static>(&mut self) {
        let tid = TypeId::of::<T>();
        if self.component_map.contains_key(&tid) {
            return;
        }
        let id = ComponentId(self.components.len() as u32);
        self.components.push(Component::new::<T>());
        self.component_map.insert(tid, id);
    }
}
