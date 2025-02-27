use core::fmt::Debug;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LineError {
    OutOfBounds,
}

#[derive(Copy, Clone)]
pub(crate) struct Line<const LEN: usize> {
    data: [u8; LEN],
    cursor_index: usize,
    end_index: usize,
}

impl<const LEN: usize> PartialEq for Line<LEN> {
    fn eq(&self, other: &Self) -> bool {
        self.start_to_end() == other.start_to_end()
            && self.cursor_index == other.cursor_index
            && self.end_index == other.end_index
    }
}

impl<const LEN: usize> Debug for Line<LEN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Line")
            .field("data", &core::str::from_utf8(self.start_to_end()).unwrap())
            .field("cursor_index", &self.cursor_index)
            .field("end_index", &self.end_index)
            .field(
                "after_end",
                &core::str::from_utf8(&self.data[self.end_index..LEN]).unwrap(),
            )
            .finish()
    }
}

impl<const LEN: usize> Line<LEN> {
    #[cfg(test)]
    pub fn from_u8(data: &[u8]) -> Self {
        let mut line = Line::default();
        line.set_from_u8(data);
        line
    }
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

impl<const A: usize> Line<A> {
    #[cfg(test)]
    pub(crate) fn start_to_cursor(&self) -> &[u8] {
        &self.data[..self.cursor_index]
    }

    pub(crate) fn start_to_end(&self) -> &[u8] {
        &self.data[..self.end_index]
    }

    pub(crate) fn num_after_cursor(&self) -> usize {
        self.end_index() - self.cursor_index()
    }

    pub(crate) fn cursor_index(&self) -> usize {
        self.cursor_index
    }

    pub(crate) fn end_index(&self) -> usize {
        self.end_index
    }

    pub(crate) fn insert_range(&mut self, at: usize, data: &[u8]) -> Result<usize, LineError> {
        let space_remaining = A - self.end_index;
        if data.len() > space_remaining {
            return Err(LineError::OutOfBounds);
        }

        let last_idx = at + data.len();
        if last_idx > A {
            return Err(LineError::OutOfBounds);
        }

        let data_len = data.len();
        for i in (at..self.end_index).rev() {
            self.data[i + data_len] = self.data[i];
        }
        self.data[at..(at + data_len)].copy_from_slice(data);
        self.end_index += data_len;
        if at <= self.cursor_index {
            self.cursor_index += data_len;
        }
        Ok(data_len)
    }

    pub(crate) fn remove_range(
        &mut self,
        range: core::ops::Range<usize>,
    ) -> Result<usize, LineError> {
        // remove chars from [range.start to range.end)
        if range.end > self.end_index {
            return Err(LineError::OutOfBounds);
        };

        self.data
            .copy_within(range.end..self.end_index, range.start);
        self.end_index -= range.len();
        if range.end <= self.cursor_index {
            self.cursor_index -= range.len();
        } else if range.start <= self.cursor_index {
            self.cursor_index = range.start;
        }

        Ok(range.len())
    }

    #[cfg(test)]
    pub(crate) fn set_from_u8(&mut self, data: &[u8]) {
        let data_len = data.len();
        self.data[0..data_len].copy_from_slice(data);
        self.cursor_index = data_len;
        self.end_index = data_len;
    }

    pub(crate) fn set_from_cursor(&mut self, from: &Self) {
        let data = from.start_to_end();
        self.data[..data.len()].copy_from_slice(data);
        self.cursor_index = from.cursor_index();
        self.end_index = from.end_index();
    }

    pub(crate) fn set_cursor_index(&mut self, cursor_index: usize) {
        self.cursor_index = cursor_index;
    }

    pub(crate) fn set_end_index(&mut self, end_index: usize) {
        self.end_index = end_index;
    }

    pub(crate) fn move_cursor(&mut self, by: isize) -> isize {
        let cursor_index = self.cursor_index as isize;
        let end_index = self.end_index as isize;
        let new_cursor_index = (cursor_index + by).max(0).min(end_index);
        let move_by = new_cursor_index - self.cursor_index as isize;
        self.cursor_index = new_cursor_index as usize;
        move_by
    }

    pub(crate) fn at_cursor(&self, by: isize) -> Option<u8> {
        let idx = self.cursor_index as isize + by;
        if idx < 0 || idx > (self.end_index as isize) {
            None
        } else {
            Some(self.data[idx as usize])
        }
    }

    pub(crate) fn clear(&mut self) {
        self.set_cursor_index(0);
        self.set_end_index(0);
    }
}

#[cfg(test)]
mod tests {
    use crate::line::LineError;

    use super::Line;

    fn make_line() -> Line<10> {
        let mut l = Line::default();
        l.set_from_u8(b"hello");
        l
    }

    macro_rules! assert_line_eq {
        ($line:ident, $start_to_end:literal, $cursor_index:literal, $end_index:literal) => {
            assert_eq!($line.start_to_end(), $start_to_end);
            assert_eq!($line.cursor_index(), $cursor_index);
            assert_eq!($line.end_index(), $end_index);
        };
    }

    #[test]
    fn test_line_overflow() {
        let mut line: Line<0> = Line::default();
        assert_eq!(line.insert_range(0, b""), Ok(0));
        assert_eq!(line.insert_range(1, b""), Err(LineError::OutOfBounds));
        assert_eq!(line.insert_range(0, b"a"), Err(LineError::OutOfBounds));

        let mut line: Line<4> = Line::default();
        assert_eq!(line.insert_range(0, b"heck"), Ok(4));
        assert_eq!(line.start_to_end(), b"heck");
        assert_eq!(line.insert_range(0, b""), Ok(0));
        assert_eq!(line.insert_range(4, b""), Ok(0));
        assert_eq!(line.insert_range(5, b""), Err(LineError::OutOfBounds));
    }

    #[test]
    fn test_line_insert_range() {
        let line = make_line();
        assert_line_eq!(line, b"hello", 5, 5);

        let mut line = make_line();
        assert_eq!(line.insert_range(2, b"ab"), Ok(2));
        assert_line_eq!(line, b"heabllo", 7, 7);

        let mut line = make_line();
        line.set_cursor_index(2);
        assert_eq!(line.insert_range(2, b"ab"), Ok(2));
        assert_line_eq!(line, b"heabllo", 4, 7);

        let mut line = make_line();
        line.insert_range(0, b"ab").unwrap();
        line.set_cursor_index(0);
        assert_line_eq!(line, b"abhello", 0, 7);

        let mut line = make_line();
        line.insert_range(0, b"").unwrap();
        assert_line_eq!(line, b"hello", 5, 5);
    }

    #[test]
    fn test_line_remove_range() {
        let mut line = make_line();
        assert_eq!(line.remove_range(0..0), Ok(0));
        assert_line_eq!(line, b"hello", 5, 5);

        line.remove_range(0..1).unwrap();
        assert_line_eq!(line, b"ello", 4, 4);

        let mut line = make_line();
        line.remove_range(2..4).unwrap();
        assert_line_eq!(line, b"heo", 3, 3);

        let mut line = make_line();
        line.remove_range(2..5).unwrap();
        assert_line_eq!(line, b"he", 2, 2);

        let mut line = make_line();
        line.set_cursor_index(3);
        line.remove_range(2..4).unwrap();
        assert_line_eq!(line, b"heo", 2, 3);

        let mut line = make_line();
        line.set_cursor_index(2);
        line.remove_range(2..4).unwrap();
        assert_line_eq!(line, b"heo", 2, 3);

        let mut line: Line<0> = Line::default();
        assert_eq!(line.remove_range(0..0), Ok(0));
    }
}
