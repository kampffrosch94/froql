# World, Entities and Components

## The World
<blockquote><sub> <pre>ゴゴゴゴゴゴゴ -- famous mangaka</pre></sub></blockquote>

All data in froql is stored in a `World`.
Froql does not use globals behind the scenes. 
You can use multiple different `World`s without issue, if you want to.

```rust
# use froql::world::World;
let mut world = World::new();
```


## Entities

An `Entity` is just an unique identifier.
You can copy it or store it in other data structures.

```rust
# use froql::world::World;
# let mut world = World::new();
let my_entity = world.create_entity();
assert!(world.is_alive(my_entity));
world.destroy(my_entity); // destroy entity
assert!(!world.is_alive(my_entity));
let my_entity = world.create_entity();
```

Use after free and the ABA problem are solved via generation checks.

```rust
# use froql::world::World;
# let mut world = World::new();
# let my_entity = world.create_entity(); // create entity
# assert!(world.is_alive(my_entity));
# world.destroy(my_entity); // destroy entity
let new_entity = world.create_entity();
// old id is reused
assert_eq!(new_entity.id, my_entity.id);
// but the old entity is still dead - because of the generation
assert!(!world.is_alive(my_entity));
assert_ne!(new_entity.generation, my_entity.generation);
```


## Components

To associate data with an `Entity` you add to the `Entity` as a component.

A component can be any `T: 'static`, there are no traits that must be implemented.

```rust
# use froql::world::World;
# let mut world = World::new();
struct MyStruct(u32);
world.register_component::<MyStruct>();

let e = world.create_entity();
world.add_component(e, MyStruct(42)); // add data

// mutation
{
    let mut mutable_ref = world.get_component_mut::<MyStruct>(e);
    mutable_ref.0 += 1;
}

// immutable reference
{
    let imm_ref = world.get_component::<MyStruct>(e);
    assert_eq!(43, imm_ref.0);
}

// remove (and drop) component
world.remove_component::<MyStruct>(e);
assert!(!world.has_component::<MyStruct>(e));
```

Components in froql use interior mutability via `RefCell`.
This allows for finegrained access, but may panic at runtime on misuse (violating the aliasing xor mutation rule).

### Registering components

Froql needs to know about what types of components it manages.
Before a component can be used, it therefore must be registered.

Registration happens automatically when adding a component to an entity.
But it does not happen with methods that borrow `World` non mutably, they panic instead
when they encounter an unregistered component.

The autoregistration exists for prototyping purposes and can be disabled by enabling the feature flag `manual_registration`.
For larger projects it is recommended to register everything upfront.

```rust
# use froql::world::World;
# struct MyStruct(u32);
fn create_world() -> World {
    let mut world = World::new();
    world.register_component::<MyStruct>();
    world
}
```


## Mutable Entity Views

`EntityViewMut` is a helper struct to reduce boilerplate when mutating entities. 
It can be used similarly to a builder.

```rust 
# use froql::world::World;
# use froql::entity_store::Entity;
# struct MyStruct(u32);
# struct Name(&'static str);
# struct Age(u32);
# let mut world = World::new();
# world.register_component::<MyStruct>();
# world.register_component::<Name>();
# world.register_component::<Age>();

let e: Entity = world.create()
    .add(MyStruct(42))
    .add(Name("Bob"))
    .add(Age(25))
    .entity;
```
