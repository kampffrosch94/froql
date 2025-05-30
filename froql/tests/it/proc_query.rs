use froql::component::TRANSITIVE;
use froql::entity_store::Entity;
use froql::query;
use froql::world::World;

#[test]
fn proc_query_relation() {
    enum Attack {}

    #[derive(Debug)]
    #[allow(dead_code)] // used only for debug output
    struct Unit(String);
    #[derive(Debug)]
    struct Health(isize);

    let mut world = World::new();
    world.register_relation::<Attack>();
    world.register_component::<Unit>();
    world.register_component::<Health>();

    let player = world.create_entity();
    world.add_component(player, Unit("Player".to_string()));
    let goblin_a = world.create_entity();
    world.add_component(goblin_a, Health(10));
    world.add_component(goblin_a, Unit("Goblin A".to_string()));
    world.add_relation::<Attack>(player, goblin_a);

    let goblin_b = world.create_entity();
    world.add_component(goblin_b, Health(10));
    world.add_component(goblin_b, Unit("Goblin B".to_string()));
    world.add_relation::<Attack>(player, goblin_b);

    // this should not be matched by the query below
    // bad example I know, but I need something
    let trap = world.create_entity();
    world.add_relation::<Attack>(trap, goblin_b);

    let origins_a: Vec<Entity> = world.relation_origins::<Attack>(goblin_a).collect();
    assert_eq!(&[player], origins_a.as_slice());
    let origins_b: Vec<Entity> = world.relation_origins::<Attack>(goblin_b).collect();
    assert_eq!(&[player, trap], origins_b.as_slice());

    let mut counter = 0;

    for (me, other, mut hp) in query!(world, Unit(me), Unit(other),
                                          mut Health(me), Attack(other, me))
    {
        println!("{me:?} attacked by {other:?}");
        hp.0 -= 5;
        println!("Hp now: {hp:?}");
        counter += 1;
    }
    assert_eq!(2, counter);
}

#[test]
fn proc_query_trivial() {
    #[derive(Debug)]
    struct CompA(usize);
    #[derive(Debug)]
    struct CompB(String);
    struct CompC {}

    let mut world = World::new();
    world.register_component::<CompA>();
    world.register_component::<CompB>();
    world.register_component::<CompC>();

    let a = world.create_entity();
    world.add_component(a, CompA(42));
    world.add_component(a, CompB("Hello".to_string()));
    let b = world.create_entity();
    world.add_component(b, CompA(21));
    let c = world.create_entity();
    world.add_component(c, CompA(42));
    world.add_component(c, CompB("Hello".to_string()));
    world.add_component(c, CompC {});

    let mut counter = 0;
    for (comp_a, comp_b) in query!(world, CompA, CompB) {
        println!("{comp_a:?}");
        println!("{comp_b:?}");
        assert_eq!(42, comp_a.0);
        assert_eq!("Hello", &comp_b.0);
        counter += 1;
    }
    assert_eq!(2, counter);
}

#[test]
fn proc_query_uncomponent() {
    #[derive(Debug)]
    struct CompA(usize);
    #[derive(Debug)]
    struct CompB(String);
    struct CompC {}

    let mut world = World::new();
    world.register_component::<CompA>();
    world.register_component::<CompB>();
    world.register_component::<CompC>();

    let a = world.create_entity();
    world.add_component(a, CompA(42));
    world.add_component(a, CompB("Hello".to_string()));
    let b = world.create_entity();
    world.add_component(b, CompA(42));
    world.add_component(b, CompB("Hello".to_string()));
    world.add_component(b, CompC {});
    let c = world.create_entity();
    world.add_component(c, CompA(42));
    world.add_component(c, CompB("Hello".to_string()));

    let mut counter = 0;
    for (comp_a, comp_b) in query!(world, CompA, CompB, !CompC) {
        println!("{comp_a:?}");
        println!("{comp_b:?}");
        assert_eq!(42, comp_a.0);
        assert_eq!("Hello", &comp_b.0);
        counter += 1;
    }
    assert_eq!(2, counter);
}

