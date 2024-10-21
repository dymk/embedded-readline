use crate::line_cursor::LineCursor;

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

pub fn previous_word_cursor_position(line: &mut dyn LineCursor) {
    // rewind past spaces
    while let Some(c) = line.at_cursor(-1) {
        // println!("space? 0x{:#02?}", c);
        if !c.is_ascii_whitespace() || line.cursor_index() == 0 {
            break;
        }
        line.move_cursor(-1);
    }

    // find the start of the word
    while let Some(c) = line.at_cursor(-1) {
        // println!("char? 0x{:#02?}", c);
        if c.is_ascii_whitespace() || line.cursor_index() == 0 {
            break;
        }
        line.move_cursor(-1);
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{get_two_mut_checked, previous_word_cursor_position};

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

    #[rstest::rstest]
    #[case("", "", "", "", 0..0)]
    #[case("   ", "", "", "", 0..3)]
    #[case("", "   ", "", "   ", 0..0)]
    #[case("hello world ", "", "hello ", "hello ", 6..12)]
    #[case("hello world", " ", "hello ", "hello  ", 6..11)]
    #[case("hello wo", "rld ", "hello ", "hello rld ", 6..8)]
    #[case("hello ", "world ", "", "world ", 0..6)]
    #[case("hello", " world ", "", " world ", 0..5)]
    fn test_simple_word_move(
        #[case] before_input: &str,
        #[case] after_input: &str,
        #[case] expected_start_to_cursor: &str,
        #[case] expected_start_to_end: &str,
        #[case] expected_range: core::ops::Range<usize>,
    ) {
        use crate::{line::Line, line_cursor::LineCursor as _};

        let buf = [before_input.as_bytes(), after_input.as_bytes()].concat();
        let mut line = Line::<16>::default();
        line.set_from_u8(&buf);
        line.set_cursor_index(before_input.len());
        let old_cursor = line.cursor_index();
        previous_word_cursor_position(&mut line);
        let new_cursor = line.cursor_index();
        let range = new_cursor..old_cursor;
        assert_eq!(
            expected_start_to_cursor.as_bytes(),
            line.start_to_cursor(),
            "`{}` != `{}`",
            expected_start_to_cursor,
            core::str::from_utf8(line.start_to_cursor()).unwrap(),
        );
        assert_eq!(expected_range, range);

        line.remove_range(range);
        assert_eq!(
            expected_start_to_end.as_bytes(),
            line.start_to_end(),
            "`{}` != `{}`",
            expected_start_to_end,
            core::str::from_utf8(line.start_to_end()).unwrap(),
        )
    }
}
