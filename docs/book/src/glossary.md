# Glossary

| term                 | explanation                                                                 | example                                          |
|----------------------|-----------------------------------------------------------------------------|--------------------------------------------------|
| entity               | something that can have components and relationships                        |                                                  |
| component            | a struct attached to an entity                                              | `Health`   (Health is a normal Rust type)        |
| relation             | a connection between two entities                                           | `Friends(a,b)`   (Friends is a normal Rust type) |
| variable             | a standin for an entity in a query                                          | `Health(a)` <- `a` is a variable                 |
| component access     |                                                                             |                                                  |
| mut component access |                                                                             |                                                  |
| singleton            | something that only exists once in a `World`                                | `world.singleton::<GameTicks>()`                 |
| outvar               | entity variable that should be returned by the query                        | `&this`                                          |
| invar                | a value for an entity that is passed into a query                           | `Health(\*me)`                                   |
| constraint           | something that filters out results from a query                             | `this != that`                                   |
| uncomponent          | negative component constraint, filters out results where var has component  | `!Health`                                        |
| unrelation           | negative relation constraint, filters out results where Relation is present | `!ChildOf(this, other)`                          |
| create               | creates an entity or entityview                                             | `let e = world.create()`                         |
| destroy              | removes an entity and cleans up its relations and components                | `e.destroy()`                                    |
| add                  | adds a component to an entity                                               | `e.add(Comp{})`                                  |
| remove               | removes a component from an entity                                          | `e.remove::<Comp>()`                             |
| relate               | creates a relation between two entities                                     | `a.relate<sub>to</sub>::<Friend>(b)`             |
| unrelate             | removes a relation between two entities                                     | `a.unrelate_to_::<Friend>(b)`                    |
| immediate            | a change of entities, components or relations is immediately executed       | `e.add(Comp{});` (with a mutable EntityView)     |
| deferred             | a change is queued up until `World::process()` is called                    | `e.add(Comp{});` (with a EntityViewDeferred)     |
| exclusive            | Rel(a,b) gets removed when Rel(a,c) is created                              |                                                  |
| reflexive            | Rel(a,b) also means Rel(b,a)                                                |                                                  |
| transitive           | Rel(a,b) and Rel(b,c) means Rel(a,c) implicitly                             |                                                  |
| cascading delete     | when a from Rel(a,b) gets destroyed, then b also gets destroyed             |                                                  |
