# Relations

A Relations is always between two entities.

You can think of Entities being nodes on directed graph and with relationships being the edges.

Relations are distinguished with a rust type via it's TypeId.
To prevent accidently adding a Relation as a Component it is recommend to use an inhibited types for them. 
For example an enum with no variants.

## Registration




## Creation and Deletion

Relations a

TODO World API

TODO builder api

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
