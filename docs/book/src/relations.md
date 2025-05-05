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

Per default relations are directed, many-to-many and non-transitive. 
But this behavior can be changed when registering the relation using flags.



### Exclusive Relations
`A -> B` implies there is no `A->C`

```rust
# use froql::world::World;
use froql::component::EXCLUSIVE;
# enum ChildOf {}
# let mut world = World::new();

world.register_relation_flags::<ChildOf>(EXCLUSIVE);
let a = world.create_entity();
let b = world.create().relate_from::<ChildOf>(a).entity;

assert!(world.has_relation::<ChildOf>(a,b));

let c = world.create().relate_from::<ChildOf>(a).entity;

// a is not in a ChildOf relation to b anymore
assert!(!world.has_relation::<ChildOf>(a,b));
assert!(world.has_relation::<ChildOf>(a,c));
```

### Transitive Relations

`A -> B -> C` implies `A->C`

```rust
# use froql::world::World;
# use froql::component::TRANSITIVE;
# enum InsideOf {}
# let mut world = World::new();
world.register_relation_flags::<InsideOf>(TRANSITIVE);

let house = world.create_entity();
let room = world.create().relate_to::<InsideOf>(house).entity;
let guy = world.create().relate_to::<InsideOf>(room).entity;

assert!(world.has_relation::<InsideOf>(guy, room));
assert!(world.has_relation::<InsideOf>(guy, house));
```

### Symmetric Relations

`A -> B` implies `B->A`

```rust
# use froql::world::World;
# use froql::component::SYMMETRIC;
# enum Friends {}
# let mut world = World::new();
world.register_relation_flags::<Friends>(SYMMETRIC);

let anna = world.create_entity();
let otto = world.create().relate_to::<Friends>(anna).entity;

assert!(world.has_relation::<Friends>(anna, otto));
assert!(world.has_relation::<Friends>(otto, anna));
```

### Cascading deletion

When `A` in `A->B` gets destroyed, `B` also gets destroyed

```rust
# use froql::world::World;
# use froql::component::CASCADING_DESTRUCT;
# enum Cleanup {}
# let mut world = World::new();
world.register_relation_flags::<Cleanup>(CASCADING_DESTRUCT);

let resource = world.create_entity();
let container = world.create().relate_to::<Cleanup>(resource).entity;
let outer_container = world.create().relate_to::<Cleanup>(container).entity;

// destruction is propagated
world.destroy(outer_container);
assert!(!world.is_alive(outer_container));
assert!(!world.is_alive(container));
assert!(!world.is_alive(resource));
```

### Multiple Flags

You can pass multiple flags when registering a relation by xoring them together.

```rust
# use froql::world::World;
# use froql::component::SYMMETRIC;
# use froql::component::EXCLUSIVE;
# enum BestFriends {}
# let mut world = World::new();
world.register_relation_flags::<BestFriends>(SYMMETRIC | EXCLUSIVE);

let mustadir = world.create_entity();
let asif = world.create_entity();
world.add_relation::<BestFriends>(asif, mustadir);

let salman = world.create_entity();

// friendship ended with mustadir, now salman is my best friend
world.add_relation::<BestFriends>(asif, salman);

assert!(!world.has_relation::<BestFriends>(asif, mustadir));
assert!(world.has_relation::<BestFriends>(salman, asif));
```


## Naming Relations

TODO

