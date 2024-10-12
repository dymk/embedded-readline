use core::cell::{RefCell, RefMut};

use embedded_io_async as eia;

use crate::{buffers::BufferTrait, Buffers};

/// Reads a line from the given UART interface into the provided buffer asynchronously.
///
/// This function reads bytes from the `uart` interface until it encounters a newline (`\n`) or
/// carriage return (`\r`) character. The read bytes are stored in the provided buffer `buf`.
///
/// # Arguments
///
/// * `uart` - A mutable reference to the UART interface implementing the `embedded_io_async::Read` trait.
/// * `buf` - A mutable reference to a byte slice where the read line will be stored.
///
/// # Returns
///
/// Returns a `Result` containing a slice of the buffer with the read line on success, or an error
/// of type `Error` on failure.
///
/// # Type Parameters
///
/// * `Error` - The error type that implements the `embedded_io_async::Error` trait.
/// * `Read` - The UART interface type that implements the `embedded_io_async::Read` trait with the
///   associated `Error` type.

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum ReadlineStatus {
    // Reading normal characters and writing to the buffer
    Char,
    // Just read an ESC character
    Escape,
    // Just read an ESC + [
    Ctrl,
}

#[derive(Debug, PartialEq)]
pub enum ReadlineError<Error> {
    ReaderWriterError(Error),
    BufferFullError,
    UnexpectedEscape,
    UnexpectedCtrl,
    UnexpectedChar(u8),
}
impl<Error> From<Error> for ReadlineError<Error> {
    fn from(e: Error) -> Self {
        ReadlineError::ReaderWriterError(e)
    }
}

enum Loop {
    Continue,
    Break,
}

struct State<'u, 'b, ReaderWriter> {
    uart: RefCell<&'u mut ReaderWriter>,
    buffers: RefCell<Option<&'b mut dyn BufferTrait>>,
    status: ReadlineStatus,
}

