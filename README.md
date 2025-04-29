# froql

Froql is a DSL for dealing with graph-like state in Rust.

## Target use case
Froql was designed with game jams in mind.
In a game jam requirements aren't clear and time is limited. 
Lots of experimentation needs to happen but global refactoring is too costly.
State is often graph-like (hard to express in Rust) and not tree-like (easy to express in Rust).

Therefore froql allows a user to input data, define relations between data objects and then
query the data back out in whatever shape is needed at the usage side.

This dynamic behavior relaxes both the requirements and the guarantees of Rust's typesystem.

## froql has
- fast compile times
- first class relation support
- an ergonomic DSL for creating queries
- queries that double as normal rust iterators
- components that don't need to implement a trait (thus letting you use library types as is)

## froql doesn't have
- systems, observers or a scheduler
- multithreading support
- a codebase free of unsafe

## Example

TODO
A tomato is a fruit. A fruit is a food.
 

## Documentation

TODO

## Inspirations

Froql was inspired by many other projects. 
Click on the arrows to see what idea was taken from each.

<details>
  <summary> <a href="https://github.com/SanderMertens/flecs">flecs</a> </summary>
  As far as I know this is the most advanced ECS out there at the moment.
  If you need something polyglot (it's written in C with bindings for lots of language),
  fancy features or 
  
  The backing archetypical ECS of froql and it's query language were inspired by flecs.
  It's creator wrote a lot of helpful articles about ECS design and also gave me direct advice ❤️
  
  I recommend reading https://medium.com/@ajmmertens/building-an-ecs-storage-in-pictures-642b8bfd6e04
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
