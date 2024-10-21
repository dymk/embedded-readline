use crate::{
    line::Line,
    line_cursor::LineCursor,
    line_diff::LineDiff,
    util::{get_two_mut_checked, previous_word_cursor_position},
};

#[derive(Debug)]
pub struct Buffers<const MAX_LINE_LEN: usize, const MAX_LINES: usize> {
    lines: [Line<MAX_LINE_LEN>; MAX_LINES],
    last_idx: usize,
    offset: usize,
}

impl<const A: usize, const B: usize> Default for Buffers<A, B> {
    fn default() -> Self {
        Self {
            lines: [Line::default(); B],
            last_idx: 0,
            offset: 0,
        }
    }
}

impl<const A: usize, const B: usize> Buffers<A, B> {
    fn selected_idx(&self) -> usize {
        (self.last_idx - self.offset) % B
    }

    fn prepare_to_change_line(&mut self) {
        // copy selected into last history slot
        let from_idx = self.selected_idx();
        self.offset = 0;
        let to_idx = self.selected_idx();
        if from_idx != to_idx {
            let (from_line, to_line) =
                get_two_mut_checked(from_idx, to_idx, &mut self.lines).unwrap();
            to_line.set_from_cursor(from_line);
        }
    }
}

// impl<const A: usize, const B: usize> Buffers<A, B> {}

// impl<const A: usize, const B: usize> BufferTrait for Buffers<A, B> {
impl<const A: usize, const B: usize> Buffers<A, B> {
    pub(crate) fn current_line(&self) -> &dyn LineCursor {
        &self.lines[self.selected_idx()]
    }

    pub(crate) fn current_line_mut(&mut self) -> &mut dyn LineCursor {
        &mut self.lines[self.selected_idx()]
    }

    pub(crate) fn insert_chars(&mut self, c: &[u8]) -> LineDiff {
        self.prepare_to_change_line();
        let line = self.current_line_mut();
        let cursor_index = line.cursor_index();
        line.insert_range(cursor_index, c);
        let num_after_cursor = line.num_after_cursor();
        LineDiff {
            move_caret_before: 0,
            write_after_prefix: Some(cursor_index..line.end_index()),
            clear_after_prefix: 0,
            move_caret_after: -(num_after_cursor as isize),
        }
    }

    pub(crate) fn delete_chars(&mut self, n: usize) -> LineDiff {
        self.prepare_to_change_line();
        let line = self.current_line_mut();
        let cursor_index = line.cursor_index();
        if cursor_index == 0 {
            return LineDiff {
                move_caret_before: 0,
                write_after_prefix: None,
                clear_after_prefix: 0,
                move_caret_after: 0,
            };
        }

        let n = n.min(cursor_index);
        let range = (cursor_index - n)..cursor_index;
        let num_after_cursor = line.num_after_cursor();
        let num_removed = line.remove_range(range);
        let write_after_prefix = if num_after_cursor == 0 {
            None
        } else {
            Some(line.cursor_index()..line.end_index())
        };
        LineDiff {
            move_caret_before: -(num_removed as isize),
            write_after_prefix,
            clear_after_prefix: num_removed,
            move_caret_after: -((num_removed + num_after_cursor) as isize),
        }
    }

    pub(crate) fn delete_word(&mut self) -> LineDiff {
        self.prepare_to_change_line();
        let line = self.current_line_mut();
        let old_cursor_index = line.cursor_index();
        previous_word_cursor_position(line);
        let num_removed = old_cursor_index - line.cursor_index();
        line.set_cursor_index(old_cursor_index);
        self.delete_chars(num_removed)
    }

    pub(crate) fn select_prev_line(&mut self) -> LineDiff {
        let old = &self.lines[self.selected_idx()];
        if self.offset < self.last_idx {
            self.offset += 1;
        }
        let new = &self.lines[self.selected_idx()];
        LineDiff::from(old, new)
    }

