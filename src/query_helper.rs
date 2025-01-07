use std::{any::TypeId, ptr::NonNull};

use crate::{
    archetype::{ArchetypeId, ArchetypeRow},
    entity_store::{Entity, EntityGeneration, EntityId},
    world::World,
};

pub struct QueryHelper<'a> {
    world: &'a World,
}

/// RelationType, from_var, to_var
type Relation = (TypeId, usize, usize);
/// ComponentType, source_var
type Component = (TypeId, usize);

/*
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
*/

/// Temporary structure which helps with non trivial multi relationship joins
pub struct JoinTable<'a, const VAR_COUNT: usize, const COMP_COUNT: usize> {
    pub world: &'a World,
    /// says which variables are already resolved
    pub filled: [bool; VAR_COUNT],
    pub rows: Vec<([Entity; VAR_COUNT], [*const u8; COMP_COUNT])>,
}

pub struct QueryWorker<'a> {
    pub world: &'a World,
    pub result_entities: &'a mut [(ArchetypeId, ArchetypeRow)],
    pub result_components: &'a mut [NonNull<u8>],
    pub ops: &'a mut [Op<'a>],
}

impl<'a> QueryWorker<'a> {
    // returns true if it produced a new result
    // returns false if execution can not continue
    pub fn process(&mut self) -> bool {
        false
    }
}

pub enum Op<'a> {
    GrabArchetype(&'a mut u32),
    GrabRow(&'a mut u32),
}

fn foobar() {
    let world = World::new();

    let mut result_entities = [(ArchetypeId(u32::MAX), ArchetypeRow(u32::MAX)); 2];
    let mut result_component = [NonNull::dangling(); 4];
    let mut state = 32;
    let mut ops = [Op::GrabRow(&mut state)];
    let mut worker = QueryWorker {
        world: &world,
        result_entities: &mut result_entities,
        result_components: &mut result_component,
        ops: &mut ops,
    };
    while worker.process() {
        dbg!(&worker.result_entities[0]);
    }
}

fn grab_archetype() {
}
