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


### Ignoring components in the result
If you only care that a component exists but don't care about its value you can ignore by prefixing the term with `_ `. 
Note that the space is not optional, since structs in Rust can start with an underscore.

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

### Component sources
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
    // ...
}
```



### Query for Relations

### Outvars: getting matched Entities

### Invars: setting a query variable to a fixed value

### Uncomponents: negative Component constraints

### Unrelations: negative Relation constraints

### Nested queries

### Query limitations
- outer joins
