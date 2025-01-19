use froql::{
    archetype::{ArchetypeId, ArchetypeRow},
    entity_store::{Entity, EntityId},
    relation::Relation,
    relation_vec::RelationVec,
    world::World,
};
use std::{any::TypeId, cell::RefCell, ops::Range};

#[test]
fn relation_flatmap() {
    enum Attack {}

    #[derive(Debug)]
    #[allow(dead_code)]
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

    let origins_a: Vec<_> = world.relation_origins::<Attack>(goblin_a).collect();
    assert_eq!(&[player], origins_a.as_slice());
    let origins_b: Vec<_> = world.relation_origins::<Attack>(goblin_b).collect();
    assert_eq!(&[player, trap], origins_b.as_slice());

    let mut counter = 0;

    // manual query for:
    // query!(world, Unit(me), Unit(other), Hp(me), Attack(other, me))
    for (me, other, mut hp) in {
        let world: &World = &world;
        let bk = &world.bookkeeping;
        let components_me = [
            world.get_component_id::<Unit>(),
            world.get_component_id::<Health>(),
            bk.get_component_id_unchecked(TypeId::of::<Relation<Attack>>())
                .flip_target(),
        ];
        let components_other = [
            world.get_component_id::<Unit>(),
            bk.get_component_id_unchecked(TypeId::of::<Relation<Attack>>()),
        ];
        let archetype_ids_me = bk.matching_archetypes(&components_me, &[]);
        let archetype_ids_other = bk.matching_archetypes(&components_other, &[]);

        assert_eq!(1, archetype_ids_me.len());
        assert_eq!(1, archetype_ids_other.len());

        archetype_ids_me.into_iter().flat_map(move |aid| {
            let arch_me = &bk.archetypes[aid.0 as usize];
            let mut col_ids_me = [usize::MAX; 3];
            arch_me.find_multiple_columns(&components_me, &mut col_ids_me);
            // need to clone before moving
            let archetype_ids_other = archetype_ids_other.clone();
            (0..(&arch_me.columns[col_ids_me[0]]).len()).flat_map(move |row_me| unsafe {
                let rel_attack =
                    &*((&arch_me.columns[col_ids_me[2]]).get(row_me) as *const RelationVec);
                assert!(
                    rel_attack.len() > 0,
                    "Entity should not be in archetype if it has no relation"
                );
                // need to clone before moving - Again :/
                let archetype_ids_other = archetype_ids_other.clone();
                rel_attack
                    .iter()
                    .map(|id_raw| {
                        let id = EntityId(*id_raw);
                        bk.entities.get_archetype_unchecked(id).0
                    })
                    .filter(move |id: &ArchetypeId| archetype_ids_other.contains(id))
                    .flat_map(move |other_id| {
                        let arch_other = &bk.archetypes[other_id.0 as usize];
                        let mut col_ids_other = [usize::MAX; 1];
                        // don't actually need the relations col here, so can slice it off
                        arch_other
                            .find_multiple_columns(&components_other[0..1], &mut col_ids_other);
                        (0..(&arch_other.columns[col_ids_other[0]]).len()).map(move |row_other| {
                            (
                                (&*((&arch_me.columns[col_ids_me[0]]).get(row_me)
                                    as *const RefCell<Unit>))
                                    .borrow(),
                                (&*((&arch_other.columns[col_ids_other[0]]).get(row_other)
                                    as *const RefCell<Unit>))
                                    .borrow(),
                                (&*((&arch_me.columns[col_ids_me[1]]).get(row_me)
                                    as *const RefCell<Health>))
                                    .borrow_mut(),
                            )
                        })
                    })
            })
        })
    } {
        println!("{me:?} attacked by {other:?}");
        hp.0 -= 5;
        println!("Hp now: {hp:?}");
        counter += 1;
    }
    assert_eq!(2, counter);
}

