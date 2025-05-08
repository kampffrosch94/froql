# Singletons

Singletons are components that only exist once in the `World`. They are attached to the singleton `Entity`.

```rust
# use froql::world::World;
struct DeltaTime(f32);
let mut world = World::new();
// create singleton
world.singleton_add(DeltaTime(1./60.));

// access singleton
assert_eq!(world.singleton::<DeltaTime>().0, 1./60.);

// mutate singleton
world.singleton_mut::<DeltaTime>().0 = 1.;
assert_eq!(world.singleton::<DeltaTime>().0, 1.);

// remove singleton
world.singleton_remove::<DeltaTime>();
```

