use froql::{entity_view_deferred::EntityViewDeferred, world::World};

#[test]
#[allow(dead_code)]
fn debug_entity_view_mut() {
    #[derive(Debug)]
    struct Unit(String);
    #[derive(Debug)]
    struct Health(isize);
    struct Other(isize);
    enum Rel {}

    let mut world = World::new();
    world.register_component::<Unit>();
    world.register_component::<Health>();
    world.register_component::<Other>();
    world.register_relation::<Rel>();
    world.register_debug::<Unit>();
    world.register_debug::<Health>();

    let a = world.create_entity();

    let e = world
        .create()
        .add(Unit("Goblin".into()))
        .add(Health(10))
        .add(Other(10))
        .relate_to::<Rel>(a);

    if !cfg!(miri) {
        insta::assert_debug_snapshot!(e, @r#"
        EntityViewMut {
            id: EntityId(
                2,
            ),
            generation: EntityGeneration(
                1,
            ),
            Rel<origin> to: [
                1,
            ],
            components: [
                Unit(
                    "Goblin",
                ),
                Health(
                    10,
                ),
                Other { .. },
            ],
        }
        "#);
    } else {
        dbg!(e);
    }
}

#[test]
#[allow(dead_code)]
fn debug_entity_view() {
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
        .relate_to::<Rel>(a)
        .entity;
    let e = EntityViewDeferred::new(&world, e);

    if !cfg!(miri) {
        insta::assert_debug_snapshot!(e, @r#"
        EntityViewDeferred {
            id: EntityId(
                2,
            ),
            generation: EntityGeneration(
                1,
            ),
            Rel<origin> to: [
                1,
            ],
            components: [
                Unit(
                    "Goblin",
                ),
                Health(
                    10,
                ),
            ],
        }
        "#);
    } else {
        dbg!(e);
    }
}
