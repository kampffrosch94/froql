# Queries

Queries in froql are proc-macros.

A `query!` always needs a reference to a `World` as first argument.

After the `World` a comma separated list of terms follows, which define the output of the query.

## Query for Components

Components can be queried by writing their type name as terms.

```rust
# use froql::world::World;
# use froql::entity_store::Entity;
use froql::query;

struct Name(&'static str);
struct Age(u32);

let world = &mut World::new();
world.register_component::<Name>();
world.register_component::<Age>();

world.create()
    .add(Name("Bob"))
    .add(Age(25));
    
world.create()
    .add(Name("Anna"))
    .add(Age(32));

# let mut check = 0;
for (name, age) in query!(world, Name, Age) {
    println!("{} is {} years old.", name.0, age.0);
    # check += age.0;
}
# assert_eq!(57, check);
```

This prints:
```txt
Bob is 25 years old.
Anna is 32 years old.
```



## Ignoring components in the result
If you only care that a component exists but don't care about its value you can ignore by prefixing the term with `_ `. 
Note that the space is not optional, since typenames in Rust can start with an underscore.

Example:
```rust
# use froql::world::World;
# use froql::entity_store::Entity;
# use froql::query;
# struct Name(&'static str);
# struct Age(u32);
struct Player{}
# let world = &mut World::new();
# world.register_component::<Name>();
# world.register_component::<Age>();
# world.register_component::<Player>();
// ...
world.create()
    .add(Name("Bob"))
    .add(Age(25))
    .add(Player{});
    
world.create()
    .add(Name("Anna"))
    .add(Age(32));

for (name, age) in query!(world, Name, Age, _ Player) {
    // ...
}
```
The query here only matches Bob, since he is the only one tagged as `Player`. 
But the result tuple is not modified.

## Component sources
Components always have a source, called a variable. 
If no variable is given `this` is used as default.
The source is specified in parenthesis after the component name.

So the query in the previous example is equivalent to:
```rust
# use froql::world::World;
# use froql::entity_store::Entity;
# use froql::query;
# struct Name(&'static str);
# struct Age(u32);
# struct Player{}
# let world = &mut World::new();
# world.register_component::<Name>();
# world.register_component::<Age>();
# world.register_component::<Player>();
# world.create()
#     .add(Name("Bob"))
#     .add(Age(25))
#     .add(Player{});
#
# world.create()
#     .add(Name("Anna"))
#     .add(Age(32));
# 
for (name, age) in query!(world, Name(this), Age(this), _ Player(this)) {
    // ... only matches Bob
}
```

## Mutating Components

Components can be mutably borrowed in a query by prefixing the term with `mut`.

So the query in the previous example is equivalent to:
```rust
# use froql::world::World;
# use froql::entity_store::Entity;
# use froql::query;
# struct Name(&'static str);
# struct Age(u32);
# let world = &mut World::new();
# world.register_component::<Name>();
# world.register_component::<Age>();
# world.create()
#     .add(Name("Bob"))
#     .add(Age(25));
#
# world.create()
#     .add(Name("Anna"))
#     .add(Age(32));
# 
# let mut check = 0;

// a year passes
for (mut age,) in query!(world, mut Age) {
    age.0 += 1;
}

for (name, age) in query!(world, Name, Age) {
    println!("{} is {} years old.", name.0, age.0);
    # check += age.0;
}
# assert_eq!(59, check);
```

Now it prints:

```txt
Bob is 26 years old.
Anna is 33 years old.
```


## Query for Relations
Relations are expressed in the form `<Type>(<variable>, <variable>)`.

```rust
# use froql::component::TRANSITIVE;
# use froql::query;
# use froql::world::World;
struct Name(&'static str);
enum IsA {}

let mut world = World::new();
world.register_relation_flags::<IsA>(TRANSITIVE);

let food = world.create().add(Name("Food")).entity;
let fruit = world.create().add(Name("Fruit")).relate_to::<IsA>(food).entity;
world.create().add(Name("Tomato")).relate_to::<IsA>(fruit);
world.create().add(Name("Bread")).relate_to::<IsA>(food);

for (a, b) in query!(world, Name(a), Name(b), IsA(a, b)) {
    println!("{} is a {}", a.0, b.0);
}

```

