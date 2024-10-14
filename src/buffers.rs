use crate::util::get_two_mut_checked;

#[derive(Copy, Clone, Debug)]
struct Line<const LEN: usize> {
    data: [u8; LEN],
    cursor_index: usize,
    end_index: usize,
}

impl<const LEN: usize> Default for Line<LEN> {
    fn default() -> Self {
        Self {
            data: [0; LEN],
            cursor_index: 0,
            end_index: 0,
        }
    }
}

pub(crate) trait LineCursor {
    fn data(&self) -> &[u8];
    fn cursor_index(&self) -> usize;
    fn end_index(&self) -> usize;

    fn start_to_cursor(&self) -> &[u8] {
        &self.data()[..self.cursor_index()]
    }
    fn cursor_to_end(&self) -> &[u8] {
        &self.data()[self.cursor_index()..self.end_index()]
    }
    fn start_to_end(&self) -> &[u8] {
        &self.data()[..self.end_index()]
    }
    fn num_after_cursor(&self) -> usize {
        self.end_index() - self.cursor_index()
    }
    fn copy_from(&mut self, from: &dyn LineCursor) {
        self.data_mut().copy_from_slice(from.data());
        *self.cursor_index_mut() = from.cursor_index();
        *self.end_index_mut() = from.end_index();
    }
    fn data_mut(&mut self) -> &mut [u8];
    fn cursor_index_mut(&mut self) -> &mut usize;
    fn end_index_mut(&mut self) -> &mut usize;
}

impl<const A: usize> LineCursor for Line<A> {
    fn data(&self) -> &[u8] {
        &self.data
    }

    fn cursor_index(&self) -> usize {
        self.cursor_index
    }

    fn end_index(&self) -> usize {
        self.end_index
    }

    fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    fn cursor_index_mut(&mut self) -> &mut usize {
        &mut self.cursor_index
    }

    fn end_index_mut(&mut self) -> &mut usize {
        &mut self.end_index
    }
}

impl core::fmt::Debug for dyn LineCursor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LineCursor")
            .field("cursor_index", &self.cursor_index())
            .field("end_index", &self.end_index())
            .field("data", &self.data())
            .finish()
    }
}

#[derive(Debug)]
pub struct Buffers<const MAX_LINE_LEN: usize, const MAX_LINES: usize> {
    lines: [Line<MAX_LINE_LEN>; MAX_LINES],
    last_line_idx: usize,
    current_line_offset: usize,
}

impl<const A: usize, const B: usize> Buffers<A, B> {
    fn current_line_idx(&self) -> usize {
        (self.last_line_idx - self.current_line_offset) % B
    }

    pub fn debug(&self) {
        log::info!(
            "buffer offsets ({}): {} - {}",
            B,
            self.last_line_idx,
            self.current_line_offset
        );
        for l in &self.lines {
            log::info!(
                "- line: ({}, {}) {:?}",
                l.cursor_index(),
                l.end_index(),
                l.start_to_cursor()
            )
        }
    }
}

impl<const MAX_LINE_LEN: usize, const MAX_LINES: usize> Default
    for Buffers<MAX_LINE_LEN, MAX_LINES>
{
    fn default() -> Self {
        if MAX_LINES == 0 {
            panic!("MAX_LINES must be greater than 0");
        }

        Self {
            lines: [Line::default(); MAX_LINES],
            last_line_idx: 0,
            current_line_offset: 0,
        }
    }
}

pub(crate) trait BufferTrait {
    fn current_line_mut(&mut self) -> &mut dyn LineCursor;
    fn current_line(&self) -> &dyn LineCursor;

    fn select_prev_line(&mut self);
    fn select_next_line(&mut self);
    fn push_history(&mut self) -> &dyn LineCursor;
}

impl<const A: usize, const B: usize> BufferTrait for Buffers<A, B> {
    fn current_line_mut(&mut self) -> &mut dyn LineCursor {
        &mut self.lines[self.current_line_idx()]
    }

    fn current_line(&self) -> &dyn LineCursor {
        &self.lines[self.current_line_idx()]
    }

    fn select_prev_line(&mut self) {
        if self.current_line_offset == self.last_line_idx {
            return;
        }
        self.current_line_offset += 1;
    }

    fn select_next_line(&mut self) {
        if self.current_line_offset == 0 {
            return;
        }
        self.current_line_offset -= 1;
    }

    fn push_history(&mut self) -> &dyn LineCursor {
        let to_idx = self.last_line_idx % B;

        if self.current_line_offset > 0 {
            let from_idx = (self.last_line_idx - self.current_line_offset) % B;
            let (from_line, to_line) =
                get_two_mut_checked(from_idx, to_idx, &mut self.lines).unwrap();
            to_line.copy_from(from_line);
        }

        // reset offsets and go to the next line
        self.current_line_offset = 0;
        self.last_line_idx += 1;

        // clear the next line
        self.lines[self.last_line_idx % B].cursor_index = 0;
        self.lines[self.last_line_idx % B].end_index = 0;

        // return the line that was just buffered
        &self.lines[to_idx]
    }
}

#[cfg(test)]
mod tests {
    use super::{BufferTrait, Buffers, LineCursor};

    fn set_line_data(line: &mut dyn LineCursor, data: &[u8]) {
        let data_len = data.len();
        line.data_mut()[0..data_len].copy_from_slice(data);
        *line.cursor_index_mut() = data_len;
        *line.end_index_mut() = data_len;
    }

    #[test]
    fn test_history() {
        let mut buffers: Buffers<2, 2> = Buffers::default();
        set_line_data(&mut buffers.lines[0], b"a0");
        set_line_data(&mut buffers.lines[1], b"b1");

        let current_line = buffers.current_line();
        assert_eq!(current_line.data(), b"a0");
        assert_eq!(current_line.cursor_index(), 2);
        assert_eq!(current_line.end_index(), 2);

        let line = buffers.push_history();
        assert_eq!(line.data(), b"a0");

        let current_line = buffers.current_line();
        assert_eq!(current_line.data(), b"b1");
        assert_eq!(current_line.cursor_index(), 0);
        assert_eq!(current_line.end_index(), 0);

        buffers.select_prev_line();
        assert_eq!(buffers.current_line().data(), b"a0");
        buffers.select_prev_line();
        assert_eq!(buffers.current_line().data(), b"a0");
        buffers.select_next_line();
        assert_eq!(buffers.current_line().data(), b"b1");
        buffers.select_next_line();
        assert_eq!(buffers.current_line().data(), b"b1");
    }
}
