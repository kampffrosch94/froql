use std::{any::TypeId, collections::HashMap};

use crate::{
    archetype::{Archetype, ArchetypeId, ArchetypeRow},
    component::{Component, ComponentId},
    entity_store::{Entity, EntityStore},
    layout_vec::LayoutVec,
};

/// A struct concerned with the nitty gritty of archetype and component management
pub struct Bookkeeping {
    pub component_map: HashMap<TypeId, ComponentId>,
    /// Indexed by ComponentId
    pub components: Vec<Component>,
    /// Indexed by ArchetypeId
    pub archetypes: Vec<Archetype>,
    /// Indexed by EntityId
    pub entities: EntityStore,
    ///
    exact_archetype: HashMap<Vec<ComponentId>, ArchetypeId>,
}

const EMPTY_ARCHETYPE_ID: ArchetypeId = ArchetypeId(0);

// ATTENTION: no function in bookkeeping may be generic over types
// we do this to save the user compile time
impl Bookkeeping {
    pub fn new() -> Self {
        let mut archetypes = Vec::new();
        let empty_archetype = Archetype::new(&[]);
        archetypes.push(empty_archetype);
        let mut exact_archetype = HashMap::new();
        exact_archetype.insert(Vec::new(), ArchetypeId(0));
        Bookkeeping {
            component_map: HashMap::new(),
            components: Vec::new(),
            archetypes,
            entities: EntityStore::new(),
            exact_archetype,
        }
    }

    pub fn create(&mut self) -> Entity {
        let e = self.entities.create();
        let empty_archetype = &mut self.archetypes[EMPTY_ARCHETYPE_ID.0 as usize];
        let row = ArchetypeRow(empty_archetype.entities.len() as u32);
        empty_archetype.entities.push(e.id);
        self.entities.set_archetype(e, EMPTY_ARCHETYPE_ID, row);
        e
    }

    #[must_use]
    pub fn get_component_id(&self, tid: TypeId) -> Option<ComponentId> {
        self.component_map.get(&tid).copied()
    }

    #[must_use]
    pub fn get_component(&self, e: Entity, cid: ComponentId) -> *mut u8 {
        let (aid, row) = self.entities.get_archetype(e);
        let a = &self.archetypes[aid.0 as usize];
        let col = a.components.iter().position(|it| *it == cid).unwrap();
        unsafe { a.columns[col].get(row.0) }
    }

    // TODO: handle ZSTs differently
    #[must_use]
    pub fn add_component(&mut self, e: Entity, cid: ComponentId) -> *mut u8 {
        let (old_a_id, old_a_row) = self.entities.get_archetype(e);
        debug_assert_eq!(
            e.id,
            self.archetypes[old_a_id.0 as usize].entities[old_a_row.0 as usize]
        );
        let mut components = self.archetypes[old_a_id.0 as usize].components.clone();
        components.push(cid);
        components.sort();
        let new_a_id = self.find_archetype_or_create(components);
        // TODO factor into util function
        let (old, new) = {
            let oi = old_a_id.0 as usize;
            let ni = new_a_id.0 as usize;
            if old_a_id < new_a_id {
                let (old_slice, new_slice) = self.archetypes.split_at_mut(ni);
                (&mut old_slice[oi], &mut new_slice[0])
            } else {
                let (new_slice, old_slice) = self.archetypes.split_at_mut(oi);
                (&mut old_slice[0], &mut new_slice[ni])
            }
        };

        // transfer every component over from the old archetype
        let mut offset = 0; // used to skip the new column while copying over
        let mut new_column = 0; // column for the new component
        let mut new_row = 0; // row in the new archetype
        let mut old_swapped = 0; // row which got swapped to fill holes in the old archetype
        for i in 0..old.components.len() {
            if old.components[i] != new.components[i] {
                offset += 1;
                new_column = i;
            }
            let from = &mut old.columns[i];
            let to = &mut new.columns[i + offset];
            unsafe {
                (old_swapped, new_row) = LayoutVec::move_entry(from, to, old_a_row.0);
            }
        }
        if offset == 0 {
            // new column was not set because we finished the loop before finding the extra column
            new_column = old.components.len();
        }
        debug_assert!(
            offset <= 1,
            "\nOld: {:?}\nNew: {:?}",
            &old.components,
            &new.components
        );

        // update entities in the entity storage
        self.entities
            .set_archetype(e, new_a_id, ArchetypeRow(new_row));
        new.entities.push(e.id);
        if old_swapped != old.entities.len() as u32 - 1 {
            // in this case we need to update the entity we swapped into the hole
            let eid = old.entities[old_swapped as usize];
            debug_assert_ne!(eid, e.id);
            self.entities
                .set_archetype_unchecked(eid, old_a_id, old_a_row);
        }
        old.entities.swap_remove(old_swapped as usize);

        // the caller must move the new component into the new archetype
        let r = unsafe { new.columns[new_column].half_push() };
        r
    }

    pub fn find_archetype_or_create(&mut self, c_ids: Vec<ComponentId>) -> ArchetypeId {
        if let Some(id) = self.exact_archetype.get(&c_ids) {
            return *id;
        }
        let components = c_ids
            .iter()
            .map(|id| &self.components[id.0 as usize])
            .collect::<Vec<_>>();
        let new_id = ArchetypeId(self.archetypes.len() as u32);
        let new_archetype = Archetype::new(&components);
        self.archetypes.push(new_archetype);
        self.exact_archetype.insert(c_ids, new_id);
        new_id
    }
}