If you only care about an entity being a relation target or origin you can use the form `<Type>(<variable>, _)` or `<Type>(_, <variable>)`.
This will only match the entity in question once.

## Outvars: getting matched Entities

Sometimes you want to get the Entity behind a variable.
For this you can use a term of the form `&<variable>`.

```rust
# use froql::component::TRANSITIVE;
# use froql::query;
# use froql::world::World;
# struct Name(&'static str);
# enum IsA {}

# let mut world = World::new();
# world.register_relation_flags::<IsA>(TRANSITIVE);

# let food = world.create().add(Name("Food")).entity;
# let fruit = world.create().add(Name("Fruit")).relate_to::<IsA>(food).entity;
# world.create().add(Name("Tomato")).relate_to::<IsA>(fruit);
# world.create().add(Name("Bread")).relate_to::<IsA>(food);

for (entity_a, a, b) in query!(world, &a, Name(a), Name(b), IsA(a, b)) {
    dbg!(entity_a);
    println!("{} is a {}", a.0, b.0);
}
```

### Modifying entities during a query

The returned entity is wrapped in an `EntityViewDeferred`.
Like the name implies structural changes on this entity are deferred until `world.process()` is called, so as to not invalidate our iterator.

```rust
# use froql::query;
# use froql::world::World;
# struct HP(i32);

# let mut world = World::new();

let e = world.create().add(HP(-5)).entity;

for (entity, hp) in query!(world, &this, HP) {
    if hp.0 <= 0 {
        entity.destroy();
    }
}

// entity is only destroyed once world.process() is called
assert!(world.is_alive(e));

world.process();
assert!(!world.is_alive(e));
```

You can also use this to add/remove components or relationships to an entity during query iteration.

You can also spawn entities using `world.create_deferred()` and directly use them as normal.

## Invars: setting a query variable to a fixed value

It's often necessary to fix a query variable to an Entity coming from an outer scope.
A variable counts as invar if it is prefixed with a star (`*`) at least once in the query.

```rust
# use froql::query;
# use froql::world::World;
# use froql::component::SYMMETRIC;
# struct Name(&'static str);
# enum Foes {}

# let mut world = World::new();
world.register_relation_flags::<Foes>(SYMMETRIC);

let player = world.create().add(Name("Player")).entity;
let goblin = world.create().add(Name("Goblin")).relate_to::<Foes>(player).entity;
world.create().add(Name("Villager")).relate_to::<Foes>(goblin);

# let mut counter = 0;
for (name,) in query!(world, Name, Foes(this, *player)) {
    println!("{} is an enemy of the player", name.0);
# counter += 1;
}
# assert_eq!(1, counter);
```


## Unrelations: negative Relation constraints
Prefix a relation type with `!` to match entities that don't have that relation.

```rust
# use froql::query;
# use froql::world::World;
# use froql::component::SYMMETRIC;
# struct Name(&'static str);
# enum Foes {}

# let mut world = World::new();
world.register_relation_flags::<Foes>(SYMMETRIC);

let player = world.create().add(Name("Player")).entity;
let goblin = world.create().add(Name("Goblin")).relate_to::<Foes>(player).entity;
world.create().add(Name("Villager")).relate_to::<Foes>(goblin);

# let mut counter = 0;
for (name,) in query!(world, Name, ! Foes(this, *player)) {
    println!("{} is not an enemy of the player", name.0);
# counter += 1;
}
# assert_eq!(2, counter);
```

This prints:

```txt
Player is not an enemy of the player
Villager is not an enemy of the player
```


## Uncomponents: negative Component constraints
Prefix a component type with `!` to match entities that don't have that component.

