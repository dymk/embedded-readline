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
    fn num_after_cursor(&self) -> usize {
        self.end_index() - self.cursor_index()
    }
}

pub(crate) trait LineCursorMut: LineCursor {
    fn data_mut(&mut self) -> &mut [u8];
    fn cursor_index_mut(&mut self) -> &mut usize;
    fn end_index_mut(&mut self) -> &mut usize;

    fn cursor_to_end_mut(&mut self) -> &mut [u8] {
        let cursor_index = self.cursor_index();
        let end_index = self.end_index();
        &mut self.data_mut()[cursor_index..end_index]
    }
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
}

impl<const A: usize> LineCursorMut for Line<A> {
    fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    fn cursor_index_mut(&mut self) -> &mut usize {
        &mut self.cursor_index
    }

    fn end_index_mut(&mut self) -> &mut usize {
        &mut self.end_index
    }

    fn cursor_to_end_mut(&mut self) -> &mut [u8] {
        &mut self.data[self.cursor_index..self.end_index]
    }
}

#[derive(Debug)]
pub struct Buffers<const MAX_LINE_LEN: usize, const MAX_LINES: usize> {
    pub lines: [Line<MAX_LINE_LEN>; MAX_LINES],
    pub lines_tail_idx: usize,
    pub history_offset: usize,
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
            lines_tail_idx: 0,
            history_offset: 0,
        }
    }
}

pub(crate) trait BufferTrait {
    fn current_line_mut(&mut self) -> &mut dyn LineCursorMut;
    fn current_line(&self) -> &dyn LineCursor;
    fn clear_current_line(&mut self);
}

impl<const A: usize, const B: usize> BufferTrait for Buffers<A, B> {
    fn current_line_mut(&mut self) -> &mut dyn LineCursorMut {
        &mut self.lines[(self.lines_tail_idx + self.history_offset) % B]
    }

    fn current_line(&self) -> &dyn LineCursor {
        &self.lines[(self.lines_tail_idx + self.history_offset) % B]
    }

    fn clear_current_line(&mut self) {
        let line = self.current_line_mut();
        *line.cursor_index_mut() = 0;
        *line.end_index_mut() = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::{BufferTrait, Buffers};

    #[test]
    fn test_history() {
        let mut buffers: Buffers<8, 2> = Buffers::default();
        assert_eq!(buffers.current_line().data(), [0; 8]);
        buffers.clear_current_line();
        assert_eq!(buffers.current_line().data(), [0; 8]);
    }
}