#[test]
fn proc_query_outvar() {
    #[derive(Debug)]
    struct CompA(usize);
    #[derive(Debug)]
    struct CompB(String);
    struct CompC {}

    let mut world = World::new();
    world.register_component::<CompA>();
    world.register_component::<CompB>();
    world.register_component::<CompC>();

    let a = world.create_entity();
    world.add_component(a, CompA(42));
    world.add_component(a, CompB("Hello".to_string()));
    let b = world.create_entity();
    world.add_component(b, CompA(21));
    let c = world.create_entity();
    world.add_component(c, CompA(42));
    world.add_component(c, CompB("Hello".to_string()));
    world.add_component(c, CompC {});

    let mut counter = 0;
    let mut c_counter = 0;
    for (this, comp_a, comp_b) in query!(world, &this, CompA, CompB) {
        println!("{comp_a:?}");
        println!("{comp_b:?}");
        assert_eq!(42, comp_a.0);
        assert_eq!("Hello", &comp_b.0);
        assert!(this.has::<CompA>());
        if this.has::<CompC>() {
            println!("I have CompC");
            c_counter += 1;
        }
        counter += 1;
    }
    assert_eq!(2, counter);
    assert_eq!(1, c_counter);
}

#[test]
fn proc_query_relation_invar() {
    enum Attack {}

    #[derive(Debug)]
    #[allow(dead_code)] // used only for debug output
    struct Unit(String);
    #[derive(Debug)]
    struct Health(isize);

    let mut world = World::new();
    world.register_component::<Unit>();
    world.register_component::<Health>();
    world.register_relation::<Attack>();

    let player = world.create_entity();
    world.add_component(player, Unit("Player".to_string()));
    let player2 = world.create_entity();
    world.add_component(player2, Unit("Player2".to_string()));
    let goblin_a = world.create_entity();
    world.add_component(goblin_a, Health(10));
    world.add_component(goblin_a, Unit("Goblin A".to_string()));
    world.add_relation::<Attack>(player, goblin_a);
    world.add_relation::<Attack>(player2, goblin_a);

    let goblin_b = world.create_entity();
    world.add_component(goblin_b, Health(10));
    world.add_component(goblin_b, Unit("Goblin B".to_string()));
    world.add_relation::<Attack>(player, goblin_b);
    world.add_relation::<Attack>(player2, goblin_b);

    // this should not be matched by the query below
    // bad example I know, but I need something
    let trap = world.create_entity();
    world.add_relation::<Attack>(trap, goblin_b);

    let origins_a: Vec<Entity> = world.relation_origins::<Attack>(goblin_a).collect();
    assert_eq!(&[player, player2], origins_a.as_slice());
    let origins_b: Vec<Entity> = world.relation_origins::<Attack>(goblin_b).collect();
    assert_eq!(&[player, player2, trap], origins_b.as_slice());

    let mut counter = 0;

    for (me, other, mut hp) in query!(world, Unit(me), Unit(player),
                                          mut Health(me), Attack(*player, me))
    {
        println!("{me:?} attacked by {other:?}");
        hp.0 -= 5;
        println!("Hp now: {hp:?}");
        counter += 1;
    }
    assert_eq!(2, counter);
}

#[test]
fn proc_query_relation_anyvar() {
    enum Attack {}

    #[derive(Debug)]
    #[allow(dead_code)] // used only for debug output
    struct Unit(String);

    let mut world = World::new();
    world.register_relation::<Attack>();
    world.register_component::<Unit>();

    let player = world.create_entity();
    world.add_component(player, Unit("Player".to_string()));

    let goblin_a = world.create_entity();
    world.add_component(goblin_a, Unit("Goblin A".to_string()));
    world.add_relation::<Attack>(player, goblin_a);

    let goblin_b = world.create_entity();
    world.add_component(goblin_b, Unit("Goblin B".to_string()));
    world.add_relation::<Attack>(player, goblin_b);

    let trap = world.create_entity();
    world.add_relation::<Attack>(trap, goblin_b);

    let mut counter = 0;

    for (me,) in query!(world, &me, Attack(_, me)) {
        println!("{me:?} is attacked by something.");
        counter += 1;
    }
    // even though there are a total of 3 attacks, we only iterate twice
    // I think that is desireable, since we only care about the attacked here
    // of which there are only two
    // we would get 3 iteration if we used a normal var instead of _
    assert_eq!(2, counter);
}

