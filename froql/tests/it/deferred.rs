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
        query!(world, &me, _ Person(me), !Spouse(me, _)).next()
    } {
        let first = first.id;
        if let Some((second,)) =
            query!(world, &me, _ Person(me), !Spouse(me, _), me != *first).next()
        {
            second.relate_to::<Spouse>(first);
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
