use std::{cell::RefCell, marker::PhantomData};

#[derive(Clone, Copy, Debug)]
pub struct ComponentId(pub u32);
#[derive(Clone, Copy, Debug)]
pub struct ArchetypetId(pub u32);

pub struct Archetype {}

/// Standin for erased types
pub enum Erased {}
type ErasedPointer = *const RefCell<Erased>;

/// Holds the data for one component in an archetype
pub struct ComponentColumn {}

trait ColumnVisitor {}

struct ColumnVisitorImpl<T> {
    phantom: PhantomData<T>,
}

impl<T> ColumnVisitor for ColumnVisitorImpl<T> {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_struct_sizes() {
        assert_eq!(0, size_of::<ColumnVisitorImpl<u32>>());
        assert_eq!(16, size_of::<Box<dyn ColumnVisitor>>());
    }
}
