use crate::line::Line;

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

pub fn previous_word_cursor_position<const LEN: usize>(line: &mut Line<LEN>) {
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
#[track_caller]
pub fn assert_eq_u8(actual: &[u8], expected: &str) {
    if actual != expected.as_bytes() {
        let actual = std::str::from_utf8(actual).unwrap();
        panic!("{:?} != {:?}", actual, expected);
    }
}

/// Builds a Line struct using string literals.
/// The cursor position is indicated by a pipe (|) character.
/// Usage:
/// make_line!(|) => empty string, cursor @ 0
/// make_line!("a"|) => "a", cursor @ 1
/// make_line!(|"a") => "a", cursor @ 0
#[macro_export]
#[cfg(test)]
macro_rules! make_line {
    ($($head:literal)? | $($tail:literal)? $(; $len:literal)?) => {{
        let bytes = make_line!(@concat $($head)? $($tail)?);
        let cursor_index = make_line!(@len $($head)?);
        make_line!(@impl bytes, cursor_index $(; $len)?)
    }};

    (@len) => { 0 };
    (@len $lit:literal) => { $lit.len() };

    (@concat) => { "" };
    (@concat $a:literal) => { $a };
    (@concat $a:literal $b:literal) => { [$a, $b].concat() };

    (@impl $data:expr, $cursor:expr; $max_len:literal) => {{
        let mut line = crate::line::Line::<$max_len>::from_u8($data.as_bytes());
        line.set_cursor_index($cursor);
        line
    }};

    (@impl $data:expr, $cursor:expr) => {{
        let mut line = crate::line::Line::from_u8($data.as_bytes());
        line.set_cursor_index($cursor);
        line
    }};
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{assert_eq_u8, get_two_mut_checked, previous_word_cursor_position};
    use crate::line::Line;

    #[test]
    fn test_line_macro() {
        let line = make_line!["a" | "b"; 2];
        assert_eq_u8(line.start_to_cursor(), "a");
        assert_eq_u8(line.start_to_end(), "ab");

        let line: Line<2> = make_line!["cd"|];
        assert_eq_u8(line.start_to_cursor(), "cd");
        assert_eq_u8(line.start_to_end(), "cd");

        let line = make_line![|"ef"; 2];
        assert_eq_u8(line.start_to_cursor(), "");
        assert_eq_u8(line.start_to_end(), "ef");

        assert_eq!(make_line!["abc"|], make_line!["abc"|; 4])
    }

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
    #[case("",             "",        "",       "",           0..0)]
    #[case("   ",          "",        "",       "",           0..3)]
    #[case("",             "   ",     "",       "   ",        0..0)]
    #[case("hello world ", "",        "hello ", "hello ",     6..12)]
    #[case("hello world",  " ",       "hello ", "hello  ",    6..11)]
    #[case("hello wo",     "rld ",    "hello ", "hello rld ", 6..8)]
    #[case("hello ",       "world ",  "",       "world ",     0..6)]
    #[case("hello",        " world ", "",       " world ",    0..5)]
    fn test_simple_word_move(
        #[case] before_input: &str,
        #[case] after_input: &str,
        #[case] expected_start_to_cursor: &str,
        #[case] expected_start_to_end: &str,
        #[case] expected_range: core::ops::Range<usize>,
    ) {
        use crate::line::Line;

        let buf = [before_input.as_bytes(), after_input.as_bytes()].concat();
        let mut line = Line::<16>::from_u8(&buf);
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

        line.remove_range(range).unwrap();
        assert_eq!(
            expected_start_to_end.as_bytes(),
            line.start_to_end(),
            "`{}` != `{}`",
            expected_start_to_end,
            core::str::from_utf8(line.start_to_end()).unwrap(),
        )
    }
}
