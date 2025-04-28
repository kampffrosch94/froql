use std::cell::RefCell;

use froql::archetype::ArchetypeId;
use froql::archetype::ArchetypeRow;
use froql::query_helper::coerce_cast;
use froql::world::World;
use std::ops::Range;

#[test]
fn iterator_query() {
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
        let components_0 = [
            world.get_component_id::<CompA>(),
            world.get_component_id::<CompB>(),
        ];
        let archetype_id_sets = [bk.matching_archetypes(&components_0, &[])];
        const VAR_COUNT: usize = 1;
        let mut a_refs = [&bk.archetypes[0]; VAR_COUNT];
        let mut a_rows = [::froql::archetype::ArchetypeRow(u32::MAX); VAR_COUNT];
        let mut current_step = 0;
        let mut a_max_rows = [0; VAR_COUNT];
        let mut a_next_indexes = [usize::MAX; VAR_COUNT];
        let mut col_indexes = [usize::MAX; 2];
        ::std::iter::from_fn(move || {
            loop {
                match current_step {
                    0 => {
                        const CURRENT_VAR: usize = 0;
                        const CURRENT_VAR_COMPONENTS: ::std::ops::Range<usize> = 0..2;
                        let next_index = &mut a_next_indexes[CURRENT_VAR];
                        let archetype_ids = &archetype_id_sets[CURRENT_VAR];
                        *next_index = next_index.wrapping_add(1);
                        if *next_index >= archetype_ids.len() {
                            return None;
                        }
                        let next_id = archetype_ids[*next_index];
                        a_rows[CURRENT_VAR] = ::froql::archetype::ArchetypeRow(u32::MAX);
                        let a_ref = &mut a_refs[CURRENT_VAR];
                        *a_ref = &bk.archetypes[next_id.as_index()];
                        a_ref.find_multiple_columns(
                            &components_0,
                            &mut col_indexes[CURRENT_VAR_COMPONENTS],
                        );
                        a_max_rows[CURRENT_VAR] = a_ref.entities.len() as u32;
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
                                coerce_cast::<CompA>(
                                    world,
                                    a_refs[0].columns[col_indexes[0]].get(a_rows[0].0),
                                )
                                .borrow(),
                                (&*((&a_refs[0].columns[col_indexes[1]]).get(a_rows[0].0)
                                    as *const ::std::cell::RefCell<CompB>))
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

#[test]
fn query_fsm_trivial() {
    #[derive(Debug)]
    struct CompA(usize);
    #[derive(Debug)]
    #[allow(dead_code)]
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
                                (&*((&arch.columns[col_ids[0]]).get(row) as *const RefCell<CompA>))
                                    .borrow(),
                                (&*((&arch.columns[col_ids[1]]).get(row) as *const RefCell<CompB>))
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
