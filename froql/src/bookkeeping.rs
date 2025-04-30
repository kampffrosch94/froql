use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
};

use hi_sparse_bitset::reduce;

use crate::{
    archetype::{Archetype, ArchetypeId, ArchetypeRow},
    component::{Component, ComponentId},
    entity_store::{Entity, EntityId, EntityStore},
    layout_vec::LayoutVec,
    relation_vec::RelationVec,
    util::get_mut_2,
};

/// A struct concerned with the nitty gritty of archetype and component management
pub struct Bookkeeping {
    /// maps TypeId to ComponentId
    /// for relationships only the Origin Relationship ID is returned
    /// to get the ID for the target use `.flip_target()`
    pub component_map: HashMap<TypeId, ComponentId>,
    // used for hotreloading
    pub component_name_map: HashMap<String, TypeId>,
    /// Indexed by ComponentId
    pub components: Vec<Component>,
    /// Indexed by ArchetypeId
    pub archetypes: Vec<Archetype>,
    /// Indexed by EntityId
    pub entities: EntityStore,
    /// maps to the Archetype which has all the components in the vec and just those
    pub exact_archetype: HashMap<Vec<ComponentId>, ArchetypeId>,
}

const EMPTY_ARCHETYPE_ID: ArchetypeId = ArchetypeId(0);

// ATTENTION: no function in bookkeeping may be generic over types
// we do this to save the user compile time
impl Bookkeeping {
    pub fn new() -> Self {
        let mut archetypes = Vec::new();
        let empty_archetype = Archetype::new(Vec::new(), Vec::new());
        archetypes.push(empty_archetype);
        let mut exact_archetype = HashMap::new();
        exact_archetype.insert(Vec::new(), ArchetypeId(0));
        Bookkeeping {
            component_map: HashMap::new(),
            component_name_map: HashMap::new(),
            components: Vec::new(),
            archetypes,
            entities: EntityStore::new(),
            exact_archetype,
        }
    }

    pub fn is_alive(&self, e: Entity) -> bool {
        self.entities.is_alive(e)
    }

    fn create_inner(&mut self) -> Entity {
        let e = self.entities.create();
        let empty_archetype = &mut self.archetypes[EMPTY_ARCHETYPE_ID.0 as usize];
        let row = ArchetypeRow(empty_archetype.entities.len() as u32);
        empty_archetype.entities.push(e.id);
        self.entities.set_archetype(e, EMPTY_ARCHETYPE_ID, row);
        e
    }

    pub fn create(&mut self) -> Entity {
        self.realize_deferred();
        self.create_inner()
    }

    pub fn create_deferred(&self) -> Entity {
        self.entities.create_deferred()
    }

    pub fn realize_deferred(&mut self) {
        for _ in 0..self.entities.realize_deferred() {
            self.create_inner();
        }
    }

    pub fn ensure_alive(&mut self, id: EntityId) -> Entity {
        self.realize_deferred();

        use crate::entity_store::ForceAliveResult as R;
        match self.entities.force_alive(id) {
            R::WasAliveBefore(e) => e,
            R::MadeAlive(e) => {
                // put in empty archetype, like create() above
                let empty_archetype = &mut self.archetypes[EMPTY_ARCHETYPE_ID.0 as usize];
                let row = ArchetypeRow(empty_archetype.entities.len() as u32);
                empty_archetype.entities.push(e.id);
                self.entities.set_archetype(e, EMPTY_ARCHETYPE_ID, row);
                e
            }
        }
    }

    pub fn get_component_id(&self, tid: TypeId) -> Option<ComponentId> {
        self.component_map.get(&tid).copied()
    }

    pub fn get_component_id_unchecked(&self, tid: TypeId) -> ComponentId {
        self.component_map
            .get(&tid)
            .copied()
            .expect("TypeId is not registered as Component.")
    }

    pub fn get_component(&self, e: Entity, cid: ComponentId) -> *mut u8 {
        assert!(self.entities.is_alive(e));
        let (aid, row) = self.entities.get_archetype(e);
        let a = &self.archetypes[aid.0 as usize];
        let col = a.find_column(cid);
        unsafe { col.get(row.0) }
    }

