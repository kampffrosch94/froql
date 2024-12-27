use std::alloc::Layout;

use crate::layout_vec::layout_vec_args;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(pub u32);

pub struct Component {
    pub id: ComponentId,
    pub layout: Layout,
    pub drop_fn: Box<fn(*mut u8)>,
}

impl Component {
    pub fn new<T: 'static>(id: ComponentId) -> Self {
        let (layout, drop_fn) = layout_vec_args::<T>();
        Component {
            layout,
            drop_fn,
            id,
        }
    }
}
