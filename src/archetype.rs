use std::cell::RefCell;

use crate::{
    component::{Component, ComponentId},
    entity_store::EntityId,
    layout_vec::LayoutVec,
};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ArchetypeId(pub u32);
#[derive(Clone, Copy, Debug)]
pub struct ArchetypeRow(pub u32);
#[derive(Clone, Copy, Debug)]
pub struct ArchetypeColumn(pub u32);

/// Standin for erased types
pub enum Erased {}
pub type ErasedPointer = *const RefCell<Erased>;

// TODO Optimization: use SmallVec instead of Vec
// or maybe boxed slices?
pub struct Archetype {
    pub components: Vec<ComponentId>,
    pub columns: Vec<LayoutVec>,
    pub entities: Vec<EntityId>,
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

    /// moves row between archetypes
    /// the caller has to fix up entity entries
    /// both for the moved entity and for the entity that was moved to fill holes in the
    /// old archetype
    pub fn move_row(old: &mut Self, new: &mut Self, row: ArchetypeRow) {
        debug_assert!(new.components.len().abs_diff(old.components.len()) <= 1);

        let new_bigger = new.components.len() > old.components.len();
        let mut i = 0;
        let mut j = 0;

        while i < old.components.len() && j < new.components.len() {
            if old.components[i] != new.components[j] {
                if new_bigger {
                    j += 1;
                } else {
                    i += 1;
                }
                debug_assert_eq!(old.components[i], new.components[j]);
            }
            let from = &mut old.columns[i];
            let to = &mut new.columns[j];
            unsafe {
                LayoutVec::move_entry(from, to, row.0);
            }
            i += 1;
            j += 1;
        }
        debug_assert!(
            i.abs_diff(j) <= 1,
            "\nOld: {:?}\nNew: {:?}",
            &old.components,
            &new.components
        );

        let e_id = old.entities.swap_remove(row.0 as usize);
        new.entities.push(e_id);
    }

    /// returns true if an entity was swapped to fill the hole
    #[must_use]
    pub fn delete_row(&mut self, row: ArchetypeRow) -> bool {
        self.entities.swap_remove(row.0 as usize);
        for col in &mut self.columns {
            col.remove_swap(row.0);
        }
        row.0 != self.entities.len() as u32
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_struct_sizes() {
        assert!(72 >= size_of::<Archetype>()); // Vec has usize inside, smaller on wasm32
    }
}
