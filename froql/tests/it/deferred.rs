use froql::{
    component::{EXCLUSIVE, SYMMETRIC},
    query,
    world::World,
};

#[test]
fn scenario_marriage() {
    #[derive(Debug)]
    #[allow(unused)]
    struct Person(usize);
    enum Spouse {}

    let mut world = World::new();
    world.register_relation_flags::<Spouse>(SYMMETRIC | EXCLUSIVE);
    for i in 0..5 {
        world.create_mut().add(Person(i));
    }

    while let Some((first,)) = {
        world.process();
        query!(world, &this, _ Person, !Spouse(this, _)).next()
    } {
        if let Some((second,)) =
            query!(world, &this, _ Person, !Spouse(this, _), this != *first).next()
        {
            second.relate_to::<Spouse>(first.id);
        } else {
            break;
        }
    }
    world.process();

    let mut married_count = 0;
    for (a, b) in query!(world, Person(a), Person(b), Spouse(a, b)) {
        println!("{a:?} is married to {b:?}");
        married_count += 1;
    }
    assert_eq!(4, married_count);

    for (person,) in query!(world, Person, !Spouse(this, _)) {
        println!("{person:?} is not married");
    }
}

#[test]
fn deferred_creation_simple() {
    let mut world = World::new();
    let e = world.create_deferred().id;
    dbg!(&e);
    assert!(!world.is_alive(e));
    world.process();
    assert!(world.is_alive(e));
}

#[test]
fn deferred_creation_using_freelist() {
    let mut world = World::new();
    for _ in 0..5 {
        world.create();
    }
    let a = world.create();
    let b = world.create();
    let c = world.create();
    world.destroy(a);
    world.destroy(b);
    world.destroy(c);

    let c_new = world.create_deferred().id;
    let b_new = world.create_deferred().id;
    world.create_deferred();
    world.create_deferred();

    assert!(!world.is_alive(c));
    assert!(!world.is_alive(c_new));
    assert_eq!(c_new.id, c.id);
    assert_ne!(c_new.generation, c.generation);

    assert!(!world.is_alive(b));
    assert!(!world.is_alive(b_new));
    assert_eq!(b_new.id, b.id);
    assert_ne!(b_new.generation, b.generation);

    world.process();
    assert!(!world.is_alive(b));
    assert!(!world.is_alive(c));
    assert!(world.is_alive(b_new));
    assert!(world.is_alive(c_new));
}

#[test]
fn deferred_creation_realised_by_undeferred() {
    let mut world = World::new();
    let e = world.create_deferred().id;
    assert!(!world.is_alive(e));
    let a = world.create();
    assert!(world.is_alive(e));

    let e = world.create_deferred().id;
    assert!(!world.is_alive(e));
    world.destroy(a);
    assert!(world.is_alive(e));
}