#[test]
fn query_fsm_relation_outvar() {
    enum Attack {}

    #[derive(Debug)]
    #[allow(dead_code)]
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
    // query!(world, &me, Unit(me), Unit(other), Hp(me), Attack(other, me))
    for (_me, unit_me, unit_other, mut hp) in {
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
                        current_step -= 1;
                        return Some(unsafe {
                            (
                                froql::entity_view_deferred::EntityViewDeferred::from_id_unchecked(
                                    world,
                                    a_refs[0].entities[a_rows[0].0 as usize],
                                ),
                                (&*((&a_refs[0].columns[col_indexes[0]]).get(a_rows[0].0)
                                    as *const RefCell<Unit>))
                                    .borrow(),
                                (&*((&a_refs[1].columns[col_indexes[3]]).get(a_rows[1].0)
                                    as *const RefCell<Unit>))
                                    .borrow(),
                                (&*((&a_refs[0].columns[col_indexes[1]]).get(a_rows[0].0)
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
        println!("{unit_me:?} attacked by {unit_other:?}");
        hp.0 -= 5;
        println!("Hp now: {hp:?}");
        counter += 1;
    }
    //assert_eq!(2, counter);
    assert_eq!(2, counter);
}

#[test]
fn query_fsm_relation_invar() {
    enum Attack {}

    #[derive(Debug)]
    #[allow(dead_code)]
    struct Unit(String);
    #[derive(Debug)]
    struct Health(isize);

    let mut world = World::new();

    // this shall be our invar
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
    // query!(world, &me, Unit(me), Unit(*player), Hp(me), Attack(*player, me))
    for (unit_me, unit_other, mut hp) in {
        let invar_other: Entity = player;
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
        let mut col_indexes = [usize::MAX; 5];
        assert_eq!(
            col_indexes.len(),
            components_me.len() + components_other.len()
        );
        // gets rolled over to 0 by wrapping_add
        let mut rel_index_1 = 0;

        // set archetype for invar
        // then fill out component IDs for archetype of invar
        {
            let (aid, arow) = bk.entities.get_archetype(invar_other);
            let a_ref = &mut a_refs[1];
            *a_ref = &bk.archetypes[aid.as_index()];
            a_ref.find_multiple_columns(&components_other, &mut col_indexes[3..5]);
            a_rows[1] = arow;
        }

        let mut current_step = 1; // start in step 1, cause 0 ist just return
        std::iter::from_fn(move || {
            loop {
                match current_step {
                    // we start at 1, when we get here we are done
                    0 => {
                        return None;
                    }
                    // follow relation
                    1 => {
                        const CURRENT_VAR: usize = 1;
                        const REL_VAR: usize = 0;
                        const RELATION_COMP_INDEX: usize = 4;
                        const REL_VAR_COMPONENTS: Range<usize> = 0..3;
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
                        if rel_index_1 >= rel_vec.len() {
                            rel_index_1 = 0;
                            current_step -= 1;
                        } else {
                            // get aid/row for entity in relation
                            let id = EntityId(rel_vec[rel_index_1 as usize]);
                            let (aid, arow) = bk.entities.get_archetype_unchecked(id);
                            rel_index_1 += 1;

                            // if in target archetype set => go to next step
                            if archetype_id_sets[REL_VAR].contains(&aid) {
                                let a_ref = &mut a_refs[REL_VAR];
                                *a_ref = &bk.archetypes[aid.as_index()];
                                a_ref.find_multiple_columns(
                                    &components_me,
                                    &mut col_indexes[REL_VAR_COMPONENTS],
                                );
                                a_rows[REL_VAR] = arow;

                                current_step += 1;
                            }
                        }
                    }
                    // yield row
                    2 => {
                        current_step -= 1;
                        return Some(unsafe {
                            (
                                (&*((&a_refs[0].columns[col_indexes[0]]).get(a_rows[0].0)
                                    as *const RefCell<Unit>))
                                    .borrow(),
                                (&*((&a_refs[1].columns[col_indexes[3]]).get(a_rows[1].0)
                                    as *const RefCell<Unit>))
                                    .borrow(),
                                (&*((&a_refs[0].columns[col_indexes[1]]).get(a_rows[0].0)
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
        println!("{unit_me:?} attacked by {unit_other:?}");
        hp.0 -= 5;
        println!("Hp now: {hp:?}");
        counter += 1;
    }
    //assert_eq!(2, counter);
    assert_eq!(2, counter);
}

#[test]
fn query_fsm_relation_unequal() {
    enum Attack {}

    #[derive(Debug)]
    #[allow(dead_code)]
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
    // query!(world, &me, Unit(me), Unit(other), Hp(me), Attack(other, me), other != me)
    for (_me, unit_me, unit_other, mut hp) in {
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

                                // check unequal
                                if ::std::ptr::eq(a_refs[CURRENT_VAR], a_refs[REL_VAR])
                                    && a_rows[CURRENT_VAR] == a_rows[REL_VAR]
                                {
                                    current_step -= 1;
                                } else {
                                    current_step += 1;
                                }
                            }
                        }
                    }
                    // yield row
                    3 => {
                        current_step -= 1;
                        return Some(unsafe {
                            (
                                froql::entity_view_deferred::EntityViewDeferred::from_id_unchecked(
                                    world,
                                    a_refs[0].entities[a_rows[0].0 as usize],
                                ),
                                (&*((&a_refs[0].columns[col_indexes[0]]).get(a_rows[0].0)
                                    as *const RefCell<Unit>))
                                    .borrow(),
                                (&*((&a_refs[1].columns[col_indexes[3]]).get(a_rows[1].0)
                                    as *const RefCell<Unit>))
                                    .borrow(),
                                (&*((&a_refs[0].columns[col_indexes[1]]).get(a_rows[0].0)
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
        println!("{unit_me:?} attacked by {unit_other:?}");
        hp.0 -= 5;
        println!("Hp now: {hp:?}");
        counter += 1;
    }
    assert_eq!(2, counter);
}

#[test]
fn query_fsm_relation_constraint() {
    enum Attack {}
    // only player and goblin A are enemies
    enum Enemy {}

    #[derive(Debug)]
    #[allow(dead_code)]
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
    // TODO make symmetric relation
    world.add_relation::<Enemy>(player, goblin_a);

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
    // query!(world, &me, Unit(me), Unit(other), Hp(me), Attack(other, me), Enemy(other, me))
    for (_me, unit_me, unit_other, mut hp) in {
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
            // 3
            bk.get_component_id_unchecked(TypeId::of::<Relation<Enemy>>())
                .flip_target(),
        ];
        let components_other = [
            // 3
            world.get_component_id::<Unit>(),
            // 4
            bk.get_component_id_unchecked(TypeId::of::<Relation<Attack>>()),
            // 5
            bk.get_component_id_unchecked(TypeId::of::<Relation<Enemy>>()),
        ];
        let archetype_ids_me = bk.matching_archetypes(&components_me, &[]);
        let archetype_ids_other = bk.matching_archetypes(&components_other, &[]);
        let archetype_id_sets = [archetype_ids_me, archetype_ids_other];

        // result set
        const VAR_COUNT: usize = 2;
        let mut a_refs = [&bk.archetypes[0]; VAR_COUNT];
        let mut a_rows = [ArchetypeRow(u32::MAX); VAR_COUNT];

        // context for statemachine
        let mut a_max_rows = [0; VAR_COUNT];
        let mut col_indexes = [usize::MAX; 7];
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
                        const CURRENT_VAR_COMPONENTS: Range<usize> = 0..4;
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
                        const RELATION_COMP_INDEX: usize = 2; // Attack
                        const REL_VAR_COMPONENTS: Range<usize> = 4..7;
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

                                // check relation constraint
                                if {
                                    // Enemy
                                    const CHECK_REL_INDEX: usize = 5;
                                    // we check as seen from the new var just joined
                                    const CHECK_VAR: usize = 0;
                                    let arch = &a_refs[REL_VAR];
                                    let col = col_indexes[CHECK_REL_INDEX];
                                    let rel_vec = unsafe {
                                        &*(arch.columns[col].get(row) as *const RelationVec)
                                    };
                                    let check_ref = a_refs[CHECK_VAR];
                                    let to_check = check_ref.entities[a_rows[CHECK_VAR].0 as usize];
                                    !rel_vec.contains(&to_check.0)
                                } {
                                    current_step -= 1;
                                } else {
                                    current_step += 1;
                                }
                            }
                        }
                    }
                    // yield row
                    3 => {
                        current_step -= 1;
                        return Some(unsafe {
                            (
                                froql::entity_view_deferred::EntityViewDeferred::from_id_unchecked(
                                    world,
                                    a_refs[0].entities[a_rows[0].0 as usize],
                                ),
                                (&*((&a_refs[0].columns[col_indexes[0]]).get(a_rows[0].0)
                                    as *const RefCell<Unit>))
                                    .borrow(),
                                (&*((&a_refs[1].columns[col_indexes[4]]).get(a_rows[1].0)
                                    as *const RefCell<Unit>))
                                    .borrow(),
                                (&*((&a_refs[0].columns[col_indexes[1]]).get(a_rows[0].0)
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
        println!("{unit_me:?} attacked by {unit_other:?}");
        hp.0 -= 5;
        println!("Hp now: {hp:?}");
        counter += 1;
    }
    assert_eq!(1, counter);
}
