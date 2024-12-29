use std::alloc::Layout;

use crate::{archetype::ArchetypeId, layout_vec::layout_vec_args};

type BitSet = hi_sparse_bitset::BitSet<hi_sparse_bitset::config::_128bit>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ComponentId(u32);

impl ComponentId {
    /// 24 bit ought to be enough component ids
    /// the rest is reserved for flags
    const MASK: u32 = 0b00000000111111111111111111111111;
    /// if set -> relation
    /// otherwise -> normal component
    const IS_RELATION: u32 = 0b10000000000000000000000000000000;
    /// if set -> target
    /// otherwise -> origin
    const IS_TARGET: u32 = Self::IS_RELATION >> 1;

    pub fn new(id: u32) -> Self {
        debug_assert!(id <= Self::MASK);
        Self(id)
    }

    pub fn set_relation(self) -> Self {
        Self(self.0 | Self::IS_RELATION)
    }

    pub fn is_relation(&self) -> bool {
        (self.0 & Self::IS_RELATION) > 0
    }

    pub fn flip_target(self) -> Self {
        Self(self.0 ^ Self::IS_TARGET)
    }

    pub fn is_target(&self) -> bool {
        (self.0 & Self::IS_TARGET) > 0
    }

    #[track_caller]
    pub fn from_usize(id: usize) -> Self {
        Self::new(u32::try_from(id).unwrap())
    }

    pub fn as_index(&self) -> usize {
        self.id() as usize
    }

    pub fn id(&self) -> u32 {
        self.0 & Self::MASK
    }
}

pub struct Component {
    pub id: ComponentId,
    pub layout: Layout,
    pub drop_fn: Box<fn(*mut u8)>,
    /// keeps track of what archetypes have this component
    archetypes: Box<BitSet>,
}

impl Component {
    pub fn new<T: 'static>(id: ComponentId) -> Self {
        let (layout, drop_fn) = layout_vec_args::<T>();
        Component {
            layout,
            drop_fn,
            id,
            archetypes: Box::new(BitSet::new()),
        }
    }

    pub fn insert_archetype(&mut self, aid: ArchetypeId) {
        self.archetypes.insert(aid.0 as usize);
    }

    pub fn has_archetype(&self, aid: ArchetypeId) -> bool {
        self.archetypes.contains(aid.0 as usize)
    }

    pub fn get_archetypes(&self) -> impl Iterator<Item = ArchetypeId> + use<'_> {
        self.archetypes
            .iter()
            .map(|index| ArchetypeId(index as u32))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bitmasking() {
        let ci = ComponentId::new(32);
        assert_eq!(32, ci.as_index());
        assert!(!ci.is_relation());
        assert_eq!(32, ci.id());
        let ci = ComponentId::new(32).set_relation();
        assert_eq!(32, ci.as_index());
        assert_eq!(32, ci.id());
        assert!(ci.is_relation());
        let ci = ComponentId::new(32).set_relation().flip_target();
        assert_eq!(32, ci.as_index());
        assert!(ci.is_relation());
        assert!(ci.is_target());
    }
}
