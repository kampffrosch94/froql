use std::alloc::Layout;

use crate::{archetype::ArchetypeId, layout_vec::layout_vec_args};

type BitSet = hi_sparse_bitset::BitSet<hi_sparse_bitset::config::_128bit>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ComponentId(u32);

/// if set -> relation
/// otherwise -> normal component
pub const RELATION: u32 = 0b10000000000000000000000000000000;

/// if set -> target
/// otherwise -> origin
const IS_TARGET: u32 = RELATION >> 1;

/// Marks exclusive relationships.
/// A relation is exclusive if an origin can only have a single target.
///
/// For example if the relation `ChildOf(child, parent)` is exclusive
/// then a child can only have a single parent.
/// A parent can still have multiple children though.
pub const EXCLUSIVE: u32 = RELATION >> 2;

/// Marks symmetric relationships.
/// A relation is symmetric if `Rel(a,b)` implies `Rel(b,a).
// this means we don't have to distinguish between origin and target in the storage
pub const SYMMETRIC: u32 = RELATION >> 3;

/// Mark a relationship with cascading destruction.
/// When an origin in a cascading destruction relation gets destroyed,
/// then all its targets in that relation also get destroyed.
///
/// For example if the relation `Contains(faction, npc)` is cascading
/// then once the faction is destroyed all NPCs belonging to it are also destroyed.
pub const CASCADING_DESTRUCT: u32 = RELATION >> 4;

impl ComponentId {
    /// 24 bit ought to be enough component ids
    /// the rest is reserved for flags
    const MASK: u32 = 0b00000000111111111111111111111111;

    pub fn new(id: u32) -> Self {
        debug_assert!(id <= Self::MASK);
        Self(id)
    }

    // TODO newtype wrapper so users can't set none existent flags
    #[must_use]
    #[track_caller]
    pub fn set_flags(self, flags: u32) -> Self {
        assert_eq!(
            flags,
            flags & !Self::MASK,
            "There are none flag bits in the flags. {flags:#x}"
        );
        Self(self.0 | flags)
    }

    #[must_use]
    pub fn set_relation(self) -> Self {
        Self(self.0 | RELATION)
    }

    pub fn is_relation(&self) -> bool {
        (self.0 & RELATION) > 0
    }

    #[must_use]
    pub fn flip_target(self) -> Self {
        debug_assert!(self.is_relation());
        if self.is_symmetric() {
            // if the relation is symmetric we don't need to distinguish between origin&target
            self
        } else {
            Self(self.0 ^ IS_TARGET)
        }
    }

    pub fn is_target(&self) -> bool {
        self.is_relation() && (self.0 & IS_TARGET) > 0
    }

    #[must_use]
    pub fn set_exclusive(self) -> Self {
        debug_assert!(self.is_relation());
        Self(self.0 ^ EXCLUSIVE)
    }

    /// only returns true for the relation origin
    pub fn is_exclusive(&self) -> bool {
        self.is_relation() && (self.0 & EXCLUSIVE) > 0 && !self.is_target()
    }

    #[must_use]
    pub fn set_cascading(self) -> Self {
        debug_assert!(self.is_relation());
        Self(self.0 ^ CASCADING_DESTRUCT)
    }

    pub fn is_cascading(&self) -> bool {
        self.is_relation() && (self.0 & CASCADING_DESTRUCT) > 0
    }

    #[must_use]
    pub fn set_symmetric(self) -> Self {
        debug_assert!(self.is_relation());
        Self(self.0 ^ SYMMETRIC)
    }

    /// only returns true for the relation origin
    pub fn is_symmetric(&self) -> bool {
        self.is_relation() && (self.0 & SYMMETRIC) > 0
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
    pub archetypes: Box<BitSet>,
    /// if this component is a relationship target we need to track archetypes separately
    pub target_archetypes: Box<BitSet>,
}

impl Component {
    pub fn new<T: 'static>(id: ComponentId) -> Self {
        let (layout, drop_fn) = layout_vec_args::<T>();
        Component {
            layout,
            drop_fn,
            id,
            archetypes: Box::new(BitSet::new()),
            target_archetypes: Box::new(BitSet::new()),
        }
    }

    pub fn insert_archetype(&mut self, aid: ArchetypeId, cid: ComponentId) {
        if cid.is_target() {
            self.target_archetypes.insert(aid.0 as usize);
        } else {
            self.archetypes.insert(aid.0 as usize);
        }
    }

    pub fn has_archetype(&self, aid: ArchetypeId, cid: ComponentId) -> bool {
        if cid.is_target() {
            self.target_archetypes.contains(aid.0 as usize)
        } else {
            self.archetypes.contains(aid.0 as usize)
        }
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

    use hi_sparse_bitset::ops::*;

    #[test]
    fn set_operations() {
        let mut a = BitSet::new();
        let mut b = BitSet::new();
        a.insert(5);
        a.insert(6);
        a.insert(7);
        b.insert(6);
        let c = hi_sparse_bitset::apply(Sub, &a, &b);
        let v: Vec<_> = c.iter().collect();
        assert_eq!(&v, &[5, 7]);
    }
}
