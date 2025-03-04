use std::alloc;
use std::alloc::Layout;
use std::ptr::NonNull;

/// A vector which does not care about the underlying type, just its Layout.
/// It can not be cloned, but elements can be savely deleted.
pub struct LayoutVec {
    len: u32,
    capacity: u32,
    element_size: u32,
    element_align: u32,
    ptr: NonNull<u8>,
    drop_fn: Box<fn(*mut u8)>,
}

impl LayoutVec {
    /// layout: Layout of the contained type
    /// drop_fn: Boxed fn of drop_in_place for the contained type with type punning
    ///
    /// the correct arguments for a type can be obtained by calling `layout_vec_args::<T>()`
    pub fn new(layout: Layout, drop_fn: Box<fn(*mut u8)>) -> Self {
        debug_assert!(layout.size() > 0, "Layout vec does not handle ZSTs");
        LayoutVec {
            len: 0,
            capacity: 0,
            element_size: layout.size() as u32,
            element_align: layout.align() as u32,
            ptr: NonNull::dangling(),
            drop_fn,
        }
    }

    /// Useful for hotreloading
    pub fn change_drop_function(&mut self, drop_fn: Box<fn(*mut u8)>) {
        self.drop_fn = drop_fn;
    }

    pub fn element_size(&self) -> usize {
        self.element_size as usize
    }

    pub fn len(&self) -> u32 {
        self.len
    }

    // mostly from https://doc.rust-lang.org/nomicon/vec/vec-alloc.html
    fn grow(&mut self) {
        let new_ptr = if self.capacity == 0 {
            self.capacity = 4;
            let new_layout = self.compute_layout();
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = self.compute_layout();
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            self.capacity *= 2;
            let new_layout = self.compute_layout();
            unsafe { alloc::realloc(old_ptr, old_layout, new_layout.size()) }
        };

        self.ptr = match NonNull::new(new_ptr) {
            Some(p) => p,
            None => alloc::handle_alloc_error(self.compute_layout()),
        };
    }

    fn compute_layout(&self) -> Layout {
        let array_size = (self.element_size * self.capacity) as usize;
        let align = self.element_align as usize;
        Layout::from_size_align(array_size, align)
            .expect("Can't create layout from {element_layout:?} with {n} elements.")
    }

    /// grows the vec by one element and provides a pointer the caller can write the element to
    #[must_use]
    pub unsafe fn half_push(&mut self) -> *mut u8 {
        if self.len >= self.capacity {
            self.grow();
        }
        let r = self
            .ptr
            .as_ptr()
            .add((self.len * self.element_size) as usize);
        self.len += 1;
        r
    }

    /// deletes the last element and calls the drop function on it if necessary
    pub fn remove_last(&mut self) {
        debug_assert!(self.len > 0);
        self.len -= 1;
        let r = unsafe {
            self.ptr
                .as_ptr()
                .add((self.len * self.element_size) as usize)
        };
        (self.drop_fn)(r)
    }

    /// deletes element at index and drops it
    /// then swaps the last element into the resulting hole and reduces len by one
    /// returns the index of the last element before it was swapped
    pub fn remove_swap(&mut self, index: u32) -> u32 {
        debug_assert!(self.len > 0 && index < self.len);
        if index == self.len - 1 {
            self.remove_last();
        } else {
            let deletee = unsafe { self.ptr.as_ptr().add((index * self.element_size) as usize) };
            (self.drop_fn)(deletee);
            unsafe {
                let last = self.get(self.len - 1);
                std::ptr::copy_nonoverlapping(last, deletee, self.element_size as usize);
            }
            self.len -= 1;
        }
        self.len
    }

    /// returns a pointer to the element at index
    #[inline]
    #[must_use]
    #[track_caller]
    pub unsafe fn get(&self, index: u32) -> *mut u8 {
        debug_assert!(
            self.len > 0 && index < self.len,
            "Len: {} Index: {index}",
            self.len
        );
        self.ptr.as_ptr().add((index * self.element_size) as usize)
    }

    /// moves entry between two LayoutVecs with the same layout
    /// returns the index of the last element (in `from`)
    /// before it was swapped (like `remove_swap`)
    /// also returns the new index in `to` as second tuple member
    pub unsafe fn move_entry(from: &mut Self, to: &mut Self, index: u32) -> (u32, u32) {
        debug_assert_eq!(from.element_align, to.element_align);
        debug_assert_eq!(from.element_size, to.element_size);
        debug_assert!(from.len > 0 && index < from.len);
        unsafe {
            // make space and copy over
            let entry = from.get(index);
            let target = to.half_push();
            std::ptr::copy_nonoverlapping(entry, target, from.element_size as usize);

            // cleanup the hole
            // don't need to do anything if it was the last element,
            // otherwise need to swap in the last element
            from.len -= 1;
            if index != from.len {
                let last = from
                    .ptr
                    .as_ptr()
                    .add((from.len * from.element_size) as usize);
                std::ptr::copy_nonoverlapping(last, entry, from.element_size as usize);
            }
        }
        // we return the index of the element which was swapped
        // so that the caller can update the archetype row for the swapped entity if necessary
        (from.len, to.len - 1)
    }
}

impl Drop for LayoutVec {
    fn drop(&mut self) {
        if self.capacity > 0 {
            while self.len > 0 {
                self.remove_last();
            }
            let layout = self.compute_layout();
            unsafe {
                alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}

#[inline]
pub fn layout_vec_args<T>() -> (Layout, Box<fn(*mut u8)>) {
    (
        Layout::new::<T>(),
        Box::new(|ptr: *mut u8| unsafe {
            let ptr = std::mem::transmute::<*mut u8, *mut T>(ptr);
            std::ptr::drop_in_place(ptr);
        }),
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_struct_sizes() {
        assert_eq!(32, size_of::<LayoutVec>());
    }

    #[test]
    fn push_and_get() {
        struct MyStruct(usize);
        let (layout, drop_fn) = layout_vec_args::<MyStruct>();
        let mut vec = LayoutVec::new(layout, drop_fn);
        for i in 0..10 {
            unsafe {
                let ptr = vec.half_push();
                let ptr = std::mem::transmute::<*mut u8, *mut MyStruct>(ptr);
                std::ptr::write(ptr, MyStruct(i * 10));
            }
        }
        let get = move |index| unsafe {
            let ptr = vec.get(index);
            let ptr = std::mem::transmute::<*mut u8, *const MyStruct>(ptr);
            &*ptr
        };
        assert_eq!(50, get(5).0);
        assert_eq!(0, get(0).0);
        assert_eq!(90, get(9).0);
    }

    #[test]
    fn test_remove_swap() {
        struct MyStruct(usize);
        let (layout, drop_fn) = layout_vec_args::<MyStruct>();
        let mut vec = LayoutVec::new(layout, drop_fn);
        for i in 0..10 {
            unsafe {
                let ptr = vec.half_push();
                let ptr = std::mem::transmute::<*mut u8, *mut MyStruct>(ptr);
                std::ptr::write(ptr, MyStruct(i * 10));
            }
        }
        let old_index = vec.remove_swap(5);
        assert_eq!(9, old_index);
        // remove last element
        let old_index = vec.remove_swap(8);
        assert_eq!(8, old_index);
        let get = move |index| unsafe {
            let ptr = vec.get(index);
            let ptr = std::mem::transmute::<*mut u8, *const MyStruct>(ptr);
            &*ptr
        };
        assert_eq!(90, get(5).0);
        assert_eq!(70, get(7).0);
    }
}
