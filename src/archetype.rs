use std::cell::RefCell;

use crate::{component::ComponentId, entity_store::EntityId, layout_vec::LayoutVec};

#[derive(Clone, Copy, Debug)]
pub struct ArchetypetId(pub u32);
/// Standin for erased types
pub enum Erased {}
type ErasedPointer = *const RefCell<Erased>;

pub struct Archetype {
    components: Vec<ComponentId>,
    columns: Vec<LayoutVec>,
    entities: Vec<EntityId>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_struct_sizes() {
        assert!(72 >= size_of::<Archetype>()); // Vec has usize inside, smaller on wasm32
    }

    #[test]
    fn insert_and_get() {
        todo!();
    }
}