#[test]
fn proc_query_unequality_invars() {
    enum Rel {}

    let mut world = World::new();
    world.register_relation::<Rel>();

    let a = world.create_entity();
    let b = world.create_entity();
    world.add_relation::<Rel>(a, b);

    let mut counter = 0;

    for (me,) in query!(world, &a, Rel(a, b), *a != *b) {
        assert_eq!(me.entity, a);
        counter += 1;
    }
    assert_eq!(1, counter);
}

#[test]
fn proc_query_constraint() {
    enum Rel {}
    enum Rel2 {}

    let mut world = World::new();
    world.register_relation::<Rel>();
    world.register_relation::<Rel2>();

    let a = world.create_entity();
    let b = world.create_entity();
    let c = world.create_entity();
    let d = world.create_entity();
    world.add_relation::<Rel>(a, b);
    world.add_relation::<Rel2>(a, b);
    world.add_relation::<Rel>(a, c);
    world.add_relation::<Rel2>(a, d);

    let mut counter = 0;

    for (me,) in query!(world, &x, Rel(x, y), Rel2(x, y)) {
        assert_eq!(me.entity, a);
        counter += 1;
    }
    assert_eq!(1, counter);
}

#[test]
#[allow(dead_code)] // used only for debug output
fn proc_query_optional_component_mut() {
    #[derive(Debug)]
    struct CompA(usize);
    #[derive(Debug)]
    struct CompB(isize);

    let mut world = World::new();
    world.register_component::<CompA>();
    world.register_component::<CompB>();

    let a = world.create_entity();
    let b = world.create_entity();
    world.add_component(a, CompA(4));
    world.add_component(a, CompB(2));
    world.add_component(b, CompA(0));

    let mut counter = 0;

    for (ca, mut cb) in query!(world, CompA, mut CompB?) {
        if let Some(ref mut cb) = cb {
            cb.0 += 5;
            counter += 1;
        }
        println!("{ca:?}");
        println!("{cb:?}");
    }

    assert_eq!(1, counter);

    // same thing, but with explicit var name
    let mut counter = 0;
    for (ca, mut cb) in query!(world, CompA, mut CompB(this)?) {
        if let Some(ref mut cb) = cb {
            cb.0 += 5;
            counter += 1;
        }
        println!("{ca:?}");
        println!("{cb:?}");
    }

    assert_eq!(1, counter);
}

#[test]
#[allow(dead_code)] // used only for debug output
fn proc_query_optional_component() {
    #[derive(Debug)]
    struct CompA(usize);
    #[derive(Debug)]
    struct CompB(isize);

    let mut world = World::new();
    world.register_component::<CompA>();
    world.register_component::<CompB>();

    let a = world.create_entity();
    let b = world.create_entity();
    world.add_component(a, CompA(4));
    world.add_component(a, CompB(2));
    world.add_component(b, CompA(0));

    let mut counter = 0;

    for (ca, cb) in query!(world, CompA, CompB?) {
        println!("{ca:?}");
        println!("{cb:?}");
        counter += 1;
    }

    assert_eq!(2, counter);
}

#[test]
#[allow(dead_code)] // used only for debug output
fn proc_query_optional_component_relation() {
    #[derive(Debug)]
    struct CompA(usize);
    enum Rel {}

    let mut world = World::new();
    world.register_component::<CompA>();
    world.register_relation::<Rel>();

    let a = world.create_entity();
    let b = world.create_entity();
    let c = world.create_entity();
    world.add_component(a, CompA(4));
    world.add_component(c, CompA(2));
    world.add_relation::<Rel>(a, b);
    world.add_relation::<Rel>(a, c);

    let mut counter = 0;

    for (ca, cb) in query!(world, CompA(a)?, CompA(b)?, Rel(a, b)) {
        println!("{ca:?}");
        println!("{cb:?}");
        counter += 1;
    }

    assert_eq!(2, counter);
}

