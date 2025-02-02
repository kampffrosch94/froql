use froql::{
    component::{CASCADING_DESTRUCT, EXCLUSIVE, SYMMETRIC, TRANSITIVE},
    entity_store::Entity,
    world::World,
};

#[test]
fn relation_simple() {
    enum Rel {}

    let mut world = World::new();
    world.register_relation::<Rel>();
    let a = world.create();
    let b = world.create();
    assert!(!world.has_relation::<Rel>(a, b));
    world.add_relation::<Rel>(a, b);
    assert!(world.has_relation::<Rel>(a, b));
    world.remove_relation::<Rel>(a, b);
    assert!(!world.has_relation::<Rel>(a, b));

    // removing multiple times is no problem
    world.remove_relation::<Rel>(a, b);
    world.remove_relation::<Rel>(a, b);
    world.remove_relation::<Rel>(a, b);
    assert!(!world.has_relation::<Rel>(a, b));
}

#[test]
fn relation_check_one_side() {
    enum Rel {}

    let mut world = World::new();
    world.register_relation::<Rel>();
    let a = world.create();
    let b = world.create();
    world.add_relation::<Rel>(a, b);
    let targets: Vec<Entity> = world.relation_targets::<Rel>(a).collect();
    assert_eq!(&[b][..], &targets[..]);
    let origins: Vec<Entity> = world.relation_origins::<Rel>(b).collect();
    assert_eq!(&[a][..], &origins[..]);
}

#[test]
fn relation_entity_destroy() {
    enum Rel {}

    let mut world = World::new();
    world.register_relation::<Rel>();
    let a = world.create();
    let b = world.create();
    assert!(!world.has_relation::<Rel>(a, b));
    world.add_relation::<Rel>(a, b);
    assert!(world.has_relation::<Rel>(a, b));
    assert_eq!(1, world.relation_origins::<Rel>(b).count());
    world.destroy(a);
    assert_eq!(0, world.relation_origins::<Rel>(b).count());
    assert!(!world.has_relation::<Rel>(a, b));
}

#[test]
fn relation_exlusive() {
    enum Rel {}

    let mut world = World::new();
    world.register_relation_flags::<Rel>(EXCLUSIVE);
    let a = world.create();
    let b = world.create();
    let c = world.create();
    assert!(!world.has_relation::<Rel>(a, b));
    assert!(!world.has_relation::<Rel>(a, c));
    world.add_relation::<Rel>(a, b);
    assert!(world.has_relation::<Rel>(a, b));
    assert!(!world.has_relation::<Rel>(a, c));
    world.add_relation::<Rel>(a, c);
    assert!(world.has_relation::<Rel>(a, c));
    assert!(!world.has_relation::<Rel>(a, b));
}

#[test]
fn relation_asymmetric() {
    enum Rel {}
    let mut world = World::new();
    world.register_relation_flags::<Rel>(0);
    let a = world.create();
    let b = world.create();
    world.add_relation::<Rel>(a, b);
    assert!(world.has_relation::<Rel>(a, b));
    assert!(!world.has_relation::<Rel>(b, a));
}

#[test]
fn relation_symmetric() {
    enum Rel {}

    let mut world = World::new();
    world.register_relation_flags::<Rel>(SYMMETRIC);
    let a = world.create();
    let b = world.create();
    assert!(!world.has_relation::<Rel>(a, b));
    assert!(!world.has_relation::<Rel>(b, a));
    world.add_relation::<Rel>(a, b);
    assert!(world.has_relation::<Rel>(a, b));
    assert!(world.has_relation::<Rel>(b, a));
}

#[test]
fn relation_cascading() {
    enum Rel {}

    let mut world = World::new();
    world.register_relation_flags::<Rel>(CASCADING_DESTRUCT);
    let a = world.create();
    let b = world.create();
    world.add_relation::<Rel>(a, b);

    assert!(world.has_relation::<Rel>(a, b));
    assert!(world.is_alive(a));
    assert!(world.is_alive(b));

    world.destroy(a);
    assert!(!world.is_alive(a));
    assert!(!world.is_alive(b));
}

#[test]
fn relation_transitive() {
    enum Rel {}

    let mut world = World::new();
    world.register_relation_flags::<Rel>(TRANSITIVE);
    let a = world.create();
    let b = world.create();
    let c = world.create();
    let d = world.create();
    world.add_relation::<Rel>(a, b);
    world.add_relation::<Rel>(b, c);
    world.add_relation::<Rel>(c, d);
    assert!(world.has_relation::<Rel>(a, b));
    assert!(world.has_relation::<Rel>(a, c));
    assert!(world.has_relation::<Rel>(a, d));
}

#[test]
fn relation_transitive_circle() {
    enum Rel {}

    let mut world = World::new();
    world.register_relation_flags::<Rel>(TRANSITIVE);
    let a = world.create();
    let b = world.create();
    let c = world.create();
    let d = world.create();
    world.add_relation::<Rel>(a, b);
    world.add_relation::<Rel>(b, c);
    world.add_relation::<Rel>(c, d);
    world.add_relation::<Rel>(b, a);
    world.add_relation::<Rel>(c, a);
    assert!(world.has_relation::<Rel>(a, b));
    assert!(world.has_relation::<Rel>(a, c));
    assert!(world.has_relation::<Rel>(a, d));
}
