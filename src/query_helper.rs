use std::{any::TypeId, mem::MaybeUninit};

use crate::{
    entity_store::Entity,
    world::World,
};

pub struct QueryHelper<'a> {
    world: &'a World,
}

/// RelationType, from_var, to_var
type Relation = (TypeId, usize, usize);
/// ComponentType, source_var
type Component = (TypeId, usize);

// OPTIMIZATION: keep the body of this function as small as possible to save on monomorphisation
/// used by the proc macro
#[allow(unused)]
pub unsafe fn relation_join_iter_components<'a, const VARS: usize, const COMPS: usize>(
    world: &'a World,
    relations: &[Relation],
    components: [Component; COMPS],
    unequals: &[(usize, usize)],
    uncomponents: &[Component],
    unrelations: &[Relation],
    prefill: &[(usize, Entity)],
) -> impl Iterator<Item = ([Entity; VARS], [*const u8; COMPS])> + use<'a, VARS, COMPS> {
    let join_table: JoinTable<'_, VARS> = JoinTable::new_init(
        world,
        relations,
        &components,
        unequals,
        uncomponents,
        unrelations,
        prefill,
    );
    join_table.rows.into_iter().filter_map(move |row| {
        let mut result = [MaybeUninit::<*const u8>::uninit(); COMPS];
        for i in 0..COMPS {
            let (tid, id) = components[i];
            let res = unsafe { Some(todo!()) }?;
            result[i].write(res);
        }
        Some((row, result.map(|entry| unsafe { entry.assume_init() })))
    })
}

/// Temporary structure which helps with non trivial multi relationship joins
pub struct JoinTable<'a, const N: usize> {
    pub world: &'a World,
    pub filled: [bool; N],
    pub rows: Vec<[Entity; N]>,
}

impl<'a, const COLUMN_COUNT: usize> JoinTable<'a, { COLUMN_COUNT }> {
    pub fn new(world: &'a World) -> Self {
        JoinTable {
            world,
            filled: [false; COLUMN_COUNT],
            rows: Vec::new(),
        }
    }

    pub fn new_init(
        world: &'a World,
        relations: &[Relation],
        components: &[Component],
        _unequals: &[(usize, usize)],
        _uncomponents: &[Component], // Not needed anymore
        _unrelations: &[Relation],
        prefill: &[(usize, Entity)],
    ) -> Self {
        let _bk = &world.bookkeeping;
        let join_table = JoinTable::new(world);
        // init
        match (
            components.is_empty(),
            prefill.is_empty(),
            relations.is_empty(),
        ) {
            (true, true, true) => {}
            (false, true, true) => {
                //join_table.init_from_component(components[0]);
                //join_table.join_components(&components[1..]);
                if COLUMN_COUNT > 1 {
                    unimplemented!("Cross Joins are not supported.")
                }
                todo!();
            }
            (_, true, false) => {
                //let joins = compute_join_order(&relations);
                //join_table.init_from_relation(joins[0]);
                //join_table.join_relations(&joins[1..]);
                todo!();
            }
            (_, false, true) => {
                //join_table.prefill_columns(prefill);
                //join_table.join_components(&components);
                todo!();
            }
            (_, false, false) => {
                //let joins = compute_join_order(&relations);
                //join_table.prefill_columns(prefill);
                //join_table.join_relations(&joins);
                todo!();
            }
        }
        //join_table.join_uncomponents(&uncomponents);
        //join_table.join_unrelations(&unrelations);
        //join_table.remove_by_constraint_unequals(unequals);
        debug_assert!(join_table.filled.iter().all(|filled| *filled));
        join_table
    }

    pub fn new_no_relation(
        world: &'a World,
        _components: &[Component],
        _uncomponents: &[Component],
    ) -> Self {
        let join_table = JoinTable::new(world);
        //let cid_a = bk.get_component_id(TypeId::of::<RefCell<CompA>>()).unwrap();
        //let cid_b = bk.get_component_id(TypeId::of::<RefCell<CompB>>()).unwrap();
        //let archetypes = bk.matching_archetypes(&[cid_a, cid_b], &[]);
        debug_assert!(join_table.filled.iter().all(|filled| *filled));
        join_table
    }
}