#[test]
#[allow(dead_code)] // used only for debug output
fn proc_query_optional_component_invar() {
    #[derive(Debug)]
    struct CompA(usize);
    enum Rel {}

    let mut world = World::new();
    world.register_component::<CompA>();
    world.register_relation::<Rel>();

    let a = world.create_entity();
    let b = world.create_entity();
    let c = world.create_entity();
    world.add_component(a, CompA(4));
    world.add_component(c, CompA(2));
    world.add_relation::<Rel>(a, b);
    world.add_relation::<Rel>(a, c);

    let mut counter = 0;

    for (ca, cb) in query!(world, CompA(a)?, CompA(b)?, Rel(*a, *b)) {
        println!("{ca:?}");
        println!("{cb:?}");
        counter += 1;
    }

    assert_eq!(1, counter);
}

#[test]
fn proc_query_relation_simple() {
    enum Rel {}
    let mut world = World::new();
    world.register_relation::<Rel>();
    let a = world.create_entity();
    let b = world.create_entity();
    world.add_relation::<Rel>(a, b);

    let mut counter = 0;
    for (_a,) in query!(world, &a, Rel(a, b)) {
        counter += 1;
    }
    assert_eq!(1, counter);
}

#[test]
fn proc_query_relation_constraint_simple() {
    enum Rel {}
    enum Rel2 {}
    let mut world = World::new();
    world.register_relation::<Rel>();
    world.register_relation::<Rel2>();
    let a = world.create_entity();
    let b = world.create_entity();
    world.add_relation::<Rel>(a, b);
    world.add_relation::<Rel2>(a, b);

    let mut counter = 0;
    for (_a,) in query!(world, &a, Rel(a, b), Rel2(a, b)) {
        counter += 1;
    }
    assert_eq!(1, counter);
}

#[test]
fn proc_query_relation_constraint_invar() {
    enum Rel {}
    enum Rel2 {}
    let mut world = World::new();
    world.register_relation::<Rel>();
    world.register_relation::<Rel2>();
    let a = world.create_entity();
    let b = world.create_entity();
    world.add_relation::<Rel>(a, b);
    world.add_relation::<Rel2>(a, b);

    let mut counter = 0;
    for (_a,) in query!(world, &a, Rel(*a, *b), Rel2(a, b)) {
        counter += 1;
    }
    assert_eq!(1, counter);
}

#[test]
fn proc_query_relation_transitive() {
    enum Rel {}
    let mut world = World::new();
    world.register_relation_flags::<Rel>(TRANSITIVE);
    let a = world.create_entity();
    let b = world.create_entity();
    let c = world.create_entity();
    let d = world.create_entity();
    world.add_relation::<Rel>(a, b);
    world.add_relation::<Rel>(b, c);
    world.add_relation::<Rel>(c, d);
    world.add_relation::<Rel>(b, a); // circles are ok
    world.add_relation::<Rel>(c, a);

    let mut counter = 0;
    for (b,) in query!(world, &b, Rel(*a, b)) {
        println!("{b:?}");
        counter += 1;
    }
    assert_eq!(4, counter);
}

#[test]
fn proc_query_relation_transitive_backwards() {
    enum Rel {}
    let mut world = World::new();
    world.register_relation_flags::<Rel>(TRANSITIVE);
    let a = world.create_entity();
    let b = world.create_entity();
    let c = world.create_entity();
    let d = world.create_entity();
    world.add_relation::<Rel>(a, b);
    world.add_relation::<Rel>(b, c);
    world.add_relation::<Rel>(c, d);
    world.add_relation::<Rel>(b, a); // circles are ok
    world.add_relation::<Rel>(c, a);

    let mut counter = 0;
    for (a,) in query!(world, &a, Rel(a, *b)) {
        println!("{a:?}");
        counter += 1;
    }
    assert_eq!(3, counter);
}

