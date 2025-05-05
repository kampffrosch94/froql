//! This crate is a workaround for mdbook not being able to include dependencies in tests.
use doc_comment::doc_comment;

// When running `cargo test`, rustdoc will check these files as well.
doc_comment!(include_str!("../../book/src/entities.md"));
doc_comment!(include_str!("../../book/src/index.md"));
doc_comment!(include_str!("../../book/src/queries.md"));
doc_comment!(include_str!("../../book/src/relations.md"));
doc_comment!(include_str!("../../book/src/singletons.md"));

#[test]
fn readme_test() {
    use froql::component::TRANSITIVE;
    use froql::query;
    use froql::world::World;

    struct Name(&'static str);
    enum IsA {}

    let mut world = World::new();
    world.register_component::<Name>();
    // registering the IsA relationship as being transitive
    world.register_relation_flags::<IsA>(TRANSITIVE);

    // creating entities and relating them
    let food = world.create().add(Name("Food")).entity;
    let fruit = world
        .create()
        .add(Name("Fruit"))
        .relate_to::<IsA>(food)
        .entity;
    world.create().add(Name("Tomato")).relate_to::<IsA>(fruit);
    world.create().add(Name("Bread")).relate_to::<IsA>(food);

    // querying
    for (a, b) in query!(world, Name(a), Name(b), IsA(a, b)) {
        println!("{} is a {}", a.0, b.0);
    }
}
