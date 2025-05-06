/// uses `split_at_mut` internally to give two mutable references into a slice
pub fn get_mut_2<T>(slice: &mut [T], a: u32, b: u32) -> (&mut T, &mut T) {
    let a = a as usize;
    let b = b as usize;
    if a < b {
        let (a_slice, b_slice) = slice.split_at_mut(b);
        (&mut a_slice[a], &mut b_slice[0])
    } else {
        let (b_slice, a_slice) = slice.split_at_mut(a);
        (&mut a_slice[0], &mut b_slice[b])
    }
}

// can be const once const_type_name stabilizes
pub fn short_type_name<T>() -> &'static str {
    let name = std::any::type_name::<T>();
    if name.contains("<") {
        name
    } else {
        name.split("::").last().unwrap()
    }
}

#[cfg(test)]
mod test {
    use crate::util::short_type_name;

    #[test]
    #[cfg(not(miri))]
    fn short_type_name_test() {
        struct Blab;
        insta::assert_snapshot!(short_type_name::<u32>(), @"u32");
        insta::assert_snapshot!(short_type_name::<Vec<u32>>(), @"alloc::vec::Vec<u32>");
        insta::assert_snapshot!(short_type_name::<Blab>(), @"Blab");
        insta::assert_snapshot!(short_type_name::<Vec<Blab>>(), @"alloc::vec::Vec<froql::util::test::short_type_name_test::Blab>");
    }
}