    pub(crate) fn select_next_line(&mut self) -> LineDiff {
        let old = &self.lines[self.selected_idx()];
        if self.offset > 0 {
            self.offset -= 1;
        }
        let new = &self.lines[self.selected_idx()];
        LineDiff::from(old, new)
    }

    pub(crate) fn push_history(&mut self) -> &dyn LineCursor {
        self.prepare_to_change_line();
        let line = &mut self.lines[self.selected_idx()];
        self.last_idx += 1;
        line
    }

    pub(crate) fn delete_to_end(&mut self) -> LineDiff {
        self.prepare_to_change_line();
        let line = self.current_line_mut();
        let cursor_index = line.cursor_index();
        let end_index = line.end_index();
        line.set_end_index(cursor_index);
        let num_to_clear = end_index - cursor_index;
        LineDiff {
            move_caret_before: 0,
            write_after_prefix: None,
            clear_after_prefix: num_to_clear,
            move_caret_after: -(num_to_clear as isize),
        }
    }

    pub(crate) fn cursor_to_end(&mut self) -> LineDiff {
        let num_after_cursor = self.current_line().num_after_cursor();
        self.move_cursor(num_after_cursor as isize)
    }

    pub(crate) fn cursor_to_start(&mut self) -> LineDiff {
        let move_by = -(self.current_line().cursor_index() as isize);
        self.move_cursor(move_by)
    }

