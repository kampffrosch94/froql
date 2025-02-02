use crate::{
    bookkeeping::Bookkeeping, component::ComponentId, entity_store::EntityId,
    layout_vec::LayoutVec, relation_vec::RelationVec,
};

/// Helps with Relation Traversal
pub struct RelationHelper<'a> {
    cid: ComponentId,
    column: Option<&'a LayoutVec>,
    row: u32,
    rel_index: u32,
}

impl<'a> RelationHelper<'a> {
    pub fn new(cid: ComponentId) -> Self {
        RelationHelper {
            cid,
            // all of the following are overwritten before use
            column: None,
            row: 0,
            rel_index: 0,
        }
    }

    pub fn set_col(&mut self, column: &'a LayoutVec) {
        self.column = Some(column);
    }

    pub fn set_row(&mut self, bk: &Bookkeeping, row_counter: u32) {
        self.row = row_counter;
        self.rel_index = u32::MAX; // rolls over to 0
        if self.cid.is_transitive() {
            todo!("Transitive needs to compute eagerly here.");
        }
    }

    pub fn next_related(&mut self) -> Option<EntityId> {
        self.rel_index = self.rel_index.wrapping_add(1);
        let rel_vec = unsafe { &*(self.column.unwrap().get(self.row) as *const RelationVec) };
        if self.rel_index >= rel_vec.len() {
            return None;
        } else {
            let id = rel_vec[self.rel_index as usize];
            return Some(EntityId(id));
        }
    }

    pub fn has_relation(&self, id: EntityId) -> bool {
        let rel_vec = unsafe { &*(self.column.unwrap().get(self.row) as *const RelationVec) };
        rel_vec.contains(&id.0)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    #[test]
    fn size_assumptions() {
        type A = (Vec<u32>, HashSet<u32>);
        type B = Option<A>;
        assert_eq!(size_of::<A>(), size_of::<B>());
    }
}
