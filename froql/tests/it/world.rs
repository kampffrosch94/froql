use froql::world::World;

#[test]
fn create_and_take() {
    #[derive(Debug, PartialEq)]
    struct Pos(i32, i32);
    let mut world = World::new();
    world.register_component::<Pos>();

    let e = world.create().add(Pos(1, 2)).entity;
    let comp = world.take_component::<Pos>(e);

    assert_eq!(comp, Some(Pos(1, 2)));
    let comp = world.take_component::<Pos>(e);
    assert_eq!(comp, None);
    let comp = world.take_component::<Pos>(e);
    assert_eq!(comp, None);
}