    pub(crate) fn move_cursor(&mut self, by: isize) -> LineDiff {
        let move_caret = self.current_line_mut().move_cursor(by);
        LineDiff {
            move_caret_before: move_caret,
            write_after_prefix: None,
            clear_after_prefix: 0,
            move_caret_after: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{line_cursor::LineCursor, line_diff::LineDiff};

    // use super::{BufferTrait, Buffers};
    use super::Buffers;

    #[test]
    fn test_buffers_delete_to_end() {
        let mut buffers: Buffers<16, 1> = Buffers::default();
        buffers.insert_chars(b"abcdefgh");
        let diff = buffers.delete_to_end();
        assert_current_line_eq(&buffers, "abcdefgh");
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: 0,
                write_after_prefix: None,
                clear_after_prefix: 0,
                move_caret_after: 0
            }
        );

        buffers.move_cursor(-3);
        let diff = buffers.delete_to_end();
        assert_current_line_eq(&buffers, "abcde");
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: 0,
                write_after_prefix: None,
                clear_after_prefix: 3,
                move_caret_after: -3
            }
        );
    }

    #[test]
    fn test_buffers_line_selection() {
        let mut buffers: Buffers<16, 4> = Buffers::default();
        buffers.insert_chars(b"abcd");
        assert_current_line_eq(&buffers, "abcd");

        let line = buffers.push_history();
        assert_line_eq(line, "abcd");
        assert_current_line_eq(&buffers, "");

        buffers.insert_chars(b"defg");
        assert_current_line_eq(&buffers, "defg");

        let diff = buffers.select_prev_line();
        assert_current_line_eq(&buffers, "abcd");
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: -4,
                write_after_prefix: Some(0..4),
                clear_after_prefix: 0,
                move_caret_after: 0
            }
        );

        let diff = buffers.select_next_line();
        assert_current_line_eq(&buffers, "defg");
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: -4,
                write_after_prefix: Some(0..4),
                clear_after_prefix: 0,
                move_caret_after: 0
            }
        );

        let line = buffers.push_history();
        assert_line_eq(line, "defg");

        buffers.insert_chars(b"def");
        assert_current_line_eq(&buffers, "def");
        assert_eq!(buffers.current_line().cursor_index(), 3);

        let diff = buffers.select_prev_line();
        assert_current_line_eq(&buffers, "defg");
        assert_eq!(buffers.current_line().cursor_index(), 4);
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: 0,
                write_after_prefix: Some(3..4),
                clear_after_prefix: 0,
                move_caret_after: 0
            }
        );

        let diff = buffers.select_next_line();
        assert_current_line_eq(&buffers, "def");
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: -1,
                write_after_prefix: None,
                clear_after_prefix: 1,
                move_caret_after: -1
            }
        );
        buffers.push_history();

        buffers.select_prev_line();
        buffers.select_prev_line();
        assert_current_line_eq(&buffers, "defg");
        let diff = buffers.delete_chars(2);
        assert_current_line_eq(&buffers, "de");
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: -2,
                write_after_prefix: None,
                clear_after_prefix: 2,
                move_caret_after: -2
            }
        );
        buffers.push_history();
    }

    #[test]
    fn test_buffers_move_caret_fwd() {
        let mut buffers: Buffers<16, 8> = Buffers::default();

        buffers.insert_chars(b"decks");
        buffers.push_history();
        buffers.insert_chars(b"delve");
        buffers.push_history();
        buffers.insert_chars(b"foobar");
        let diff = buffers.cursor_to_start();
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: -6,
                write_after_prefix: None,
                clear_after_prefix: 0,
                move_caret_after: 0
            }
        );

        let diff = buffers.select_prev_line();
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: 0,
                write_after_prefix: Some(0..5),
                clear_after_prefix: 1,
                move_caret_after: -1
            }
        );

        let diff = buffers.select_next_line();
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: -5,
                write_after_prefix: Some(0..6),
                clear_after_prefix: 0,
                move_caret_after: -6
            }
        );
    }

    #[test]
    fn test_buffers_line_change() {
        let mut buffers: Buffers<16, 3> = Buffers::default();

        let diff = buffers.insert_chars(b"abc");
        assert_current_line_eq(&buffers, "abc");
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: 0,
                write_after_prefix: Some(0..3),
                clear_after_prefix: 0,
                move_caret_after: 0
            }
        );

        let diff = buffers.move_cursor(-1);
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: -1,
                write_after_prefix: None,
                clear_after_prefix: 0,
                move_caret_after: 0
            }
        );

        let diff = buffers.insert_chars(b"d");
        assert_current_line_eq(&buffers, "abdc");
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: 0,
                write_after_prefix: Some(2..4),
                clear_after_prefix: 0,
                move_caret_after: -1
            }
        );

        let diff = buffers.move_cursor(1);
        assert_current_line_eq(&buffers, "abdc");
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: 1,
                write_after_prefix: None,
                clear_after_prefix: 0,
                move_caret_after: 0
            }
        );

        buffers.insert_chars(b" efgh");
        assert_current_line_eq(&buffers, "abdc efgh");

        let diff = buffers.delete_word();
        assert_current_line_eq(&buffers, "abdc ");
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: -4,
                write_after_prefix: None,
                clear_after_prefix: 4,
                move_caret_after: -4
            }
        );

        let diff = buffers.delete_word();
        assert_current_line_eq(&buffers, "");
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: -5,
                write_after_prefix: None,
                clear_after_prefix: 5,
                move_caret_after: -5
            }
        );

        buffers.insert_chars(b"hello");
        assert_current_line_eq(&buffers, "hello");
        let diff = buffers.delete_chars(1);
        assert_current_line_eq(&buffers, "hell");
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: -1,
                write_after_prefix: None,
                clear_after_prefix: 1,
                move_caret_after: -1
            }
        );

        buffers.move_cursor(-2);
        assert_eq!(buffers.current_line().cursor_index(), 2);
        let diff = buffers.delete_chars(2);
        assert_current_line_eq(&buffers, "ll");
        assert_eq!(
            diff,
            LineDiff {
                move_caret_before: -2,
                write_after_prefix: Some(0..2),
                clear_after_prefix: 2,
                move_caret_after: -4
            }
        );
    }

    #[track_caller]
    fn assert_line_eq(actual: &dyn LineCursor, expected: &str) {
        let actual = actual.start_to_end();
        if actual != expected.as_bytes() {
            let actual = std::str::from_utf8(actual).unwrap();
            panic!("{:?} != {:?}", actual, expected);
        }
    }

    #[track_caller]
    fn assert_current_line_eq<const A: usize, const B: usize>(
        actual: &Buffers<A, B>,
        expected: &str,
    ) {
        assert_line_eq(actual.current_line(), expected)
    }
}
