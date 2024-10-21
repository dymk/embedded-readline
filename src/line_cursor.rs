pub(crate) trait LineCursor {
    fn set_from_u8(&mut self, data: &[u8]);

    fn cursor_index(&self) -> usize;
    fn end_index(&self) -> usize;

    fn start_to_cursor(&self) -> &[u8];
    fn cursor_to_end(&self) -> &[u8];
    fn start_to_end(&self) -> &[u8];

    fn num_after_cursor(&self) -> usize {
        self.end_index() - self.cursor_index()
    }
    fn set_from_cursor(&mut self, from: &dyn LineCursor);

    fn set_cursor_index(&mut self, cursor_index: usize);
    fn set_end_index(&mut self, end_index: usize);

    fn move_cursor(&mut self, by: isize) -> isize;
    fn at_cursor(&self, by: isize) -> Option<u8>;
    fn clear(&mut self) {
        self.set_cursor_index(0);
        self.set_end_index(0);
    }

    fn remove_range(&mut self, range: core::ops::Range<usize>) -> usize;
    fn insert_range(&mut self, at: usize, data: &[u8]) -> usize;
}

impl core::fmt::Debug for dyn LineCursor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LineCursor")
            .field("cursor_index", &self.cursor_index())
            .field("end_index", &self.end_index())
            .field("data", &self.start_to_cursor())
            .finish()
    }
}
