use embedded_io_async as eia;

use crate::line::Line;

#[derive(Debug, PartialEq, Default)]
pub(crate) struct LineDiff {
    pub caret_back_before: usize,
    pub write_bytes: core::ops::Range<usize>,
    pub clear_bytes: usize,
    pub caret_back_after: usize,
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
        let line_data = new_line.start_to_end();

        for _ in 0..self.caret_back_before {
            writer.write_all(&[0x08]).await?;
        }

        let data = &line_data[self.write_bytes.clone()];
        writer.write_all(data).await?;

        for _ in 0..self.clear_bytes {
            writer.write_all(b" ").await?;
        }

        for _ in 0..self.caret_back_after {
            writer.write_all(&[0x08]).await?;
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

    let caret_back_before = if prefix_length < old_line.cursor_index() {
        old_line.cursor_index() - prefix_length
    } else {
        0
    };

    let current_index = old_line.cursor_index() - caret_back_before;
    let write_bytes = current_index..new_line.end_index();
    let clear_bytes = if new_line.end_index() < old_line.end_index() {
        old_line.end_index() - new_line.end_index()
    } else {
        0
    };

    let current_index = current_index + write_bytes.len() + clear_bytes;
    let caret_back_after = current_index - new_line.cursor_index();

    LineDiff {
        caret_back_before,
        write_bytes,
        clear_bytes,
        caret_back_after,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        line::Line, line_diff::LineDiff, make_line, test_reader_writer::TestReaderWriter,
        util::assert_eq_u8,
    };

    #[rstest::rstest]
    #[case(
        make_line!(|""),
        make_line!(|""),
        LineDiff {
            caret_back_before: 0,
            write_bytes: 0..0,
            clear_bytes: 0,
            caret_back_after: 0
        },
        ""
    )]
    #[case(
        make_line!(|"hello"),
        make_line!(|"heck"),
        LineDiff {
            caret_back_before: 0,
            write_bytes: 0..4,
            clear_bytes: 1,
            caret_back_after: 5
        },
        "heck \x08\x08\x08\x08\x08"
    )]
    #[case(
        make_line!("hel"|"lo"),
        make_line!(|"heck"),
        LineDiff {
            caret_back_before: 1,
            write_bytes: 2..4,
            clear_bytes: 1,
            caret_back_after: 5
        },
        "\x08ck \x08\x08\x08\x08\x08"
    )]
    #[case(
        make_line!("he"|"ck!"),
        make_line!(|"heck"),
        LineDiff {
            caret_back_before: 0,
            write_bytes: 2..4,
            clear_bytes: 1,
            caret_back_after: 5
        },
        "ck \x08\x08\x08\x08\x08"
    )]
    async fn test_line_diff(
        #[case] old_line: Line<8>,
        #[case] new_line: Line<8>,
        #[case] expected_line_diff: LineDiff,
        #[case] expected_apply: &str,
    ) {
        let actual_line_diff = LineDiff::from(&old_line, &new_line);
        assert_eq!(actual_line_diff, expected_line_diff);

        let mut writer = TestReaderWriter::new(&[]);
        let ok = actual_line_diff.apply(&mut writer, &new_line).await;
        assert_eq!(ok, Ok(()));
        assert_eq_u8(&writer.data_to_write, expected_apply);
    }
}
