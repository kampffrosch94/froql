use froql::{
    archetype::ArchetypeId, entity_store::EntityId, relation::Relation, relation_vec::RelationVec,
    world::World,
};
use std::{any::TypeId, cell::RefCell};

#[test]
fn manual_query_relation() {
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