```rust
# use froql::world::World;
# use froql::entity_store::Entity;
# use froql::query;
# struct Name(&'static str);
# struct Age(u32);
struct Player{}
# let world = &mut World::new();
# world.register_component::<Name>();
# world.register_component::<Age>();
# world.register_component::<Player>();
// ...
world.create()
    .add(Name("Bob"))
    .add(Age(25))
    .add(Player{});
    
world.create()
    .add(Name("Anna"))
    .add(Age(32));

# let mut counter = 0;
for (name, age) in query!(world, Name, Age, ! Player) {
    // ... only matches Anna
    assert_eq!(name.0, "Anna");
# counter += 1;
}
# assert_eq!(1, counter);
```

## Unequalities
The term `<variable_a> != <variable_b>` makes sure that the two variables don't have the same entity as value.
This is especially useful for preventing dynamic borrowing errors when mutably borrowed components.

```rust
# use froql::query;
# use froql::world::World;
# use froql::component::SYMMETRIC;
# struct Name(&'static str);
# enum Foes {}

# let mut world = World::new();
world.register_relation_flags::<Foes>(SYMMETRIC);

let player = world.create().add(Name("Player")).entity;
let goblin = world.create().add(Name("Goblin")).relate_to::<Foes>(player).entity;
world.create().add(Name("Villager")).relate_to::<Foes>(goblin);

# let mut counter = 0;
for (name,) in query!(world, Name, ! Foes(this, *player), this != player) {
    println!("{} is not an enemy of the player", name.0);
# counter += 1;
}
# assert_eq!(1, counter);
```

This prints:

```txt
Villager is not an enemy of the player
```

## Matching singletons

Singletons can be accessed through the world API. 
For convenience they also can be accessed via query, by prefixing their typename with `$`.

```rust
# use froql::query;
# use froql::world::World;
# use froql::component::SYMMETRIC;
# struct DeltaTime(f32);
# struct Animation {time_left: f32}

# let mut world = World::new();
# world.create().add(Animation {time_left: 5.});
# world.singleton_add(DeltaTime(1./60.));

# let mut counter = 0;
for (dt, mut animation, animation_e) 
        in query!(world, $ DeltaTime, mut Animation, &this) {
    animation.time_left -= dt.0;
    if animation.time_left < 0.0 {
        animation_e.destroy();
    }
# counter += 1;
}
world.process();
# assert_eq!(1, counter);
```


## Query limitations

Out joins in queries are not allowed.

So a query like `query!(world, Name(a), Name(b), a != b)` will not compile.
This limitation is put in place intentionally, so that the user does not get O(n^2) scaling on accident.

If an outerjoin is desired you can nest queries. 

## Nested queries

```rust
# use froql::query;
# use froql::world::World;
# enum Likes {}
# struct Name(&'static str);
# let mut world = World::new();
world.register_relation::<Likes>();

world.create().add(Name("Jack"));
world.create().add(Name("Paul"));
world.create().add(Name("Fred"));

for (a,) in query!(world, &this, _ Name) {
    for (b,) in query!(world, &this, _ Name, this != *a) {
        a.relate_to::<Likes>(*b);
    }
}
world.process(); // don't forget this part !

# let mut counter = 0;
for (a,b) in query!(world, Name(a), Name(b), Likes(a,b)) {
    println!("{} likes {}.", a.0, b.0);
# counter += 1;
}
# assert_eq!(6, counter);
```

Outputs:
```txt
Jack likes Paul.
Fred likes Paul.
Paul likes Jack.
Fred likes Jack.
Jack likes Fred.
Paul likes Fred.
```

## Queries are iterators

Something that may not be obvious from the examples so far is that `query!(..)` returns an iterator.
So you can use all the normal iterator methods on them.

```rust
# use froql::query;
# use froql::world::World;
# enum Likes {}
# struct Name(&'static str);
# struct Age(i32);
# let mut world = World::new();
world.create().add(Name("Jack")).add(Age(32));
world.create().add(Name("Paul")).add(Age(42));
world.create().add(Name("Fred")).add(Age(21));

let iterator = query!(world, Name, Age);

let Some(oldest) = iterator
    .max_by_key(|(_, age)| age.0)
    .map(|(name, _)| name.0)
  else { panic!() };

assert_eq!(oldest, "Paul");
```
