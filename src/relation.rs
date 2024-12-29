use std::marker::PhantomData;

use crate::relation_vec::RelationVec;

#[repr(transparent)]
pub struct RelationOrigin<T> {
    phantom: PhantomData<T>,
    targets: RelationVec,
}

#[repr(transparent)]
pub struct RelationTarget<T> {
    phantom: PhantomData<T>,
    origins: RelationVec,
}
