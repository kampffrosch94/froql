use crate::{
    component::ComponentId, entity_store::EntityId, layout_vec::LayoutVec,
    relation_vec::RelationVec,
};

/// Helps with Relation Traversal
#[derive(Default)]
pub struct RelationHelper<'a> {
    // TODO replace with MaybeUinit after making sure its safe
    column: Option<&'a LayoutVec>,
    row: u32,
    rel_index: u32,
}

impl<'a> RelationHelper<'a> {
    pub fn new(cid: ComponentId) -> Self {
        if cid.is_transitive() {
            todo!("Transitive is not done yet.");
        }
        RelationHelper::default()
    }

    pub fn set_col(&mut self, column: &'a LayoutVec) {
        self.column = Some(column);
    }

    pub fn set_row(&mut self, row_counter: u32) {
        self.row = row_counter;
        self.rel_index = u32::MAX; // rolls over to 0
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
