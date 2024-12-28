use std::marker::PhantomData;

use crate::entity_store::EntityId;

struct RelationOrigin<T> {
    phantom: PhantomData<T>,
    targets: Vec<EntityId>, // TODO smallvec instead
}
