use std::any::TypeId;

use crate::{entity_store::Entity, world::World};

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

struct QueryWorker<'world, const VAR_COUNT: usize, const COMP_COUNT: usize> {
    world: &'world World,
    result: ([Entity; VAR_COUNT], [*const u8; COMP_COUNT]),
}

struct QueryIterator<'world, 'me, const VAR_COUNT: usize, const COMP_COUNT: usize> {
    state: &'me mut QueryWorker<'world, VAR_COUNT, COMP_COUNT>,
}

impl<'world, 'me, const VAR_COUNT: usize, const COMP_COUNT: usize> Iterator
    for QueryIterator<'world, 'me, VAR_COUNT, COMP_COUNT>
{
    type Item<'a> = &'a ([Entity; VAR_COUNT], [*const u8; COMP_COUNT]);

    fn next(&mut self) -> Option<Self::Item> {
        let r = &self.state.result;
        let r = unsafe { std::mem::transmute(r) };
        Some(r)
    }
}

fn foobar() {
    let world = World::new();
    let mut state = QueryWorker {
        world: &world,
        result: ([], []),
    };
    let mut iterator = QueryIterator { state: &mut state };
    let a = iterator.next();
    let b = iterator.next();
}
