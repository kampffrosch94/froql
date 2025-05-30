use std::{any::TypeId, cell::RefCell, collections::HashSet};

use crate::{
    archetype::Archetype, bookkeeping::Bookkeeping, component::ComponentId, entity_store::EntityId,
    layout_vec::LayoutVec, relation_vec::RelationVec, world::World,
};

/// Helps with Relation Traversal
pub struct RelationHelper<'a> {
    cid: ComponentId,
    column: Option<&'a LayoutVec>,
    row: u32,
    rel_index: u32,
    // only used for transitive relations
    transitive_vec: Vec<u32>,
    transitive_set: HashSet<u32>,
}

impl<'a> RelationHelper<'a> {
    pub fn new(cid: ComponentId) -> Self {
        RelationHelper {
            cid,
            // all of the following are overwritten before use
            column: None,
            row: 0,
            rel_index: 0,
            transitive_vec: Vec::new(),
            transitive_set: HashSet::new(),
        }
    }

    pub fn set_col(&mut self, column: &'a LayoutVec) {
        self.column = Some(column);
    }

    pub fn set_row(&mut self, bk: &Bookkeeping, row_counter: u32) {
        self.row = row_counter;
        self.rel_index = u32::MAX; // rolls over to 0

        // compute related eagerly
        if self.cid.is_transitive() {
            self.transitive_set.clear();
            self.transitive_vec.clear();
            let rel_vec = unsafe { &*(self.column.unwrap().get(self.row) as *const RelationVec) };
            let mut work = Vec::new();
            let visited = &mut self.transitive_set;
            work.extend_from_slice(rel_vec);
            visited.extend(work.iter().copied());
            self.transitive_vec.extend_from_slice(rel_vec);

            while !work.is_empty() {
                let current = EntityId(work.pop().unwrap());
                let comp_opt = bk.get_component_opt_unchecked(current, self.cid);
                if let Some(ptr) = comp_opt {
                    let rel_vec = unsafe { &*(ptr as *const RelationVec) };
                    for id in &rel_vec[..] {
                        if !visited.contains(id) {
                            work.push(*id);
                            self.transitive_vec.push(*id);
                            visited.insert(*id);
                        }
                    }
                }
            }
        }
    }

    pub fn next_related(&mut self) -> Option<EntityId> {
        self.rel_index = self.rel_index.wrapping_add(1);
        if self.cid.is_transitive() {
            return self
                .transitive_vec
                .get(self.rel_index as usize)
                .map(|id| EntityId(*id));
        } else {
            let rel_vec = unsafe { &*(self.column.unwrap().get(self.row) as *const RelationVec) };
            if self.rel_index >= rel_vec.len() {
                return None;
            } else {
                let id = rel_vec[self.rel_index as usize];
                return Some(EntityId(id));
            }
        }
    }

    pub fn has_relation(&self, id: EntityId) -> bool {
        if self.cid.is_transitive() {
            self.transitive_set.contains(&id.0)
        } else {
            let rel_vec = unsafe { &*(self.column.unwrap().get(self.row) as *const RelationVec) };
            rel_vec.contains(&id.0)
        }
    }
}

#[repr(transparent)] // same size as RelationHelper
pub struct UnrelationHelper<'a> {
    rel: RelationHelper<'a>,
}

impl<'a> UnrelationHelper<'a> {
    pub fn new(cid: ComponentId) -> Self {
        Self {
            rel: RelationHelper::new(cid),
        }
    }

    pub fn set_col(&mut self, archetype: &'a Archetype) {
        self.rel.column = archetype.find_column_opt(self.rel.cid);
    }

    pub fn set_row(&mut self, bk: &Bookkeeping, row_counter: u32) {
        if self.rel.column.is_some() {
            self.rel.set_row(bk, row_counter);
        }
    }

    /// an unrelation is satisfied if either the RelationComponent does not exist in the or
    /// it does not contain the other entity
    pub fn satisfied(&self, other: EntityId) -> bool {
        if self.rel.column.is_none() {
            return true;
        }
        return !self.rel.has_relation(other);
    }
}

/// This function exists as a helper for user macros that care about compile time
/// You need to wrap the Type you care about in RefCell<>, since all components are RefCells
pub fn trivial_query_one_component(world: &World, ty: TypeId) -> Vec<EntityId> {
    let bk = &world.bookkeeping;
    let cid = bk
        .component_map
        .get(&ty)
        .expect("Type is not registered as component.");
    let c = &bk.components[cid.as_index()];
    let archetypes = c.get_archetypes();
    archetypes
        .flat_map(|aid| bk.archetypes[aid.as_index()].entities.iter())
        .copied()
        .collect()
}

/// This function is used inside the proc macro to cast outputs.
/// We need an extra function to coerce them to the correct lifetime.
///
/// # SAFETY
/// The same rules as casting a raw pointer apply.
/// The target lifetime must be correct, otherwise there will be soundness issues.
pub unsafe fn coerce_cast<'input: 'output, 'output, T>(
    _world: &'input World, // just used to grab a lifetime
    ptr: *mut u8,
) -> &'output RefCell<T> {
    unsafe { &*(ptr as *const RefCell<T>) }
}
