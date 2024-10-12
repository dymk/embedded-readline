use embedded_io_async as eia;

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
    uart: &'u mut ReaderWriter,
    buffer: &'b mut [u8],
    status: ReadlineStatus,
    cursor_index: usize,
    end_index: usize,
}

impl<'u, 'b, ReaderWriter, Error> State<'u, 'b, ReaderWriter>
where
    ReaderWriter: eia::Read<Error = Error> + eia::Write<Error = Error>,
    Error: eia::Error,
{
    async fn readline(mut self) -> Result<&'b [u8], ReadlineError<Error>> {
        loop {
            let byte = self.read_byte().await?;
            if matches!(self.process_byte(byte).await?, Loop::Break) {
                break;
            }
        }
        Ok(&self.buffer[..self.end_index])
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
                for _ in 0..self.cursor_index {
                    self.write_byte(0x08).await?;
                }
                self.cursor_index = 0;
            }
            (0x05, ReadlineStatus::Char) => {
                // go to the end of the line
                self.write_buffer(self.cursor_index..self.end_index).await?;
                self.cursor_index = self.end_index;
            }
            (0x0B, ReadlineStatus::Char) => {
                // delete to end of line
                for _ in self.cursor_index..self.end_index {
                    self.write_byte(b' ').await?;
                }
                for _ in self.cursor_index..self.end_index {
                    self.write_byte(0x08).await?;
                }
                self.end_index = self.cursor_index;
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
        let old_cursor = self.cursor_index;

        // find the most recent word
        while self.cursor_index > 0 && self.buffer[self.cursor_index - 1].is_ascii_whitespace() {
            self.cursor_index -= 1;
        }

        // find the start of the word
        while self.cursor_index > 0 && !self.buffer[self.cursor_index - 1].is_ascii_whitespace() {
            self.cursor_index -= 1;
        }

        // delete all the chars between the two
        let num_deleted = old_cursor - self.cursor_index;
        self.end_index -= num_deleted;
        for i in 0..num_deleted {
            self.buffer[self.cursor_index + i] = self.buffer[old_cursor + i];
        }

        // move the caret back & write out the moved letters
        for _ in 0..num_deleted {
            self.write_byte(0x08).await?;
        }
        self.write_buffer(self.cursor_index..self.end_index).await?;

        // overwrite remaining chars with spaces
        for _ in 0..num_deleted {
            self.write_byte(b' ').await?;
        }

        // finally, move the caret all the way back
        for _ in self.cursor_index..self.end_index {
            self.write_byte(0x08).await?;
        }
        for _ in 0..num_deleted {
            self.write_byte(0x08).await?;
        }

        Ok(())
    }

    async fn handle_backspace(&mut self) -> Result<(), ReadlineError<Error>> {
        if self.cursor_index == 0 {
            return Ok(());
        }

        // move all the bytes after the cursor to the left
        for i in self.cursor_index..self.end_index {
            self.buffer[i - 1] = self.buffer[i];
        }
        self.end_index -= 1;
        self.cursor_index -= 1;

        // send all the chars after the cursor to the terminal
        self.write_byte(0x08).await?;
        self.write_buffer(self.cursor_index..self.end_index).await?;
        self.write_byte(b' ').await?;

        // move the terminal's cursor back
        for _ in self.cursor_index..(self.end_index + 1) {
            self.write_byte(0x08).await?;
        }

        Ok(())
    }

    /// Handle a character byte, put a character in the buffer and move the cursor
    async fn handle_char(&mut self, byte: u8) -> Result<(), ReadlineError<Error>> {
        // move all the bytes after the cursor to the right
        for i in (self.cursor_index..self.end_index).rev() {
            self.buffer[i + 1] = self.buffer[i];
        }
        self.buffer[self.cursor_index] = byte;
        self.end_index += 1;
        self.cursor_index += 1;

        self.write_byte(byte).await?;

        // send all the chars after the cursor to the terminal
        self.write_buffer(self.cursor_index..self.end_index).await?;

        // move the terminal's cursor back
        for _ in self.cursor_index..self.end_index {
            self.uart.write(&[0x08]).await?;
        }
        Ok(())
    }

    async fn handle_control(&mut self, byte: u8) -> Result<(), ReadlineError<Error>> {
        match byte {
            // A = up arrow key
            // B = down arrow key
            b'C' => {
                // right arrow key
                if self.cursor_index >= self.end_index {
                    return Ok(());
                }
                self.write_byte(self.buffer[self.cursor_index]).await?;
                self.cursor_index += 1;
            }
            b'D' => {
                // left arrow key
                if self.cursor_index == 0 {
                    return Ok(());
                }
                self.cursor_index -= 1;
                self.write_byte(0x08).await?;
            }
            _ => {}
        }
        return Ok(());
    }

    async fn read_byte(&mut self) -> Result<u8, Error> {
        let mut byte = [0];
        self.uart.read(&mut byte).await?;
        Ok(byte[0])
    }

    async fn write_byte(&mut self, byte: u8) -> Result<(), Error> {
        self.uart.write(&[byte]).await?;
        Ok(())
    }

    async fn write_buffer(&mut self, range: core::ops::Range<usize>) -> Result<(), Error> {
        self.uart.write(&self.buffer[range]).await?;
        Ok(())
    }

    async fn write_bytes(&mut self, buffer: &[u8]) -> Result<(), Error> {
        self.uart.write(buffer).await?;
        Ok(())
    }
}

pub async fn readline<'u, 'b, Error, ReaderWriter>(
    uart: &'u mut ReaderWriter,
    buf: &'b mut [u8],
) -> Result<&'b [u8], ReadlineError<Error>>
where
    Error: eia::Error,
    ReaderWriter: eia::Read<Error = Error> + eia::Write<Error = Error>,
{
    State {
        uart,
        buffer: buf,
        status: ReadlineStatus::Char,
        cursor_index: 0,
        end_index: 0,
    }
    .readline()
    .await
}
