# Relations

A Relation is always between two entities.

You can think of Entities being nodes on directed graph and with relationships being the edges.

Relations are distinguished with a rust type via its `TypeId`.
To prevent accidentally adding a Relation as a Component it is recommended to use uninhabited types for them. 
For example an enum with no variants.

A Relation always has an origin and a target.

## Registration

Like for components, it is recommended to register relations before use.

```rust
# use froql::world::World;
enum MyRelation {}
fn create_world() -> World {
    let mut world = World::new();
    world.register_relation::<MyRelation>();
    world
}
```

## Adding and removing relations between entities

```rust
# use froql::world::World;
# enum MyRelation {}
# let mut world = World::new();
# world.register_relation::<MyRelation>();
let a = world.create_entity();
let b = world.create_entity();

world.add_relation::<MyRelation>(a,b);
assert!(world.has_relation::<MyRelation>(a,b));

world.remove_relation::<MyRelation>(a,b);
assert!(!world.has_relation::<MyRelation>(a,b));
```

In the EntityView the vocubalary is `relate` and `unrelate`.

```rust
# use froql::world::World;
# enum MyRelation {}
# let mut world = World::new();
# world.register_relation::<MyRelation>();
let b = world.create_entity();
let a = world.create().relate_to::<MyRelation>(b).entity;

assert!(world.has_relation::<MyRelation>(a,b));

world.view_mut(a).unrelate_to::<MyRelation>(b);
assert!(!world.has_relation::<MyRelation>(a,b));
```


## Relation Flags


### Cascading deletion

when `A` in `A->B` gets destroyed, B also gets destroyed

#### Example
- temporary effect that gets deleted -> Timer with Relation
- could also work for temporary items

### Transitive Relations

`A -> B -> C` implies `A->B`

### Symmetric Relations

`A -> B` implies `B->A`

### Symmetric Relations

`A -> B` implies there is no `A->C`
