use std::cell::RefCell;

use crate::{
    component::{Component, ComponentId},
    entity_store::EntityId,
    layout_vec::LayoutVec,
};

#[derive(Clone, Copy, Debug)]
pub struct ArchetypeId(pub u32);
#[derive(Clone, Copy, Debug)]
pub struct ArchetypeRow(pub u32);
#[derive(Clone, Copy, Debug)]
pub struct ArchetypeColumn(pub u32);

/// Standin for erased types
pub enum Erased {}
type ErasedPointer = *const RefCell<Erased>;

// TODO Optimization: use SmallVec instead of Vec
// or maybe boxed slices?
pub struct Archetype {
    // TODO not sure if needed
    components: Vec<ComponentId>,
    columns: Vec<LayoutVec>,
    // TODO not sure if needed
    entities: Vec<EntityId>,
}

impl Archetype {
    pub fn new(components: &[&Component]) -> Self {
        debug_assert!(components.is_sorted_by_key(|c| c.id.0));
        let columns = components
            .iter()
            .map(|c| LayoutVec::new(c.layout, c.drop_fn.clone()))
            .collect();
        let components = components.iter().map(|c| c.id).collect();
        Archetype {
            components,
            columns,
            entities: Vec::new(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::any::TypeId;

    use crate::world::World;

    use super::*;

    #[test]
    fn check_struct_sizes() {
        assert!(72 >= size_of::<Archetype>()); // Vec has usize inside, smaller on wasm32
    }

    #[test]
    fn insert_and_get() {
        struct Name(String);
        struct Health(i32);
        let mut world = World::default();
        world.register_component::<Name>();
        world.register_component::<Health>();
        let name_id = world.component_map.get(&TypeId::of::<Name>()).unwrap();
        let health_id = world.component_map.get(&TypeId::of::<Health>()).unwrap();
        let components = [
            &world.components[name_id.0 as usize],
            &world.components[health_id.0 as usize],
        ];
        let archetype = Archetype::new(&components);
    }
}
