use std::marker::PhantomData;

use crate::relation_vec::RelationVec;

#[repr(transparent)]
pub struct Relation<T> {
    phantom: PhantomData<T>,
    targets: RelationVec,
}
