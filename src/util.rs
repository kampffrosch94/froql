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
