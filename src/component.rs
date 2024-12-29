use std::alloc::Layout;

use crate::{archetype::ArchetypeId, layout_vec::layout_vec_args};

type BitSet = hi_sparse_bitset::BitSet<hi_sparse_bitset::config::_128bit>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ComponentId(u32);

impl ComponentId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    #[track_caller]
    pub fn from_usize(id: usize) -> Self {
        Self(u32::try_from(id).unwrap())
    }

    pub fn as_index(&self) -> usize {
        self.0 as usize
    }

    pub fn inner(&self) -> u32 {
        self.0
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