impl<'u, 'b, ReaderWriter, Error> State<'u, 'b, ReaderWriter>
where
    ReaderWriter: eia::Read<Error = Error> + eia::Write<Error = Error>,
    Error: eia::Error,
{
    async fn readline(mut self) -> Result<&'b [u8], ReadlineError<Error>> {
        self.borrow_buffers_mut().clear_current_line();

        loop {
            let byte = self.read_byte().await?;
            if matches!(self.process_byte(byte).await?, Loop::Break) {
                break;
            }
        }

        let buffers = self.buffers.take().unwrap();
        let line = buffers.current_line();
        Ok(line.start_to_cursor())
    }

    // fn borrow_buffers(&self) -> Ref<dyn BufferTrait> {
    //     Ref::map(self.buffers.borrow(), |opt| opt.as_deref().unwrap())
    // }

    fn borrow_buffers_mut(&self) -> RefMut<dyn BufferTrait> {
        RefMut::map(self.buffers.borrow_mut(), |opt| opt.as_deref_mut().unwrap())
    }

    async fn process_byte(&mut self, byte: u8) -> Result<Loop, ReadlineError<Error>> {
        match (byte, self.status) {
            (b'\n', _) | (b'\r', _) => return Ok(Loop::Break),
            // ESC = 0x1B
            (0x1B, ReadlineStatus::Char) => {
                self.status = ReadlineStatus::Escape;
            }
            (0x1B, _) => {
                return Err(ReadlineError::UnexpectedEscape);
            }
            (b'[', ReadlineStatus::Escape) => {
                self.status = ReadlineStatus::Ctrl;
            }
            (b'[', _) => {
                return Err(ReadlineError::UnexpectedCtrl);
            }
            (0x08, ReadlineStatus::Char) | (0x7F, ReadlineStatus::Char) => {
                self.handle_backspace().await?;
            }
            (0x01, ReadlineStatus::Char) => {
                // go to the beginning of the line
                // let mut buffers = self.borrow_buffers_mut();
                let mut buffers = self.borrow_buffers_mut();
                let line = buffers.current_line_mut();
                self.write_caret_back(line.cursor_index()).await?;
                *line.cursor_index_mut() = 0;
            }
            (0x05, ReadlineStatus::Char) => {
                // go to the end of the line
                let mut buffers = self.borrow_buffers_mut();
                let line = buffers.current_line_mut();
                self.write_bytes(line.cursor_to_end()).await?;
                *line.cursor_index_mut() = line.end_index();
            }
            (0x0B, ReadlineStatus::Char) => {
                // delete to end of line
                let mut buffers = self.borrow_buffers_mut();
                let line = buffers.current_line_mut();
                let num_deleted = line.num_after_cursor();
                *line.end_index_mut() = line.cursor_index();
                self.write_spaces(num_deleted).await?;
                self.write_caret_back(num_deleted).await?;
                *line.end_index_mut() = line.cursor_index();
            }
            (0x17, ReadlineStatus::Char) => {
                self.handle_delete_word().await?;
            }
            (byte, ReadlineStatus::Char) => {
                // other printable chars
                self.handle_char(byte).await?;
            }
            (byte, ReadlineStatus::Escape) => {
                return Err(ReadlineError::UnexpectedChar(byte));
            }
            (byte, ReadlineStatus::Ctrl) => {
                self.handle_control(byte).await?;
                self.status = ReadlineStatus::Char;
            }
        }

        Ok(Loop::Continue)
    }

    async fn handle_delete_word(&mut self) -> Result<(), ReadlineError<Error>> {
        // delete to previous word's end
        let mut buffers = self.borrow_buffers_mut();
        let line = buffers.current_line_mut();
        let old_cursor = line.cursor_index();

        // find the most recent word
        while line.cursor_index() > 0 && line.data()[line.cursor_index() - 1].is_ascii_whitespace()
        {
            *line.cursor_index_mut() -= 1;
        }

        // find the start of the word
        while line.cursor_index() > 0 && !line.data()[line.cursor_index() - 1].is_ascii_whitespace()
        {
            *line.cursor_index_mut() -= 1;
        }

        // delete all the chars between the two
        let num_deleted = old_cursor - line.cursor_index();
        *line.end_index_mut() -= num_deleted;
        let num_after_cursor = line.end_index() - line.cursor_index();

        for i in 0..num_deleted {
            let idx = line.cursor_index() + i;
            line.data_mut()[idx] = line.data()[old_cursor + i];
        }

        // move the caret back & write out the moved letters
        self.write_caret_back(num_deleted).await?;
        self.write_bytes(line.cursor_to_end()).await?;
        self.write_spaces(num_deleted).await?;
        self.write_caret_back(num_deleted + num_after_cursor)
            .await?;

        Ok(())
    }

    async fn handle_backspace(&mut self) -> Result<(), ReadlineError<Error>> {
        let mut buffers = self.borrow_buffers_mut();
        let line = buffers.current_line_mut();

        if line.cursor_index() == 0 {
            return Ok(());
        }

        // move all the bytes after the cursor to the left
        for i in line.cursor_index()..line.end_index() {
            line.data_mut()[i - 1] = line.data()[i];
        }
        *line.end_index_mut() -= 1;
        *line.cursor_index_mut() -= 1;
        let num_after_cursor = line.end_index() - line.cursor_index();

        // send all the chars after the cursor to the terminal
        self.write_byte(0x08).await?;
        self.write_bytes(line.cursor_to_end()).await?;
        self.write_byte(b' ').await?;
        self.write_caret_back(num_after_cursor + 1).await?;

        Ok(())
    }

    /// Handle a character byte, put a character in the buffer and move the cursor
    async fn handle_char(&mut self, byte: u8) -> Result<(), ReadlineError<Error>> {
        let mut buffers = self.borrow_buffers_mut();
        let line = buffers.current_line_mut();

        // move all the bytes after the cursor to the right
        for i in (line.cursor_index()..line.end_index()).rev() {
            line.data_mut()[i + 1] = line.data()[i];
        }
        let idx = line.cursor_index();
        line.data_mut()[idx] = byte;
        *line.end_index_mut() += 1;
        *line.cursor_index_mut() += 1;

        // send all the chars after the cursor to the terminal
        let num_after_cursor = line.end_index() - line.cursor_index();
        self.write_byte(byte).await?;
        self.write_bytes(line.cursor_to_end()).await?;
        self.write_caret_back(num_after_cursor).await?;

        Ok(())
    }

    async fn handle_control(&mut self, byte: u8) -> Result<(), ReadlineError<Error>> {
        let mut buffers = self.borrow_buffers_mut();
        let line = buffers.current_line_mut();

        match byte {
            // A = up arrow key
            b'A' => {
                // see if there is a previous line in the history
            }
            // B = down arrow key
            b'B' => {
                // see if there is a next line in the history
            }
            b'C' => {
                // right arrow key
                if line.cursor_index() >= line.end_index() {
                    return Ok(());
                }
                self.write_byte(line.data()[line.cursor_index()]).await?;
                *line.cursor_index_mut() += 1;
            }
            b'D' => {
                // left arrow key
                if line.cursor_index() == 0 {
                    return Ok(());
                }
                *line.cursor_index_mut() -= 1;
                self.write_caret_back(1).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn read_byte(&self) -> Result<u8, Error> {
        let mut uart = self.uart.borrow_mut();
        let mut byte = [0];
        uart.read(&mut byte).await?;
        Ok(byte[0])
    }

    async fn write_caret_back(&self, num: usize) -> Result<(), Error> {
        for _ in 0..num {
            self.write_byte(0x08).await?;
        }
        Ok(())
    }
    async fn write_spaces(&self, num: usize) -> Result<(), Error> {
        for _ in 0..num {
            self.write_byte(b' ').await?;
        }
        Ok(())
    }
    async fn write_byte(&self, byte: u8) -> Result<(), Error> {
        self.write_bytes(&[byte]).await
    }

    async fn write_bytes(&self, bytes: &[u8]) -> Result<(), Error> {
        let mut uart = self.uart.borrow_mut();
        uart.write(bytes).await?;
        Ok(())
    }
}

pub async fn readline<'u, 'b, Error, ReaderWriter, const A: usize, const B: usize>(
    uart: &'u mut ReaderWriter,
    buffers: &'b mut Buffers<A, B>,
) -> Result<&'b [u8], ReadlineError<Error>>
where
    Error: eia::Error,
    ReaderWriter: eia::Read<Error = Error> + eia::Write<Error = Error>,
{
    State {
        uart: RefCell::new(uart),
        buffers: RefCell::new(Some(buffers)),
        status: ReadlineStatus::Char,
    }
    .readline()
    .await
}