    pub fn get_component_opt_unchecked(&self, e: EntityId, cid: ComponentId) -> Option<*mut u8> {
        let (aid, row) = self.entities.get_archetype_unchecked(e);
        let a = &self.archetypes[aid.0 as usize];
        let col = a.find_column_opt(cid);
        col.map(|col| unsafe { col.get(row.0) })
    }

    pub fn has_component(&self, e: Entity, cid: ComponentId) -> bool {
        if !self.entities.is_alive(e) {
            // dead entities have no components
            return false;
        }
        let (aid, _) = self.entities.get_archetype(e);
        let comp = &self.components[cid.as_index()];
        comp.has_archetype(aid, cid)
    }

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

        unsafe { new.columns[new_column].half_push() }
    }

    fn find_archetype_or_create(&mut self, c_ids: Vec<ComponentId>) -> ArchetypeId {
        // find
        if let Some(id) = self.exact_archetype.get(&c_ids) {
            return *id;
        }

        // create
        let new_aid = ArchetypeId(self.archetypes.len() as u32);
        for cid in &c_ids {
            let c = &mut self.components[cid.as_index()];
            c.insert_archetype(new_aid, *cid);
        }

        // the archetype only knows about components that have a size
        let a_components = c_ids
            .iter()
            .filter(|id| self.components[id.as_index()].layout.size() > 0)
            .cloned()
            .collect::<Vec<_>>();
        let columns = a_components
            .iter()
            .map(|id| &self.components[id.as_index()])
            .map(|c| LayoutVec::new(c.layout, c.drop_fn.clone()))
            .collect::<Vec<_>>();

        let new_archetype = Archetype::new(a_components, columns);
        self.archetypes.push(new_archetype);
        self.exact_archetype.insert(c_ids, new_aid);
        new_aid
    }

    /// Works for normal components and ZSTs
    pub fn remove_component(&mut self, e: Entity, cid: ComponentId) {
        let (old_a_id, old_a_row) = self.entities.get_archetype(e);
        debug_assert_eq!(
            e.id,
            self.archetypes[old_a_id.0 as usize].entities[old_a_row.0 as usize]
        );
        let mut components = self.archetypes[old_a_id.0 as usize].components.clone();
        let removed_column = components.iter().position(|it| *it == cid);
        components.retain(|it| *it != cid);
        let new_a_id = self.find_archetype_or_create(components);

        let (old, new) = get_mut_2(&mut self.archetypes, old_a_id.0, new_a_id.0);

        Archetype::move_row(old, new, old_a_row);
        if let Some(removed_column) = removed_column {
            debug_assert!(self.components[cid.as_index()].layout.size() > 0);
            old.columns[removed_column].remove_swap(old_a_row.0);
        } else {
            debug_assert!(self.components[cid.as_index()].layout.size() == 0);
        }

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
            old.columns.iter().all(|col| col.len() == expected as u32)
        });
        debug_assert!({
            let expected = new.entities.len();
            new.columns.iter().all(|col| col.len() == expected as u32)
        });
    }

    pub fn matching_archetypes(
        &self,
        with: &[ComponentId],
        without: &[ComponentId],
    ) -> Vec<ArchetypeId> {
        debug_assert!(with.len() + without.len() > 0);
        use hi_sparse_bitset::ops::{And, Or};
        if without.is_empty() {
            // simplest case
            let with_sets = with
                .iter()
                .copied()
                .map(|cid| self.components[cid.as_index()].get_archetype_bitset(cid));
            let b_with = reduce(And, with_sets).unwrap();
            let ids: Vec<_> = b_with
                .into_iter()
                .map(|id| ArchetypeId(id as u32))
                .collect();
            ids
        } else if with.is_empty() {
            // don't think this case is great for performance personally, but 仕方がない
            let without_sets = without
                .iter()
                .copied()
                .map(|cid| self.components[cid.as_index()].get_archetype_bitset(cid));
            // union
            let b_without = reduce(Or, without_sets).unwrap();
            // invert
            let result = (0..self.archetypes.len()).filter(|index| !b_without.contains(*index));

            let ids: Vec<_> = result.map(|id| ArchetypeId(id as u32)).collect();
            ids
        } else {
            // general case
            let with_sets = with
                .iter()
                .copied()
                .map(|cid| self.components[cid.as_index()].get_archetype_bitset(cid));
            // intersect
            let b_with = reduce(And, with_sets).unwrap();
            let without_sets = without
                .iter()
                .copied()
                .map(|cid| self.components[cid.as_index()].get_archetype_bitset(cid));
            // union
            let b_without = reduce(Or, without_sets).unwrap();

            // subtract
            let result = b_with - b_without;

            // returning a vec because otherwise lifetimes get really annoying
            // and also impl Iterator<..> with a monstrosity of a
            // type like the sparse bitset produces can't be good for compile times
            let ids: Vec<_> = result
                .into_iter()
                .map(|id| ArchetypeId(id as u32))
                .collect();
            ids
        }
    }

    pub fn destroy(&mut self, e: Entity) {
        self.realize_deferred(); // need to do that so we don't break the free list

        if self.entities.is_alive(e) {
            let (a_id, a_row) = self.entities.get_archetype(e);
            let a = &self.archetypes[a_id.0 as usize];

            // first clean up all relationships pointing to this component
            // or being pointed to from this component

            let mut to_delete = Vec::new(); // just here to avoid borrow checker
            let mut to_destroy = Vec::new(); // for cascading destruction
            for (index, cid) in a.components.iter().enumerate() {
                if cid.is_relation() {
                    let ptr = unsafe { a.columns[index].get(a_row.0) } as *const RelationVec;
                    let vec = unsafe { &*ptr };
                    debug_assert!(!vec.is_empty());
                    let flipped = cid.flip_target();
                    for other_id in vec.iter() {
                        to_delete.push((flipped, EntityId(*other_id)));
                    }
                    if cid.is_cascading() {
                        for other_id in vec.iter() {
                            to_destroy.push(EntityId(*other_id));
                        }
                    }
                }
            }

            // delete the row from the archetype
            let a = &mut self.archetypes[a_id.0 as usize];
            let swapped = a.delete_row(a_row);
            self.entities.destroy(e);
            if swapped {
                let swapped_e = a.entities[a_row.0 as usize];
                self.entities
                    .set_archetype_unchecked(swapped_e, a_id, a_row);
            }

            // delete ourselves from our relation partners
            for (cid, other_id) in to_delete {
                let (a_id, a_row) = self.entities.get_archetype_unchecked(other_id);
                let a = &mut self.archetypes[a_id.0 as usize];
                let col = a.components.iter().position(|it| *it == cid).unwrap();
                let ptr = unsafe { a.columns[col].get(a_row.0) } as *mut RelationVec;
                let rel_vec = unsafe { &mut *ptr };
                rel_vec.remove(e.id.0);
                if rel_vec.is_empty() {
                    let other = self.entities.get_from_id(other_id);
                    // this moves the other entity
                    self.remove_component(other, cid);
                }
            }

            // cascading destruction if necessary
            for other_id in to_destroy {
                let other_e = self.entities.get_from_id(other_id);
                self.destroy(other_e);
            }
        }
    }

    pub fn add_relation(&mut self, cid: ComponentId, from: Entity, to: Entity) {
        debug_assert!(cid.is_relation());
        debug_assert!(!cid.is_target());
        inner(self, cid, from, to);
        inner(self, cid.flip_target(), to, from);
        // inner function because removing the relationship component
        // from Origin and Target works the same, just gotta swap arguments
        fn inner(this: &mut Bookkeeping, cid: ComponentId, e: Entity, other: Entity) {
            // all relationtypes are repr(transparent) to RelationVec,
            // so we can just treat pointers to them as RelationVec
            if this.has_component(e, cid) {
                let ptr = this.get_component(e, cid) as *mut RelationVec;
                let rel_vec = unsafe { &mut *ptr };
                if cid.is_exclusive() {
                    rel_vec[0] = other.id.0;
                } else {
                    rel_vec.push(other.id.0);
                }
            } else {
                let mut rel_vec = RelationVec::new();
                rel_vec.push(other.id.0);
                let ptr = this.add_component(e, cid) as *mut RelationVec;
                unsafe { std::ptr::write(ptr, rel_vec) };
            }
        }
    }

    pub fn remove_relation(&mut self, cid: ComponentId, from: Entity, to: Entity) {
        debug_assert!(cid.is_relation());
        debug_assert!(!cid.is_target());
        inner(self, cid, from, to);
        inner(self, cid.flip_target(), to, from);
        // inner function because adding the necessary component
        // to Origin and Target works the same, just gotta swap arguments
        fn inner(this: &mut Bookkeeping, cid: ComponentId, e: Entity, other: Entity) {
            // all relationtypes are repr(transparent) to RelationVec,
            // so we can just treat pointers to them as RelationVec
            if this.has_component(e, cid) {
                let ptr = this.get_component(e, cid) as *mut RelationVec;
                let rel_vec = unsafe { &mut *ptr };
                rel_vec.remove(other.id.0);
                if rel_vec.is_empty() {
                    this.remove_component(e, cid);
                }
            }
        }
    }

    pub fn has_relation(&self, origin_cid: ComponentId, from: Entity, to: Entity) -> bool {
        // all relationtypes are repr(transparent) to RelationVec,
        // so we can just treat pointers to them as RelationVec
        debug_assert!(!origin_cid.is_target());
        if self.has_component(from, origin_cid) {
            let ptr = self.get_component(from, origin_cid) as *const RelationVec;
            let rel_vec = unsafe { &*ptr };
            if rel_vec.contains(&to.id.0) {
                return true;
            }
            if origin_cid.is_transitive() {
                // now we need to follow the transitive relationship
                let mut visited = HashSet::new();
                let mut work = Vec::new();
                work.extend_from_slice(rel_vec);
                visited.extend(work.iter().copied());
                while !work.is_empty() {
                    let current = EntityId(work.pop().unwrap());
                    let comp_opt = self.get_component_opt_unchecked(current, origin_cid);
                    if let Some(ptr) = comp_opt {
                        let rel_vec = unsafe { &*(ptr as *const RelationVec) };
                        if rel_vec.contains(&to.id.0) {
                            return true;
                        }
                        for id in &rel_vec[..] {
                            if !visited.contains(id) {
                                work.push(*id);
                                visited.insert(*id);
                            }
                        }
                    }
                }
            }
        }
        return false;
    }

    /// Returns all directly related partners
    /// DOES NOT follow transitive relations
    pub fn relation_partners(
        &self,
        relation_cid: ComponentId,
        e: Entity,
    ) -> Option<impl Iterator<Item = Entity> + use<'_>> {
        if self.has_component(e, relation_cid) {
            let ptr = self.get_component(e, relation_cid) as *mut RelationVec;
            let rel_vec = unsafe { &mut *ptr };
            return Some(
                rel_vec
                    .iter()
                    .map(|id| self.entities.get_from_id(EntityId(*id))),
            );
        }
        None
    }

    /// Returns all directly related pairs
    /// DOES NOT follow transitive relations
    pub fn relation_pairs(&self, tid: TypeId) -> Vec<(Entity, Entity)> {
        let cid = self.get_component_id(tid).unwrap(); // TODO error msg
        let c = &self.components[cid.as_index()];
        let archetypes = c.get_archetypes();
        let entities = archetypes
            .flat_map(|aid| self.archetypes[aid.as_index()].entities.iter())
            .map(|id| self.entities.get_from_id(*id));
        entities
            .flat_map(|e| {
                self.relation_partners(cid, e)
                    .into_iter()
                    .flatten()
                    .map(move |other| (e, other))
            })
            .collect()
    }
}
