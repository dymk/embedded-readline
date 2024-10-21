use crate::line_cursor::LineCursor;

#[derive(Debug, PartialEq)]
pub(crate) struct LineDiff {
    pub move_caret_before: isize,
    pub write_after_prefix: core::ops::Range<usize>,
    pub clear_after_prefix: usize,
    pub move_caret_after: isize,
}

pub(crate) fn calc_line_diff(old_line: &dyn LineCursor, new_line: &dyn LineCursor) -> LineDiff {
    let old_data = old_line.start_to_end();
    let new_data = new_line.start_to_end();

    // find the common prefix between the two lines
    let mut prefix_length = 0;
    for (i, (old, new)) in old_data.iter().zip(new_data.iter()).enumerate() {
        if old != new {
            break;
        }
        prefix_length = i + 1;
    }

    let (write_after_prefix, clear_after_prefix) =
        if old_data.len() == new_data.len() && prefix_length == old_data.len() {
            (0..0, 0)
        } else if old_data.len() > new_data.len() {
            if prefix_length == new_data.len() {
                (0..0, old_data.len() - prefix_length)
            } else {
                (
                    prefix_length..new_data.len(),
                    old_data.len() - new_data.len(),
                )
            }
        } else {
            (prefix_length..new_data.len(), 0)
        };

    let old_line_index = old_line.cursor_index() as isize;
    let move_caret_before = (prefix_length as isize) - old_line_index;
    extern crate std;
    let cursor_diff = new_line.cursor_index() as isize - old_line.cursor_index() as isize;
    let move_caret_after =
        -(move_caret_before + write_after_prefix.len() as isize + clear_after_prefix as isize)
            + cursor_diff;
    LineDiff {
        move_caret_before,
        write_after_prefix,
        clear_after_prefix,
        move_caret_after,
    }
}

#[cfg(test)]
mod tests {
    use crate::{line::Line, line_cursor::LineCursor, line_diff::LineDiff};

    use super::calc_line_diff;

    #[test]
    fn test_calc_diff1() {
        let mut old_line: Line<10> = Line::from_u8(b"hello");
        let mut new_line: Line<10> = Line::from_u8(b"heck");
        old_line.set_cursor_index(0);
        new_line.set_cursor_index(0);
        let result = calc_line_diff(&old_line, &new_line);
        assert_eq!(
            result,
            LineDiff {
                move_caret_before: 2,
                write_after_prefix: 2..4,
                clear_after_prefix: 1,
                move_caret_after: -5
            }
        );
    }

    #[rstest::rstest]
    #[case(b"hello", b"hello",   0, 0..0, 0,  0)]
    #[case(b"hello", b"hello!",  0, 5..6, 0,  0)]
    #[case(b"",      b"hi",      0, 0..2, 0,  0)]
    #[case(b"hello", b"he",     -3, 0..0, 3, -3)]
    #[case(b"hello", b"heck",   -3, 2..4, 1, -1)]
    fn test_calc_diff(
        #[case] old_data: &[u8],
        #[case] new_data: &[u8],
        #[case] move_caret_before: isize,
        #[case] write_after_prefix: core::ops::Range<usize>,
        #[case] clear_after_prefix: usize,
        #[case] move_caret_after: isize,
    ) {
        let old_line: Line<10> = Line::from_u8(old_data);
        let new_line: Line<10> = Line::from_u8(new_data);

        let result = calc_line_diff(&old_line, &new_line);
        assert_eq!(result.move_caret_before, move_caret_before);
        assert_eq!(
            result.write_after_prefix,
            write_after_prefix,
            "old: {:?}, new: {:?}",
            core::str::from_utf8(old_data).unwrap(),
            core::str::from_utf8(new_data).unwrap()
        );
        assert_eq!(result.clear_after_prefix, clear_after_prefix);
        assert_eq!(result.move_caret_after, move_caret_after);
    }
}
