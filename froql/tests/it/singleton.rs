use froql::{query, world::World};

struct MySingleton {
    #[allow(unused)]
    x: i32,
}

struct SomeComponent {
    #[allow(unused)]
    x: i32,
}

#[test]
fn singleton_presence() {
    let mut world = World::new();
    world.register_component::<MySingleton>();
    world.register_component::<SomeComponent>();
    world.create().add(SomeComponent { x: 123 });
    world.singleton_add(MySingleton { x: 321 });

    let mut counter = 0;

    for _ in query!(world, $ MySingleton, SomeComponent) {
        counter += 1;
    }

    assert_eq!(1, counter);
}

#[test]
fn singleton_absence() {
    let mut world = World::new();
    world.register_component::<MySingleton>();
    world.register_component::<SomeComponent>();
    world.create().add(SomeComponent { x: 123 });
    //world.singleton_add(MySingleton { x: 321 });

    let mut counter = 0;

    for _ in query!(world, $ MySingleton, SomeComponent) {
        counter += 1;
    }

    assert_eq!(0, counter);
}
