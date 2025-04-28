# froql

Kampf**fro**sch's **q**uery **l**anguage

## froql has
- fast compile times
- first class relation support
- an ergonomic DSL for creating queries
- queries that double as normal rust iterators
- components that don't need to implement a trait

## froql doesn't have
- systems
- observers
- a scheduler
- multithreading support
- a codebase free of unsafe


## Inspirations

Froql was inspired by many other projects. 
Click on the arrow to see what idea was taken from them.

<details>
  <summary> <a href="https://github.com/SanderMertens/flecs">flecs</a> </summary>
  As far as I know this is the most advanced ECS out there at the moment.
  If you need something poliglot (it's written in C with bindings for lots of language),
  fancy features or 
  
  The backing archetypical ECS of froql and it's query language were inspired by flecs.
  It's creator wrote a lot of helpful articles about ECS design and also gave me direct advice ❤️
  
  I recommend reading https://medium.com/@ajmmertens/building-an-ecs-storage-in-pictures-642b8bfd6e04
  if you are curious.
</details>
<details>
  <summary>flecs-rust</summary>
  The idea for EntityViews came from here.
</details>
<details>
  <summary>ascent</summary>
  Transpiling a query language to Rust.
</details>
<details>
  <summary>nanoserde</summary>
  This is the fastest compiling proc macro crate for serialization I know.
  So copying from that I wrote froqls proc macro without any external dependencies.
</details>
<details>
  <summary>bevy</summary>
  Bevy has lots of interesting ideas and I ignored most of them.
  Froql has pretty different approach after all.
  
  But how bevy reserves entity IDs safely in deferred contexts is something I copied.
</details>
