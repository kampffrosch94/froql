use std::{any::TypeId, ptr::NonNull};

use crate::{
    archetype::{ArchetypeId, ArchetypeRow},
    bookkeeping::Bookkeeping,
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

pub struct ResultData<'a> {
    pub var_archetype_id: &'a mut [ArchetypeId],
    pub var_archetype_row: &'a mut [ArchetypeRow],
    pub result_components: &'a mut [NonNull<u8>],
    pub col_ids: &'a mut [usize],
}

pub fn grab_archetype_id(
    var_archetype_id: &mut [ArchetypeId],
    var_archetype_row: &mut [ArchetypeRow],
    variable: usize,
    archetype_set: &Vec<ArchetypeId>,
    next_index: &mut usize, // needs to start with 0 before first call
) -> bool {
    if *next_index >= archetype_set.len() {
        return false;
    }
    var_archetype_id[variable] = archetype_set[*next_index];
    var_archetype_row[variable] = ArchetypeRow(u32::MAX);
    *next_index += 1;
    return true;
}

fn grab_archetype_row(
    var_archetype_id: &mut [ArchetypeId],
    var_archetype_row: &mut [ArchetypeRow],
    variable: usize,
    bk: &Bookkeeping,
) -> bool {
    let var_row = &mut var_archetype_row[variable];
    let var_a = &var_archetype_id[variable];
    if var_row.0 == u32::MAX {
        var_row.0 = 0;
    } else {
        var_row.0 += 1;
    }
    if var_row.0 as usize >= bk.archetypes[var_a.0 as usize].entities.len() {
        return false;
    }
    return true;
}

/// Iterator that continues as long as the inner closure returns Some()
struct ClosureIterator<F> {
    closure: F,
}

impl<F, T> Iterator for ClosureIterator<F>
where
    F: FnMut() -> Option<T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        (self.closure)()
    }
}

impl<F, T> ClosureIterator<F>
where
    F: FnMut() -> Option<T>,
{
    pub fn new(closure: F) -> Self {
        Self { closure }
    }
}

#[cfg(test)]
mod test {
    use std::cell::RefCell;

    use crate::{
        archetype::{ArchetypeId, ArchetypeRow},
        query_helper::{grab_archetype_id, grab_archetype_row, ClosureIterator},
        world::World,
    };

    #[test]
    fn manual_query_helper_trivial() {
        #[derive(Debug)]
        struct CompA(usize);
        #[derive(Debug)]
        struct CompB(String);
        struct CompC {}

        let mut world = World::new();
        let a = world.create();
        world.add_component(a, CompA(42));
        world.add_component(a, CompB("Hello".to_string()));
        let b = world.create();
        world.add_component(b, CompA(21));
        let c = world.create();
        world.add_component(c, CompA(42));
        world.add_component(c, CompB("Hello".to_string()));
        world.add_component(c, CompC {});

        let mut counter = 0;
        for (comp_a, comp_b) in {
            let world: &World = &world;
            let bk = &world.bookkeeping;
            let components = [
                world.get_component_id::<CompA>(),
                world.get_component_id::<CompB>(),
            ];
            let archetype_ids = bk.matching_archetypes(&components, &[]);

            // array length is amount of variable
            const VAR_COUNT: usize = 1;
            let mut a_ids = [ArchetypeId(u32::MAX); VAR_COUNT];
            let mut a_rows = [ArchetypeRow(u32::MAX); VAR_COUNT];
            let mut col_ids = [usize::MAX; 2];

            //let mut result_component = [NonNull::dangling(); 4];

            let mut next_index = 0;

            let mut current_step = 0;
            ClosureIterator::new(move || {
                loop {
                    match current_step {
                        // next archetype
                        0 => {
                            if grab_archetype_id(
                                &mut a_ids,
                                &mut a_rows,
                                0,
                                &archetype_ids,
                                &mut next_index,
                            ) {
                                let arch = &bk.archetypes[a_ids[0].0 as usize];
                                arch.find_multiple_columns(&components, &mut col_ids[0..2]);
                                current_step += 1;
                            } else {
                                return None;
                            }
                        }
                        // next row in archetype
                        1 => {
                            // TODO just define maxlength as context and iterate to that
                            let next = grab_archetype_row(&mut a_ids, &mut a_rows, 0, bk);
                            current_step += if next { 1 } else { -1 };
                        }
                        // yield row
                        2 => {
                            let arch = &bk.archetypes[a_ids[0].0 as usize];
                            let row = a_rows[0].0;
                            current_step -= 1;
                            return Some(unsafe {
                                (
                                    (&*((&arch.columns[col_ids[0]]).get(row)
                                        as *const RefCell<CompA>))
                                        .borrow(),
                                    (&*((&arch.columns[col_ids[1]]).get(row)
                                        as *const RefCell<CompB>))
                                        .borrow(),
                                )
                            });
                        }
                        _ => unreachable!(),
                    }
                }
            })
        } {
            println!("{comp_a:?}");
            println!("{comp_b:?}");
            assert_eq!(42, comp_a.0);
            assert_eq!("Hello", &comp_b.0);
            counter += 1;
        }
        assert_eq!(2, counter);
    }
}
