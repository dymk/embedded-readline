pub fn get_two_mut_checked<T>(
    idx1: usize,
    idx2: usize,
    slice: &mut [T],
) -> Result<(&mut T, &mut T), &'static str> {
    if idx1 >= slice.len() {
        return Err("idx1 out of range");
    }
    // (a) => idx1 < slice.len()
    if idx2 >= slice.len() {
        return Err("idx2 out of range");
    }
    // (b) => idx2 < slice.len()
    if idx1 == idx2 {
        return Err("idx1 == idx2; must be different");
    }
    // (c) => idx1 != idx2

    let (swapped, idx1, idx2) = if idx1 < idx2 {
        (false, idx1, idx2)
    } else {
        (true, idx2, idx1)
    };
    // (d) c => idx1 < idx2
    // (e) c => idx2 - idx1 > 0
    // (f) b => idx1+1 < slice.len()

    let (s1, s2) = slice.split_at_mut(idx1 + 1);
    let e1 = &mut s1[idx1];
    let e2 = &mut s2[idx2 - idx1 - 1];

    if swapped {
        Ok((e2, e1))
    } else {
        Ok((e1, e2))
    }
}

// assume [0, 1, 2, 3]
// idx1 = 0
// idx2 = 1
// s1, s2 = [0], [1, 2, 3]

#[cfg(test)]
mod tests {
    extern crate std;
    use super::get_two_mut_checked;

    #[test]
    fn test_middle_works() {
        let mut arr = [0, 1, 2, 3, 4];
        let (e1, e2) = get_two_mut_checked(1, 2, &mut arr).unwrap();
        assert_eq!(*e1, 1);
        assert_eq!(*e2, 2);
    }

    #[test]
    fn test_middle_rev_works() {
        let mut arr = [0, 1, 2, 3, 4];
        let (e1, e2) = get_two_mut_checked(2, 1, &mut arr).unwrap();
        assert_eq!(*e1, 2);
        assert_eq!(*e2, 1);
    }

    #[test]
    fn test_boundary_works() {
        let mut arr = [0, 1, 2, 3, 4];
        let (e1, e2) = get_two_mut_checked(0, 4, &mut arr).unwrap();
        assert_eq!(*e1, 0);
        assert_eq!(*e2, 4);

        let mut arr = [0, 1, 2, 3, 4];
        let (e1, e2) = get_two_mut_checked(0, 1, &mut arr).unwrap();
        assert_eq!(*e1, 0);
        assert_eq!(*e2, 1);
    }

    #[test]
    fn test_boundary_rev_works() {
        let mut arr = [0, 1, 2, 3, 4];
        let (e1, e2) = get_two_mut_checked(4, 0, &mut arr).unwrap();
        assert_eq!(*e1, 4);
        assert_eq!(*e2, 0);

        let mut arr = [0, 1, 2, 3, 4];
        let (e1, e2) = get_two_mut_checked(1, 0, &mut arr).unwrap();
        assert_eq!(*e1, 1);
        assert_eq!(*e2, 0);
    }
}
