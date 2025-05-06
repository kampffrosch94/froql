use froql::world::World;

#[test]
#[allow(dead_code)]
#[cfg(not(miri))] // can't mix miri and insta
fn debug_entity_view_mut() {
    #[derive(Debug)]
    struct Unit(String);
    #[derive(Debug)]
    struct Health(isize);
    enum Rel {}

    let mut world = World::new();
    world.register_component::<Unit>();
    world.register_component::<Health>();
    world.register_relation::<Rel>();
    world.register_debug::<Unit>();
    world.register_debug::<Health>();

    let a = world.create_entity();

    let e = world
        .create()
        .add(Unit("Goblin".into()))
        .add(Health(10))
        .relate_to::<Rel>(a);

    insta::assert_debug_snapshot!(e, @r#"
    EntityViewMut {
        id: EntityId(
            2,
        ),
        generation: EntityGeneration(
            1,
        ),
        component: Unit(
            "Goblin",
        ),
        component: Health(
            10,
        ),
        component: "froql::relation::Relation<it::entity_view::debug_entity_view_mut::Rel>",
    }
    "#);
}
