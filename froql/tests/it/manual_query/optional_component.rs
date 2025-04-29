use froql::{archetype::ArchetypeRow, world::World};
use std::cell::RefCell;

#[test]
#[allow(unused)]
fn manual_query_optional_component() {
    #[derive(Debug)]
    struct CompA(usize);
    #[derive(Debug)]
    struct CompB(isize);
    let mut world = World::new();
    let a = world.create_entity();
    let b = world.create_entity();
    world.add_component(a, CompA(4));
    world.add_component(a, CompB(2));
    world.add_component(b, CompA(0));
    let mut counter = 0;
    for (ca, cb) in {
        let world: &World = &world;
        let bk = &world.bookkeeping;
        let components_0 = [world.get_component_id::<CompA>()];
        let archetype_id_sets = [bk.matching_archetypes(&components_0, &[])];
        const VAR_COUNT: usize = 1;
        let mut a_refs = [&bk.archetypes[0]; VAR_COUNT];
        let mut a_rows = [ArchetypeRow(u32::MAX); VAR_COUNT];
        let mut current_step = 0;
        let mut a_max_rows = [0; VAR_COUNT];
        let mut a_next_indexes = [usize::MAX; VAR_COUNT];
        let mut col_indexes = [usize::MAX; 1];
        let opt_cid_0 = world.get_component_id::<CompB>();
        let mut opt_col_0 = None;
        ::std::iter::from_fn(move || {
            loop {
                match current_step {
                    0 => {
                        const CURRENT_VAR: usize = 0;
                        const CURRENT_VAR_COMPONENTS: ::std::ops::Range<usize> = 0..1;
                        let next_index = &mut a_next_indexes[CURRENT_VAR];
                        let archetype_ids = &archetype_id_sets[CURRENT_VAR];
                        *next_index = next_index.wrapping_add(1);
                        if *next_index >= archetype_ids.len() {
                            return None;
                        }
                        let next_id = archetype_ids[*next_index];
                        a_rows[CURRENT_VAR] = ArchetypeRow(u32::MAX);
                        let a_ref = &mut a_refs[CURRENT_VAR];
                        *a_ref = &bk.archetypes[next_id.as_index()];
                        a_ref.find_multiple_columns(
                            &components_0,
                            &mut col_indexes[CURRENT_VAR_COMPONENTS],
                        );
                        a_max_rows[CURRENT_VAR] = a_ref.entities.len() as u32;
                        opt_col_0 = a_ref.find_column_opt(opt_cid_0);
                        current_step += 1;
                    }
                    1 => {
                        const CURRENT_VAR: usize = 0;
                        let row_counter = &mut a_rows[CURRENT_VAR].0;
                        let max_row = a_max_rows[CURRENT_VAR];
                        *row_counter = row_counter.wrapping_add(1);
                        if *row_counter >= max_row {
                            current_step -= 1;
                        } else {
                            current_step += 1;
                        }
                    }
                    2 => {
                        current_step -= 1;
                        return Some(unsafe {
                            (
                                (&*((&a_refs[0].columns[col_indexes[0]]).get(a_rows[0].0)
                                    as *const RefCell<CompA>))
                                    .borrow(),
                                // opt_col_index, var and type required
                                (opt_col_0.map(|col| {
                                    (&*(col.get(a_rows[0].0) as *const RefCell<CompA>)).borrow()
                                })),
                            )
                        });
                    }
                    _ => {
                        unreachable!();
                    }
                }
            }
        })
    } {
        dbg!(ca);
        dbg!(cb);
        counter += 1;
    }
    match (&2, &counter) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                panic!();
            }
        }
    };
}
