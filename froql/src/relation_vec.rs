//! small vector optimization specifically for relations

use std::{
    alloc::{self, Layout},
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

const INLINE_COUNT: usize = 3;

struct RelationVecInline {
    elements: [u32; INLINE_COUNT],
}

impl RelationVecInline {
    pub fn new() -> Self {
        RelationVecInline {
            elements: [0; INLINE_COUNT],
        }
    }
}

#[repr(packed(4))]
struct RelationVecOutline {
    ptr: NonNull<u32>,
    cap: u32,
}

impl RelationVecOutline {
    fn new_alloc() -> RelationVecOutline {
        let cap = 8;
        let layout = Self::layout(cap);
        let new_ptr = unsafe { alloc::alloc(layout) as *mut u32 };
        let ptr = match NonNull::new(new_ptr) {
            Some(p) => p,
            None => alloc::handle_alloc_error(layout),
        };
        RelationVecOutline { ptr, cap }
    }

    fn layout(cap: u32) -> Layout {
        Layout::from_size_align(size_of::<u32>() * cap as usize, align_of::<u32>()).unwrap()
    }

    unsafe fn write(&mut self, index: usize, val: u32) {
        debug_assert!(index < self.cap as usize);
        unsafe {
            let dst = self.ptr.as_ptr().add(index);
            std::ptr::write(dst, val)
        };
    }

    unsafe fn grow(&mut self) {
        let old_layout = RelationVecOutline::layout(self.cap);
        self.cap *= 2;
        let new_layout = RelationVecOutline::layout(self.cap);
        unsafe {
            let ptr = alloc::realloc(self.ptr.as_ptr() as *mut u8, old_layout, new_layout.size());
            let ptr = match NonNull::new(ptr as *mut u32) {
                Some(p) => p,
                None => alloc::handle_alloc_error(new_layout),
            };
            self.ptr = ptr;
        }
    }

    unsafe fn dealloc(&mut self) {
        let layout = RelationVecOutline::layout(self.cap);
        unsafe {
            alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
        }
    }
}

union RelationVecUnion {
    inline: ManuallyDrop<RelationVecInline>,
    outline: ManuallyDrop<RelationVecOutline>,
}

pub struct RelationVec {
    len: u32,
    content: RelationVecUnion,
}

impl RelationVec {
    pub fn new() -> Self {
        let content = RelationVecUnion {
            inline: ManuallyDrop::new(RelationVecInline::new()),
        };
        RelationVec { len: 0, content }
    }

    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn push(&mut self, new_val: u32) {
        // maybe sort? for binary search
        if self.len < INLINE_COUNT as u32 {
            let inline = unsafe { &mut self.content.inline };
            inline.elements[self.len as usize] = new_val;
        } else {
            if self.len == INLINE_COUNT as u32 {
                // transform to outline vec
                let inline = unsafe { &mut self.content.inline };
                let mut outline = RelationVecOutline::new_alloc();
                for i in 0..self.len as usize {
                    unsafe { outline.write(i, inline.elements[i]) };
                }
                self.content = RelationVecUnion {
                    outline: ManuallyDrop::new(outline),
                };
            }
            unsafe {
                let outline = &mut self.content.outline;
                // grow if necessary
                if self.len == outline.cap {
                    outline.grow();
                }
                outline.write(self.len as usize, new_val);
            }
        }
        self.len += 1;
    }

    pub fn remove(&mut self, element: u32) {
        let index = self[..].iter().position(|it| *it == element);
        if let Some(index) = index {
            if self.len <= INLINE_COUNT as u32 {
                let slice = &mut unsafe { &mut self.content.inline }.elements;
                let last = self.len - 1;
                slice.swap(index, last as usize);
            } else {
                let slice = &mut self[..];
                let last = slice.len() - 1;
                slice.swap(index, last);
                if self.len == INLINE_COUNT as u32 + 1 {
                    // convert back to inline
                    let mut inline = RelationVecInline::new();
                    for i in 0..INLINE_COUNT {
                        inline.elements[i] = self[i];
                    }
                    let outline = unsafe { &mut self.content.outline };
                    unsafe {
                        outline.dealloc();
                    }
                    self.content = RelationVecUnion {
                        inline: ManuallyDrop::new(inline),
                    };
                }
            }
            self.len -= 1;
        }
    }
}

impl Drop for RelationVec {
    fn drop(&mut self) {
        if self.len > INLINE_COUNT as u32 {
            unsafe {
                let outline = &mut self.content.outline;
                outline.dealloc();
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

impl DerefMut for RelationVec {
    fn deref_mut(&mut self) -> &mut [u32] {
        if self.len <= INLINE_COUNT as u32 {
            let inline = unsafe { &mut self.content.inline };
            return &mut inline.elements[0..self.len as usize];
        } else {
            let outline = unsafe { &self.content.outline };
            unsafe { std::slice::from_raw_parts_mut(outline.ptr.as_ptr(), self.len as usize) }
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

    #[test]
    fn outline_vec() {
        let mut vec = RelationVec::new();
        assert_eq!(0, vec.len());
        vec.push(10);
        vec.push(20);
        vec.push(30);
        vec.push(40);
        vec.push(50);
        assert_eq!(5, vec.len());
        assert_eq!(&[10, 20, 30, 40, 50], &vec[..]);
        vec.remove(20);
        vec.remove(30);
        assert_eq!(&[10, 50, 40], &vec[..]);
    }

    #[test]
    fn grow() {
        let mut vec = RelationVec::new();
        for i in 0..20 {
            vec.push(i * 100);
        }
        assert_eq!(20, vec.len);
    }
}
