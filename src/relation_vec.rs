//! small vector optimization specifically for relations

use std::{mem::ManuallyDrop, ops::Deref, ptr::NonNull};

const INLINE_COUNT: usize = 3;

struct RelationVecInline {
    elements: [u32; INLINE_COUNT],
}

#[repr(packed)]
struct RelationVecOutline {
    cap: u32,
    ptr: NonNull<u32>,
}

union RelationVecUnion {
    inline: ManuallyDrop<RelationVecInline>,
    outline: ManuallyDrop<RelationVecOutline>,
}

struct RelationVec {
    len: u32,
    content: RelationVecUnion,
}

impl RelationVec {
    pub fn new() -> Self {
        let content = RelationVecUnion {
            inline: ManuallyDrop::new(RelationVecInline {
                elements: [0; INLINE_COUNT],
            }),
        };
        RelationVec { len: 0, content }
    }

    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn push(&mut self, new: u32) {
        // maybe sort? for binary search
        if self.len < INLINE_COUNT as u32 {
            let inline = unsafe { &mut self.content.inline };
            inline.elements[self.len as usize] = new;
        } else {
            todo!();
        }
        self.len += 1;
    }

    pub fn remove(&mut self, element: u32) {
        if self.len <= INLINE_COUNT as u32 {
            let slice = &mut unsafe { &mut self.content.inline }.elements;
            let index = slice.iter().position(|it| *it == element);
            if let Some(index) = index {
                let last = slice.len() - 1;
                slice.swap(index, last);
                self.len -= 1;
            }
        } else {
            todo!();
        }
    }
}

impl Drop for RelationVec {
    fn drop(&mut self) {
        if self.len > INLINE_COUNT as u32 {
            unsafe {
                let outline = &mut self.content.outline;
                // TODO check this is dropping the whole slice
                outline.ptr.drop_in_place();
                ManuallyDrop::drop(outline);
            }
        } else {
            unsafe {
                let inline = &mut self.content.inline;
                ManuallyDrop::drop(inline);
            }
        }
    }
}

impl Deref for RelationVec {
    type Target = [u32];
    fn deref(&self) -> &[u32] {
        if self.len <= INLINE_COUNT as u32 {
            let inline = unsafe { &self.content.inline };
            return &inline.elements[0..self.len as usize];
        } else {
            let outline = unsafe { &self.content.outline };
            unsafe { std::slice::from_raw_parts(outline.ptr.as_ptr(), self.len as usize) }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_struct_sizes() {
        assert_eq!(12, size_of::<RelationVecInline>());
        assert_eq!(12, size_of::<RelationVecOutline>());
        assert_eq!(12, size_of::<RelationVecUnion>());
        assert_eq!(16, size_of::<RelationVec>());
    }


    #[test]
    fn inline_vec() {
        let mut vec = RelationVec::new();
        assert_eq!(0, vec.len());
        vec.push(10);
        vec.push(20);
        vec.push(30);
        assert_eq!(3, vec.len());
        assert_eq!(&[10, 20, 30], &vec[..]);
        vec.remove(20);
        vec.remove(42);
        assert_eq!(&[10, 30], &vec[..]);
        assert_eq!(2, vec.len());
        vec.remove(10);
        vec.remove(30);
        vec.remove(42);
        let arr: &[u32] = &[];
        assert_eq!(arr, &vec[..]);
        assert_eq!(0, vec.len());
    }

}
