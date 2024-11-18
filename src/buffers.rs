use crate::{
    line::{Line, LineError},
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

type LineResult = Result<LineDiff, LineError>;

impl<const MAX_LINE_LEN: usize, const MAX_LINES: usize> Buffers<MAX_LINE_LEN, MAX_LINES> {
    fn selected_idx(&self) -> usize {
        (self.last_idx - self.offset) % MAX_LINES
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

    pub fn debug(&self) {
        let start_idx = if self.last_idx < MAX_LINES {
            0
        } else {
            self.last_idx - MAX_LINES
        };

        log::info!("last_idx: {}, offset: {}", self.last_idx, self.offset);

        for idx in start_idx..self.last_idx {
            let idx = idx % MAX_LINES;
            let line = &self.lines[idx];
            log::info!(
                " - {}: {}/{} = {}",
                idx,
                line.cursor_index(),
                line.end_index(),
                core::str::from_utf8(line.start_to_end()).unwrap()
            );
        }
    }
}

impl<const MAX_LINE_LEN: usize, const MAX_LINES: usize> Buffers<MAX_LINE_LEN, MAX_LINES> {
    pub(crate) fn current_line(&self) -> &Line<MAX_LINE_LEN> {
        &self.lines[self.selected_idx()]
    }

    pub(crate) fn current_line_mut(&mut self) -> &mut Line<MAX_LINE_LEN> {
        &mut self.lines[self.selected_idx()]
    }

    pub(crate) fn insert_chars(&mut self, c: &[u8]) -> LineResult {
        self.prepare_to_change_line();
        let line = self.current_line_mut();
        let cursor_index = line.cursor_index();
        line.insert_range(cursor_index, c)?;
        let num_after_cursor = line.num_after_cursor();
        Ok(LineDiff {
            caret_back_before: 0,
            write_bytes: cursor_index..line.end_index(),
            clear_bytes: 0,
            caret_back_after: num_after_cursor,
        })
    }

    pub(crate) fn delete_chars(&mut self, n: usize) -> LineResult {
        self.prepare_to_change_line();
        let line = self.current_line_mut();
        let cursor_index = line.cursor_index();
        if cursor_index == 0 {
            return Ok(LineDiff::default());
        }

        let n = n.min(cursor_index);
        let range = (cursor_index - n)..cursor_index;
        let num_after_cursor = line.num_after_cursor();
        let num_removed = line.remove_range(range)?;
        let write_bytes = line.cursor_index()..line.end_index();
        Ok(LineDiff {
            caret_back_before: num_removed,
            write_bytes,
            clear_bytes: num_removed,
            caret_back_after: num_removed + num_after_cursor,
        })
    }

    pub(crate) fn delete_word(&mut self) -> LineResult {
        self.prepare_to_change_line();
        let line = self.current_line_mut();
        let old_cursor_index = line.cursor_index();
        previous_word_cursor_position(line);
        let num_removed = old_cursor_index - line.cursor_index();
        line.set_cursor_index(old_cursor_index);
        self.delete_chars(num_removed)
    }

    pub(crate) fn select_prev_line(&mut self) -> LineResult {
        let old = &self.lines[self.selected_idx()];
        if self.offset < self.last_idx {
            self.offset += 1;
        }
        let new = &self.lines[self.selected_idx()];
        Ok(LineDiff::from(old, new))
    }

    pub(crate) fn select_next_line(&mut self) -> LineResult {
        let old = &self.lines[self.selected_idx()];
        if self.offset > 0 {
            self.offset -= 1;
        }
        let new = &self.lines[self.selected_idx()];
        Ok(LineDiff::from(old, new))
    }

    pub(crate) fn delete_to_end(&mut self) -> LineResult {
        self.prepare_to_change_line();
        let line = self.current_line_mut();
        let cursor_index = line.cursor_index();
        let end_index = line.end_index();
        line.set_end_index(cursor_index);
        let num_to_clear = end_index - cursor_index;
        Ok(LineDiff {
            caret_back_before: 0,
            write_bytes: cursor_index..cursor_index,
            clear_bytes: num_to_clear,
            caret_back_after: num_to_clear,
        })
    }

    pub(crate) fn cursor_to_end(&mut self) -> LineResult {
        self.cursor_fwd_by(self.current_line().num_after_cursor())
    }

    pub(crate) fn cursor_to_start(&mut self) -> LineResult {
        self.cursor_back_by(self.current_line().cursor_index())
    }

    pub(crate) fn move_cursor_by(&mut self, by: isize) -> LineResult {
        if by < 0 {
            self.cursor_back_by(by.unsigned_abs())
        } else {
            self.cursor_fwd_by(by.unsigned_abs())
        }
    }

    pub(crate) fn cursor_fwd_by(&mut self, by: usize) -> LineResult {
        let line = self.current_line_mut();
        let old_cursor_index = line.cursor_index();
        let move_caret = line.move_cursor(by as isize);
        Ok(LineDiff {
            caret_back_before: 0,
            write_bytes: old_cursor_index..old_cursor_index + (move_caret as usize),
            clear_bytes: 0,
            caret_back_after: 0,
        })
    }

    pub(crate) fn cursor_back_by(&mut self, by: usize) -> LineResult {
        let line = self.current_line_mut();
        let move_caret = line.move_cursor(-(by as isize));
        Ok(LineDiff {
            caret_back_before: move_caret.unsigned_abs(),
            write_bytes: 0..0,
            clear_bytes: 0,
            caret_back_after: 0,
        })
    }

    pub(crate) fn push_history(&mut self) -> &Line<MAX_LINE_LEN> {
        self.prepare_to_change_line();

        // reset all cursors to end of lines
        for idx in self.last_idx.saturating_sub(MAX_LINES)..(self.last_idx + 1) {
            let line = &mut self.lines[idx % MAX_LINES];
            let end_index = line.end_index();
            line.set_cursor_index(end_index);
        }

        let line = &mut self.lines[self.selected_idx()];
        self.last_idx += 1;
        line
    }
}

#[cfg(test)]
mod tests {
    use core::panic;
    use std::println;

    use embedded_io_async::{ErrorType, Write};
    use futures_lite::future::block_on;

    use crate::{line::Line, make_line};

    // use super::{BufferTrait, Buffers};
    use super::{Buffers, LineResult};

    #[derive(Debug, Default)]
    struct BuffersTest<const LEN: usize> {
        buffers: Buffers<LEN, 16>,
        console: Console<LEN>,
    }
    impl<const LEN: usize> BuffersTest<LEN> {
        #[track_caller]
        fn assert_op<F>(&mut self, f: F, expected_line: &Line<LEN>)
        where
            F: FnOnce(&mut Buffers<LEN, 16>) -> LineResult,
        {
            let line_diff = f(&mut self.buffers).unwrap();
            let actual_line = self.buffers.current_line();
            println!("line diff: {:?}", line_diff);
            assert_eq!(actual_line, expected_line);
            println!("actual line: {:?}", actual_line);
            block_on(async {
                line_diff
                    .apply(&mut self.console, actual_line)
                    .await
                    .unwrap();
            });
            assert_eq!(
                self.console.cursor,
                actual_line.cursor_index(),
                "cursor index mismatch"
            );
            let (console_data, console_data_rest) =
                self.console.data.split_at(actual_line.end_index());
            assert_eq!(console_data, expected_line.start_to_end());
            assert!(
                console_data_rest.iter().all(|chr| *chr == b' '),
                "remainder of line is not all spaces: {:?}",
                console_data_rest
            );
        }

        fn push_history(&mut self) -> &Line<LEN> {
            self.console = Default::default();
            self.buffers.push_history()
        }
    }

    #[derive(Debug)]
    struct Console<const LEN: usize> {
        data: [u8; LEN],
        cursor: usize,
    }
    impl<const LEN: usize> Default for Console<LEN> {
        fn default() -> Self {
            Self {
                data: [b' '; LEN],
                cursor: 0,
            }
        }
    }
    impl<const LEN: usize> ErrorType for Console<LEN> {
        type Error = embedded_io_async::ErrorKind;
    }
    impl<const LEN: usize> Write for Console<LEN> {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            for c in buf {
                match c {
                    0x08 => {
                        self.cursor -= 1;
                    }
                    _ if !c.is_ascii_control() => {
                        self.data[self.cursor] = *c;
                        self.cursor += 1;
                    }
                    _ => panic!("unexpected char: {:#02?}", c),
                };
            }
            Ok(buf.len())
        }
    }

    #[test]
    fn test_buffers_cursor_moving() {
        let mut bt: BuffersTest<16> = BuffersTest::default();
        bt.assert_op(|b| b.insert_chars(b"abcd"), &make_line!("abcd"|));
        bt.assert_op(|b| b.cursor_to_start(), &make_line!(|"abcd"));
        bt.assert_op(|b| b.cursor_to_end(), &make_line!("abcd"|));
        bt.assert_op(|b| b.move_cursor_by(-1), &make_line!("abc" | "d"));
        bt.assert_op(|b| b.move_cursor_by(2), &make_line!("abcd"|));
        bt.assert_op(|b| b.move_cursor_by(-3), &make_line!("a" | "bcd"));
        bt.assert_op(|b| b.move_cursor_by(-2), &make_line!(|"abcd"));

        bt.assert_op(|b| b.move_cursor_by(1), &make_line!("a" | "bcd"));
        bt.assert_op(|b| b.delete_chars(1), &make_line!(|"bcd"));
        bt.assert_op(|b| b.insert_chars(b"012 "), &make_line!("012 " | "bcd"));
        bt.assert_op(|b| b.delete_word(), &make_line!(|"bcd"));

        bt.assert_op(|b| b.insert_chars(b"012 "), &make_line!("012 " | "bcd"));
        bt.assert_op(|b| b.move_cursor_by(-1), &make_line!("012" | " bcd"));
        bt.assert_op(|b| b.delete_word(), &make_line!(|" bcd"));

        bt.assert_op(|b| b.insert_chars(b"012"), &make_line!("012" | " bcd"));
        bt.assert_op(|b| b.move_cursor_by(3), &make_line!("012 bc" | "d"));
        bt.assert_op(|b| b.delete_word(), &make_line!("012 " | "d"));
    }

    #[test]
    fn test_buffers_delete_to_end() {
        let mut bt: BuffersTest<16> = BuffersTest::default();
        bt.assert_op(|b| b.insert_chars(b"abcd"), &make_line!("abcd"|));
        bt.assert_op(|b| b.move_cursor_by(-3), &make_line!("a" | "bcd"));
        bt.assert_op(|b| b.delete_to_end(), &make_line!("a"|));
        bt.assert_op(|b| b.move_cursor_by(-1), &make_line!(|"a"));
        bt.assert_op(|b| b.delete_to_end(), &make_line!(|));
    }

    #[test]
    fn test_buffers_line_selection() {
        let mut bt: BuffersTest<16> = BuffersTest::default();
        bt.assert_op(|b| b.insert_chars(b"abcd"), &make_line!("abcd"|));
        assert_eq!(bt.push_history(), &make_line!("abcd"|));
        bt.assert_op(|b| b.insert_chars(b"efgh"), &make_line!("efgh"|));
        bt.assert_op(|b| b.move_cursor_by(-1), &make_line!("efg" | "h"));
        bt.assert_op(|b| b.select_prev_line(), &make_line!("abcd"|));
        bt.assert_op(|b| b.move_cursor_by(-2), &make_line!("ab" | "cd"));
        bt.assert_op(|b| b.select_next_line(), &make_line!("efg" | "h"));
        bt.assert_op(|b| b.select_prev_line(), &make_line!("ab" | "cd"));
        bt.assert_op(|b| b.insert_chars(b"1"), &make_line!("ab1" | "cd"));
        bt.assert_op(|b| b.select_next_line(), &make_line!("ab1" | "cd"));
        assert_eq!(bt.push_history(), &make_line!("ab1cd"|));
        bt.assert_op(|b| b.select_prev_line(), &make_line!("ab1cd"|));
    }
}
