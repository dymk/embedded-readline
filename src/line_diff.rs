use core::cmp::Ordering;

use embedded_io_async as eia;

use crate::line::Line;

#[derive(Debug, PartialEq)]
pub(crate) struct LineDiff {
    pub move_caret_before: isize,
    pub write_after_prefix: Option<core::ops::Range<usize>>,
    pub clear_after_prefix: usize,
    pub move_caret_after: isize,
}

impl LineDiff {
    pub fn from<const LEN: usize>(old_line: &Line<LEN>, new_line: &Line<LEN>) -> Self {
        calc_line_diff(old_line, new_line)
    }

    pub async fn apply<Writer, Error, const LEN: usize>(
        self,
        writer: &mut Writer,
        new_line: &Line<LEN>,
    ) -> Result<(), Error>
    where
        Writer: eia::Write<Error = Error>,
        Error: eia::Error,
    {
        let cursor_index = new_line.cursor_index();
        let line_data = new_line.start_to_end();

        match self.move_caret_before.cmp(&0) {
            Ordering::Less => {
                let move_caret = self.move_caret_before.unsigned_abs();
                for _ in 0..move_caret {
                    writer.write(&[0x08]).await?;
                }
            }
            Ordering::Greater => {
                let move_caret = self.move_caret_before.unsigned_abs();
                let range_to_write = cursor_index..(cursor_index + move_caret);
                let data_to_write = &line_data[range_to_write];
                writer.write(data_to_write).await?;
            }
            _ => {}
        }

        if let Some(write_after_prefix) = self.write_after_prefix {
            let write_after_prefix = &line_data[write_after_prefix.clone()];
            writer.write(write_after_prefix).await?;
        }

        for _ in 0..self.clear_after_prefix {
            writer.write(b" ").await?;
        }

        if self.move_caret_after <= 0 {
            for _ in 0..self.move_caret_after.abs() {
                writer.write(&[0x08]).await?;
            }
        } else {
            panic!("invariant: caret after move cannot be positive");
        }

        Ok(())
    }
}

fn calc_line_diff<const LEN: usize>(old_line: &Line<LEN>, new_line: &Line<LEN>) -> LineDiff {
    let old_data = old_line.start_to_end();
    let new_data = new_line.start_to_end();

    // find the common prefix between the two lines
    let mut prefix_length = 0;
    for (old, new) in old_data.iter().zip(new_data.iter()) {
        if old != new {
            break;
        }
        prefix_length += 1;
    }

    let (write_after_prefix, clear_after_prefix) =
        if old_data.len() == new_data.len() && prefix_length == new_data.len() {
            (None, 0)
        } else if old_data.len() > new_data.len() {
            if prefix_length == new_data.len() {
                (None, old_data.len() - prefix_length)
            } else {
                (
                    Some(prefix_length..new_data.len()),
                    old_data.len() - new_data.len(),
                )
            }
        } else {
            (Some(prefix_length..new_data.len()), 0)
        };

    let old_line_index = old_line.cursor_index() as isize;
    let move_caret_before = (prefix_length as isize) - old_line_index;
    let cursor_moved_by = new_line.cursor_index() as isize - old_line.cursor_index() as isize;

    let write_after_prefix_len = write_after_prefix
        .as_ref()
        .map(|range| range.len())
        .unwrap_or(0);

    let move_caret_after =
        -(move_caret_before + write_after_prefix_len as isize + clear_after_prefix as isize)
            + cursor_moved_by;

    LineDiff {
        move_caret_before,
        write_after_prefix,
        clear_after_prefix,
        move_caret_after,
    }
}

#[cfg(test)]
mod tests {
    use crate::{line::Line, line_diff::LineDiff, test_reader_writer::TestReaderWriter};

    #[test]
    fn test_calc_diff1() {
        let mut old_line: Line<10> = Line::from_u8(b"hello");
        let mut new_line: Line<10> = Line::from_u8(b"heck");
        old_line.set_cursor_index(0);
        new_line.set_cursor_index(0);
        let result = LineDiff::from(&old_line, &new_line);
        assert_eq!(
            result,
            LineDiff {
                move_caret_before: 2,
                write_after_prefix: Some(2..4),
                clear_after_prefix: 1,
                move_caret_after: -5
            }
        );
    }

    #[rstest::rstest]
    #[case(LineDiff {
        move_caret_before: 2,
        write_after_prefix: Some(2..4),
        clear_after_prefix: 1,
        move_caret_after: -5
    }, 0, b"heck", b"heck \x08\x08\x08\x08\x08")]
    #[case(LineDiff {
        move_caret_before: 0,
        write_after_prefix: Some(2..4),
        clear_after_prefix: 1,
        move_caret_after: -5
    }, 2, b"heck", b"ck \x08\x08\x08\x08\x08")]
    async fn test_apply(
        #[case] line_diff: LineDiff,
        #[case] cursor_index: usize,
        #[case] new_line: &[u8],
        #[case] data_to_write: &[u8],
    ) {
        let mut new_line = Line::<10>::from_u8(new_line);
        new_line.set_cursor_index(cursor_index);
        let mut writer = TestReaderWriter::new(&[]);
        let ok = line_diff.apply(&mut writer, &new_line).await;
        assert_eq!(ok, Ok(()));
        assert_eq!(writer.data_to_write, data_to_write);
    }

    #[rstest::rstest]
    #[case(b"hello", b"hello", 0, None, 0, 0)]
    #[case(b"hello", b"hello!",  0, Some(5..6), 0,  0)]
    #[case(b"",      b"hi",      0, Some(0..2), 0,  0)]
    #[case(b"hello", b"he",     -3, None,       3, -3)]
    #[case(b"hello", b"heck",   -3, Some(2..4), 1, -1)]
    fn test_calc_diff(
        #[case] old_data: &[u8],
        #[case] new_data: &[u8],
        #[case] move_caret_before: isize,
        #[case] write_after_prefix: Option<core::ops::Range<usize>>,
        #[case] clear_after_prefix: usize,
        #[case] move_caret_after: isize,
    ) {
        let old_line: Line<10> = Line::from_u8(old_data);
        let new_line: Line<10> = Line::from_u8(new_data);

        let result = LineDiff::from(&old_line, &new_line);
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
