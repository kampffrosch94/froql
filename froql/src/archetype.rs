use std::cell::RefCell;

use crate::{component::ComponentId, entity_store::EntityId, layout_vec::LayoutVec};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ArchetypeId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ArchetypeRow(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ArchetypeColumn(pub u32);

impl ArchetypeId {
    pub fn as_index(&self) -> usize {
        self.0 as usize
    }
}

impl ArchetypeRow {
    pub fn as_index(&self) -> usize {
        self.0 as usize
    }
}

/// Standin for erased types
pub enum Erased {}
pub type ErasedPointer = *const RefCell<Erased>;

pub struct Archetype {
    pub components: Vec<ComponentId>,
    pub columns: Vec<LayoutVec>,
    pub entities: Vec<EntityId>,
}

impl Archetype {
    pub fn new(components: Vec<ComponentId>, columns: Vec<LayoutVec>) -> Self {
        debug_assert!(components.is_sorted());
        debug_assert_eq!(components.len(), columns.len());
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

    /// returns column this component is in
    /// PANICS if column is not in the archetype
    // TODO move looking up the column into a hashmap in bookkeeping
    // similar to ecs_table_record_t in flecs
    // may also lookup differently depending on column count
    pub fn find_column(&self, cid: ComponentId) -> &LayoutVec {
        let index = self.components.iter().position(|it| *it == cid).unwrap();
        &self.columns[index]
    }

    /// returns column this component is in
    /// PANICS if column is not in the archetype
    pub fn find_column_mut(&mut self, cid: ComponentId) -> &mut LayoutVec {
        let index = self.components.iter().position(|it| *it == cid).unwrap();
        &mut self.columns[index]
    }

    /// returns column this component is in
    /// None if not found
    pub fn find_column_opt(&self, cid: ComponentId) -> Option<&LayoutVec> {
        let index = self.components.iter().position(|it| *it == cid);
        index.map(|it| &self.columns[it])
    }

    // we use an outvar so that we don't have to allocate
    // ideally the results can live in an array on the stack
    fn find_multiple_columns_internal(
        &self,
        cids: &[ComponentId],
        result_indexes: &mut [usize],
    ) -> usize {
        debug_assert_eq!(cids.len(), result_indexes.len());
        debug_assert!(cids.is_sorted());
        if cids.is_empty() {
            return 0;
        }
        let mut j = 0;
        for i in 0..self.components.len() {
            if self.components[i] == cids[j] {
                result_indexes[j] = i;
                j += 1;
                if j >= cids.len() {
                    break;
                }
            }
        }
        return j;
    }

    pub fn find_multiple_columns(&self, cids: &[ComponentId], result_indexes: &mut [usize]) {
        let j = self.find_multiple_columns_internal(cids, result_indexes);
        debug_assert_eq!(
            cids.len(),
            j,
            "Internal: did not find as many cids as requested."
        );
    }

    /// returns false on failure
    pub fn find_multiple_columns_fallible(
        &self,
        cids: &[ComponentId],
        result_indexes: &mut [usize],
    ) -> bool {
        let j = self.find_multiple_columns_internal(cids, result_indexes);
        cids.len() == j
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
