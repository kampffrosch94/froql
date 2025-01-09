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

#[cfg(test)]
mod test {
    use std::{any::TypeId, cell::RefCell, ops::Range};

    use crate::{
        archetype::{ArchetypeId, ArchetypeRow},
        entity_store::EntityId,
        relation::Relation,
        relation_vec::RelationVec,
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
        world.add_component(c, CompB("World".to_string()));
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

            // result set
            const VAR_COUNT: usize = 1;
            let mut a_ids = [ArchetypeId(u32::MAX); VAR_COUNT];
            let mut a_rows = [ArchetypeRow(u32::MAX); VAR_COUNT];

            // context for statemachine
            let mut current_step = 0;
            let mut a_max_rows = [0; VAR_COUNT];
            let mut col_ids = [usize::MAX; 2];
            // gets rolled over to 0 by wrapping_add
            let mut a_next_indexes = [usize::MAX; VAR_COUNT];
            let mut a_refs = [&bk.archetypes[0]; VAR_COUNT];

            std::iter::from_fn(move || {
                loop {
                    match current_step {
                        // next archetype
                        0 => {
                            const CURRENT_VAR: usize = 0;
                            const CURRENT_VAR_COMPONENTS: Range<usize> = 0..2;
                            let next_index = &mut a_next_indexes[CURRENT_VAR];
                            *next_index = next_index.wrapping_add(1);
                            if *next_index >= archetype_ids.len() {
                                return None;
                            }
                            a_ids[CURRENT_VAR] = archetype_ids[*next_index];

                            // gets rolled over to 0 by wrapping_add
                            a_rows[0] = ArchetypeRow(u32::MAX);
                            let a_ref = &mut a_refs[CURRENT_VAR];
                            *a_ref = &bk.archetypes[a_ids[CURRENT_VAR].as_index()];
                            a_ref.find_multiple_columns(
                                &components,
                                &mut col_ids[CURRENT_VAR_COMPONENTS],
                            );
                            a_max_rows[0] = a_ref.entities.len() as u32;
                            current_step += 1;
                        }
                        // next row in archetype
                        1 => {
                            const CURRENT_VAR: usize = 0;
                            let row_counter = &mut a_rows[CURRENT_VAR].0;
                            let max_row = a_max_rows[CURRENT_VAR];
                            // rolls over to 0 for u32::MAX, which is our start value
                            *row_counter = row_counter.wrapping_add(1);

                            if *row_counter >= max_row {
                                current_step -= 1;
                            } else {
                                current_step += 1;
                            }
                        }
                        // yield row
                        2 => {
                            let arch = a_refs[0];
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
            counter += 1;
        }
        assert_eq!(2, counter);
    }

    #[test]
    #[allow(unused)]
    fn manual_query_helper_relation() {
        enum Attack {}

        #[derive(Debug)]
        struct Unit(String);
        #[derive(Debug)]
        struct Health(isize);

        let mut world = World::new();
        let player = world.create();
        world.add_component(player, Unit("Player".to_string()));
        let goblin_a = world.create();
        world.add_component(goblin_a, Health(10));
        world.add_component(goblin_a, Unit("Goblin A".to_string()));
        world.add_relation::<Attack>(player, goblin_a);

        let goblin_b = world.create();
        world.add_component(goblin_b, Health(10));
        world.add_component(goblin_b, Unit("Goblin B".to_string()));
        world.add_relation::<Attack>(player, goblin_b);

        // this should not be matched by the query below
        // bad example I know, but I need something
        let trap = world.create();
        world.add_relation::<Attack>(trap, goblin_b);

        let mut counter = 0;

        // manual query for:
        // query!(world, Unit(me), Unit(other), Hp(me), Attack(other, me))
        for (me, other, mut hp) in {
            let world: &World = &world;
            let bk = &world.bookkeeping;
            let components_me = [
                // 0
                world.get_component_id::<Unit>(),
                // 1
                world.get_component_id::<Health>(),
                // 2
                bk.get_component_id_unchecked(TypeId::of::<Relation<Attack>>())
                    .flip_target(),
            ];
            let components_other = [
                // 3
                world.get_component_id::<Unit>(),
                // 4
                bk.get_component_id_unchecked(TypeId::of::<Relation<Attack>>()),
            ];
            let archetype_ids_me = bk.matching_archetypes(&components_me, &[]);
            let archetype_ids_other = bk.matching_archetypes(&components_other, &[]);
            let archetype_id_sets = [archetype_ids_me, archetype_ids_other];

            // result set
            const VAR_COUNT: usize = 2;
            let mut a_refs = [&bk.archetypes[0]; VAR_COUNT];
            let mut a_rows = [ArchetypeRow(u32::MAX); VAR_COUNT];

            // context for statemachine
            let mut current_step = 0;
            let mut a_max_rows = [0; VAR_COUNT];
            let mut col_indexes = [usize::MAX; 5];
            assert_eq!(
                col_indexes.len(),
                components_me.len() + components_other.len()
            );
            // gets rolled over to 0 by wrapping_add
            let mut a_next_indexes = [usize::MAX; VAR_COUNT];
            let mut rel_index_2 = 0;

            let mut current_step = 0;
            std::iter::from_fn(move || {
                loop {
                    match current_step {
                        // next archetype
                        0 => {
                            const CURRENT_VAR: usize = 0;
                            const CURRENT_VAR_COMPONENTS: Range<usize> = 0..3;
                            let next_index = &mut a_next_indexes[CURRENT_VAR];
                            let archetype_ids = &archetype_id_sets[CURRENT_VAR];
                            *next_index = next_index.wrapping_add(1);
                            if *next_index >= archetype_ids.len() {
                                return None;
                            }
                            let next_id = archetype_ids[*next_index];

                            // gets rolled over to 0 by wrapping_add
                            a_rows[0] = ArchetypeRow(u32::MAX);
                            let a_ref = &mut a_refs[CURRENT_VAR];
                            *a_ref = &bk.archetypes[next_id.as_index()];
                            a_ref.find_multiple_columns(
                                &components_me,
                                &mut col_indexes[CURRENT_VAR_COMPONENTS],
                            );
                            a_max_rows[0] = a_ref.entities.len() as u32;
                            current_step += 1;
                        }
                        // next row in archetype
                        1 => {
                            const CURRENT_VAR: usize = 0;
                            let row_counter = &mut a_rows[CURRENT_VAR].0;
                            let max_row = a_max_rows[CURRENT_VAR];
                            // rolls over to 0 for u32::MAX, which is our start value
                            *row_counter = row_counter.wrapping_add(1);

                            if *row_counter >= max_row {
                                current_step -= 1;
                            } else {
                                current_step += 1;
                            }
                        }
                        // follow relation
                        2 => {
                            const CURRENT_VAR: usize = 0;
                            const REL_VAR: usize = 1;
                            const RELATION_COMP_INDEX: usize = 2;
                            const REL_VAR_COMPONENTS: Range<usize> = 3..5;
                            let row = a_rows[CURRENT_VAR].0;
                            let col = col_indexes[RELATION_COMP_INDEX];
                            let arch = &a_refs[CURRENT_VAR];
                            debug_assert_eq!(
                                arch.columns[col].element_size(),
                                size_of::<RelationVec>()
                            );
                            let ptr = unsafe { arch.columns[col].get(row) } as *const RelationVec;
                            let rel_vec = unsafe { &*ptr };
                            debug_assert!(rel_vec.len() > 0);
                            if rel_index_2 >= rel_vec.len() {
                                rel_index_2 = 0;
                                current_step -= 1;
                            } else {
                                // get aid/row for entity in relation
                                let id = EntityId(rel_vec[rel_index_2 as usize]);
                                let (aid, arow) = bk.entities.get_archetype_unchecked(id);
                                rel_index_2 += 1;

                                // if in target archetype set => go to next step
                                if archetype_id_sets[REL_VAR].contains(&aid) {
                                    let a_ref = &mut a_refs[REL_VAR];
                                    *a_ref = &bk.archetypes[aid.as_index()];
                                    a_ref.find_multiple_columns(
                                        &components_other,
                                        &mut col_indexes[REL_VAR_COMPONENTS],
                                    );
                                    a_rows[REL_VAR] = arow;

                                    current_step += 1;
                                }
                            }
                        }
                        // yield row
                        3 => {
                            let arch_me = a_refs[0];
                            let arch_other = a_refs[1];
                            let row_me = a_rows[0].0;
                            let row_other = a_rows[1].0;
                            current_step -= 1;
                            return Some(unsafe {
                                (
                                    (&*((&arch_me.columns[col_indexes[0]]).get(row_me)
                                        as *const RefCell<Unit>))
                                        .borrow(),
                                    (&*((&arch_other.columns[col_indexes[3]]).get(row_other)
                                        as *const RefCell<Unit>))
                                        .borrow(),
                                    (&*((&arch_me.columns[col_indexes[1]]).get(row_me)
                                        as *const RefCell<Health>))
                                        .borrow_mut(),
                                )
                            });
                        }
                        _ => unreachable!(),
                    }
                }
            })
        } {
            println!("\nHp before: {hp:?}");
            println!("{me:?} attacked by {other:?}");
            hp.0 -= 5;
            println!("Hp now: {hp:?}");
            counter += 1;
        }
        //assert_eq!(2, counter);
        assert_eq!(2, counter);
    }
}
