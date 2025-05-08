![image](https://raw.githubusercontent.com/gist/kampffrosch94/96566ce42758964cc6862971c60a8f7f/raw/230a84cd5b04ec6d77ca0edf39f835ed80a63d97/diagram.svg)

# froql

Froql is a proc_macro based DSL for dealing with graph-like state in Rust.

## Target use case
Froql was designed with game jams in mind.
In a game jam requirements aren't clear and time is limited. 
Lots of experimentation needs to happen but large scale refactoring is too costly.
State is often graph-like (hard to express in Rust) and not tree-like (easy to express in Rust).

Froql allows an user to input data, define relations between data objects and then
query the data back out in whatever shape is needed at the usage site.

This dynamic behavior relaxes both the requirements and the guarantees of Rust's typesystem.

## froql has
- Fast compile times
- First class relation support
- An ergonomic DSL for creating queries
- Queries that double as normal rust iterators
- Components that don't need to implement a trait (thus letting you use library types as is)

## froql doesn't have
- Systems, observers or a scheduler
- Multithreading support
- A codebase free of unsafe

## Example

Let's name some facts that can be expressed as relations:

A tomato is a fruit. A fruit is food. Bread is food.

These truths can be expressed and evaluated in froql like this:
```rust
struct Name(&'static str);
enum IsA {}

let mut world = World::new();
// registering the IsA relationship as being transitive
world.register_relation_flags::<IsA>(TRANSITIVE);

// creating entities and relating them
let food = world.create().add(Name("Food")).entity;
let fruit = world.create().add(Name("Fruit")).relate_to::<IsA>(food).entity;
world.create().add(Name("Tomato")).relate_to::<IsA>(fruit);
world.create().add(Name("Bread")).relate_to::<IsA>(food);

// querying
for (a, b) in query!(world, Name(a), Name(b), IsA(a, b)) {
    println!("{} is a {}", a.0, b.0);
}
```

Output:

```txt
Fruit is a Food
Bread is a Food
Tomato is a Food
Tomato is a Fruit
```

To learn how this works check out the [book](https://kampffrosch94.github.io/froql/).

You can also find more elaborate examples in the `examples/` folder.

## Plans for 1.0

I am happy with the current feature set and APIs. 
Next up is smoothing out rough edges and improving the developer experience a bit.

Then improving performance (both runtime and compile time) which first up needs some proper
benchmarks and more elaborate (fuzzy) testing.

Once I am happy with that I'll release 1.0 and then consider adding fancier queries, like
branches, components as entities, aggregate functions and straight up injecting user
supplied code.

## Inspirations

Froql was inspired by many other projects. 
Click on the arrows to see what idea was taken from each.

<details>
  <summary> <a href="https://github.com/SanderMertens/flecs">flecs</a> </summary>
  As far as I know this is the most advanced ECS out there at the moment.
  If you need something polyglot (it's written in C with bindings for lots of language),
  fancy features or state of the art performance, flecs is what I would recommend.
  
  The backing archetypical ECS of froql and its query language were inspired by flecs.
  Its creator wrote a lot of helpful articles about ECS design and also gave me direct advice ❤️
  
  Start by reading https://medium.com/@ajmmertens/building-an-ecs-storage-in-pictures-642b8bfd6e04
  if you are curious.
</details>
<details>
  <summary><a href="https://github.com/Indra-db/Flecs-Rust">flecs-rust</a></summary>
  The idea for EntityViews came from here.
</details>
<details>
  <summary><a href="https://github.com/s-arash/ascent">ascent</a></summary>
  Transpiling a query language to Rust.
  
  How ascent can interact with Rust by calling regular Rust functions is really cool.
  I want to explore that idea more for advanced queries.
</details>
<details>
  <summary><a href="https://github.com/not-fl3/nanoserde">nanoserde</a></summary>
  This is the fastest compiling proc macro crate for serialization I know.
  So copying from that I wrote froql's proc macro without any external dependencies.
</details>
<details>
  <summary><a href="https://github.com/bevyengine/bevy">bevy_ecs</a></summary>
  Bevy has lots of interesting ideas and I ignored most of them.
  Froql has a pretty different approach after all.
  
  But how bevy reserves entity IDs safely in deferred contexts is something I copied.
</details>


## License

Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
