use std::cell::RefCell;

use froql::world::World;

#[test]
fn test1() {
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
        assert_eq!(archetype_ids.len(), 2);
        archetype_ids.into_iter().flat_map(move |aid| {
            let arch = &bk.archetypes[aid.0 as usize];
            let mut col_ids = [usize::MAX; 2];
            arch.find_multiple_columns(&components, &mut col_ids);
            (0..(&arch.columns[col_ids[0]]).len()).map(move |row| unsafe {
                (
                    (&*((&arch.columns[col_ids[0]]).get(row) as *const RefCell<CompA>)).borrow(),
                    (&*((&arch.columns[col_ids[1]]).get(row) as *const RefCell<CompB>)).borrow(),
                )
            })
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