#[test]
fn proc_query_unrelation_anyvars() {
    enum Rel {}
    #[allow(unused)]
    struct Comp(usize);
    let mut world = World::new();
    world.register_relation::<Rel>();
    world.register_component::<Comp>();

    let a = world.create_entity();
    let b = world.create_entity();
    let c = world.create_entity();
    world.add_relation::<Rel>(a, c);
    world.add_component(a, Comp(0));
    world.add_component(b, Comp(1));
    world.add_component(c, Comp(2));

    let mut counter = 0;
    for (x,) in query!(world, &x, _ Comp(x), !Rel(x, _)) {
        println!("{x:?}");
        counter += 1;
    }
    assert_eq!(2, counter);
    println!("----");
    let mut counter = 0;
    for (x,) in query!(world, &x, _ Comp(x), !Rel(_, x)) {
        println!("{x:?}");
        counter += 1;
    }
    assert_eq!(2, counter);
}

#[test]
fn proc_query_unrelation() {
    enum Rel {}
    enum Rel2 {}

    let mut world = World::new();
    world.register_relation::<Rel>();
    world.register_relation::<Rel2>();

    let a = world.create_entity();
    let b = world.create_entity();
    let c = world.create_entity();
    world.add_relation::<Rel>(a, b);
    world.add_relation::<Rel>(a, c);
    world.add_relation::<Rel2>(a, c);

    let mut counter = 0;
    for (x,) in query!(world, &x, Rel(x, y), !Rel2(x, y)) {
        println!("{x:?}");
        counter += 1;
    }
    assert_eq!(counter, 1);
}

#[test]
fn proc_query_unrelation_invar() {
    enum Rel {}
    enum Rel2 {}

    let mut world = World::new();
    world.register_relation::<Rel>();
    world.register_relation::<Rel2>();

    let a = world.create_entity();
    let b = world.create_entity();
    let c = world.create_entity();
    world.add_relation::<Rel>(a, b);
    world.add_relation::<Rel>(a, c);
    world.add_relation::<Rel2>(a, c);

    let mut counter = 0;
    for (x,) in query!(world, &a, Rel(*a, b), !Rel2(a, *b)) {
        println!("{x:?}");
        counter += 1;
    }
    assert_eq!(counter, 1);
}

#[test]
fn proc_query_relation_multihop() {
    enum Inside {}

    let mut world = World::new();
    world.register_relation::<Inside>();

    let container1 = world.create_entity();
    let container2 = world.create_entity();
    let a = world.create().relate_to::<Inside>(container1).entity;
    let b = world.create().relate_to::<Inside>(container1).entity;
    let _c = world.create().relate_to::<Inside>(container2).entity;

    let mut counter = 0;
    // find all entites that are inside the same container as a
    for (x,) in query!(world, &this, Inside(this, container), Inside(*a, container)) {
        println!("{x:?}");
        assert!(*x == a || *x == b);
        counter += 1;
    }
    assert_eq!(counter, 2);
}

#[test]
fn proc_query_invar_wrong_components() {
    enum Rel {}
    #[allow(unused)]
    struct Comp(usize);

    let mut world = World::new();
    world.register_relation::<Rel>();
    world.register_component::<Comp>();
    let a = world.create_entity();
    let _b = world.create_entity();

    let mut counter = 0;
    for _x in query!(world, &x, Comp(a), Rel(x, *a)) {
        counter += 1;
    }
    assert_eq!(counter, 0);
}

#[test]
fn proc_query_singleton() {
    struct Value(usize);
    struct Accum(usize);

    let mut world = World::new();
    world.register_component::<Accum>();
    world.register_component::<Value>();

    world.singleton_add(Accum(0));
    world.create().add(Value(1));
    world.create().add(Value(2));
    world.create().add(Value(3));

    let mut counter = 0;
    for (_acc, _val) in query!(world, $Accum, Value) {
        counter += 1;
    }
    assert_eq!(counter, 3);

    let mut counter = 0;
    for (mut acc, val) in query!(world, mut $ Accum, Value) {
        acc.0 += val.0;
        counter += 1;
    }
    assert_eq!(6, world.singleton::<Accum>().0);
    assert_eq!(counter, 3);
}
