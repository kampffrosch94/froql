use std::{any::TypeId, collections::HashMap};

use crate::{
    archetype::{Archetype, ArchetypeId, ArchetypeRow},
    component::{Component, ComponentId},
    entity_store::{Entity, EntityStore},
    util::get_mut_2,
};

/// A struct concerned with the nitty gritty of archetype and component management
pub struct Bookkeeping {
    pub component_map: HashMap<TypeId, ComponentId>,
    /// Indexed by ComponentId
    pub components: Vec<Component>,
    /// Indexed by ArchetypeId
    archetypes: Vec<Archetype>,
    /// Indexed by EntityId
    entities: EntityStore,
    /// maps to the Archetype which has all the components in the vec and just those
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

    pub fn get_component_id(&self, tid: TypeId) -> Option<ComponentId> {
        self.component_map.get(&tid).copied()
    }

    pub fn get_component(&self, e: Entity, cid: ComponentId) -> *mut u8 {
        assert!(self.entities.is_alive(e));
        let (aid, row) = self.entities.get_archetype(e);
        let a = &self.archetypes[aid.0 as usize];
        let col = a.components.iter().position(|it| *it == cid).unwrap();
        unsafe { a.columns[col].get(row.0) }
    }

    pub fn has_component(&self, e: Entity, cid: ComponentId) -> bool {
        assert!(self.entities.is_alive(e));
        let (aid, _) = self.entities.get_archetype(e);
        let comp = &self.components[cid.0 as usize];
        comp.has_archetype(aid)
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
        let new_column = components.iter().position(|it| *it == cid).unwrap();
        let new_a_id = self.find_archetype_or_create(components);

        let (old, new) = get_mut_2(&mut self.archetypes, old_a_id.0, new_a_id.0);

        Archetype::move_row(old, new, old_a_row);

        // update entities in the entity storage
        let new_row = (new.entities.len() - 1) as u32;
        self.entities
            .set_archetype(e, new_a_id, ArchetypeRow(new_row));
        if old_a_row.0 < old.entities.len() as u32 {
            // in this case we need to update the entity we swapped into the hole
            let eid = old.entities[old_a_row.0 as usize];
            debug_assert_ne!(eid, e.id);
            self.entities
                .set_archetype_unchecked(eid, old_a_id, old_a_row);
        }

        // the caller must move the new component into the new archetype
        let r = unsafe { new.columns[new_column].half_push() };
        r
    }

    pub fn find_archetype_or_create(&mut self, c_ids: Vec<ComponentId>) -> ArchetypeId {
        // find
        if let Some(id) = self.exact_archetype.get(&c_ids) {
            return *id;
        }

        // create
        let new_aid = ArchetypeId(self.archetypes.len() as u32);
        for cid in &c_ids {
            let c = &mut self.components[cid.0 as usize];
            c.insert_archetype(new_aid);
        }

        let components = c_ids
            .iter()
            .map(|id| &self.components[id.0 as usize])
            .collect::<Vec<_>>();

        let new_archetype = Archetype::new(&components);
        self.archetypes.push(new_archetype);
        self.exact_archetype.insert(c_ids, new_aid);
        new_aid
    }

    pub fn remove_component(&mut self, e: Entity, cid: ComponentId) {
        let (old_a_id, old_a_row) = self.entities.get_archetype(e);
        debug_assert_eq!(
            e.id,
            self.archetypes[old_a_id.0 as usize].entities[old_a_row.0 as usize]
        );
        let mut components = self.archetypes[old_a_id.0 as usize].components.clone();
        let removed_column = components.iter().position(|it| *it == cid).unwrap();
        components.retain(|it| *it != cid);
        let new_a_id = self.find_archetype_or_create(components);

        let (old, new) = get_mut_2(&mut self.archetypes, old_a_id.0, new_a_id.0);

        Archetype::move_row(old, new, old_a_row);
        old.columns[removed_column].remove_swap(old_a_row.0);

        // update entities in the entity storage
        let new_row = (new.entities.len() - 1) as u32;
        self.entities
            .set_archetype(e, new_a_id, ArchetypeRow(new_row));
        if old_a_row.0 < old.entities.len() as u32 {
            // in this case we need to update the entity we swapped into the hole
            let eid = old.entities[old_a_row.0 as usize];
            debug_assert_ne!(eid, e.id);
            self.entities
                .set_archetype_unchecked(eid, old_a_id, old_a_row);
        }

        debug_assert!({
            let expected = old.entities.len();
            old.columns.iter().all(|col| col.len() == expected)
        });
        debug_assert!({
            let expected = new.entities.len();
            new.columns.iter().all(|col| col.len() == expected)
        });
    }

    pub fn destroy(&mut self, e: Entity) {
        if self.entities.is_alive(e) {
            let (aid, arow) = self.entities.get_archetype(e);
            let a = &mut self.archetypes[aid.0 as usize];
            let swapped = a.delete_row(arow);
            self.entities.destroy(e);
            if swapped {
                let swapped_e = a.entities[arow.0 as usize];
                self.entities.set_archetype_unchecked(swapped_e, aid, arow);
            }
        }
    }
}
