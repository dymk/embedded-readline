use core::{cell::RefCell, ops::DerefMut};

use embedded_io_async as eia;

use crate::{line_diff::LineDiff, readline_error::ReadlineError, Buffers};

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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum Loop {
    Continue,
    Break,
}

struct Readline<'u, 'b, ReaderWriter, const A: usize, const B: usize> {
    uart: RefCell<&'u mut ReaderWriter>,
    buffers: &'b mut Buffers<A, B>,
    status: ReadlineStatus,
}

impl<'u, 'b, ReaderWriter, Error, const A: usize, const B: usize>
    Readline<'u, 'b, ReaderWriter, A, B>
where
    ReaderWriter: eia::Read<Error = Error> + eia::Write<Error = Error>,
    Error: eia::Error,
{
    async fn readline(mut self) -> Result<&'b [u8], ReadlineError<Error>> {
        self.buffers.current_line_mut().clear();

        loop {
            let byte = self.read_byte().await?;
            if self.process_byte(byte).await? == Loop::Break {
                break;
            }
        }

        let line = self.buffers.push_history();
        Ok(line.start_to_end())
    }

    async fn apply_diff(
        &mut self,
        f: impl FnOnce(&mut Buffers<A, B>) -> LineDiff,
    ) -> Result<(), ReadlineError<Error>> {
        let diff = f(self.buffers);
        self.apply_line_diff(diff).await
    }

    async fn process_byte(&mut self, byte: u8) -> Result<Loop, ReadlineError<Error>> {
        match (byte, self.status) {
            (b'\n', _) | (b'\r', _) => {
                return Ok(Loop::Break);
            }
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
                self.apply_diff(|buffers| buffers.cursor_to_start()).await?;
            }
            (0x05, ReadlineStatus::Char) => {
                // go to the end of the line
                self.apply_diff(|buffers| buffers.cursor_to_end()).await?;
            }
            (0x0B, ReadlineStatus::Char) => {
                // delete to end of line
                self.apply_diff(|buffers| buffers.delete_to_end()).await?;
            }
            (0x0E, ReadlineStatus::Char) => {
                // ctrl+n, next history line
                self.apply_diff(|buffers| buffers.select_next_line())
                    .await?;
            }
            (0x10, ReadlineStatus::Char) => {
                // ctrl+p, previous history line
                self.apply_diff(|buffers| buffers.select_prev_line())
                    .await?;
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

    async fn apply_line_diff(&mut self, line_diff: LineDiff) -> Result<(), ReadlineError<Error>> {
        let line = self.buffers.current_line();
        let mut uart = self.uart.borrow_mut();
        line_diff.apply(uart.deref_mut(), line).await?;
        Ok(())
    }

    async fn handle_delete_word(&mut self) -> Result<(), ReadlineError<Error>> {
        self.apply_diff(|buffers| buffers.delete_word()).await?;
        Ok(())
    }

    async fn handle_backspace(&mut self) -> Result<(), ReadlineError<Error>> {
        self.apply_diff(|buffers| buffers.delete_chars(1)).await
    }

    /// Handle a character byte, put a character in the buffer and move the cursor
    async fn handle_char(&mut self, byte: u8) -> Result<(), ReadlineError<Error>> {
        self.apply_diff(|buffers| buffers.insert_chars(&[byte]))
            .await
    }

    async fn handle_control(&mut self, byte: u8) -> Result<(), ReadlineError<Error>> {
        match byte {
            b'A' => {
                // up arrow key, go to previous history item
                return self.apply_diff(|buffers| buffers.select_prev_line()).await;
            }
            b'B' => {
                // B arrow key, go to next history item
                return self.apply_diff(|buffers| buffers.select_next_line()).await;
            }
            b'C' => {
                // C arrow key, go right
                return self.apply_diff(|buffers| buffers.move_cursor(1)).await;
            }
            b'D' => {
                // D arrow key, go left
                return self.apply_diff(|buffers| buffers.move_cursor(-1)).await;
            }
            _ => {}
        }
        Ok(())
    }

    async fn read_byte(&self) -> Result<u8, Error> {
        let mut byte = [0];
        let mut uart = self.uart.borrow_mut();
        uart.read(&mut byte).await?;
        Ok(byte[0])
    }
}

pub async fn readline<'u, 'b, Error, ReaderWriter, const A: usize, const B: usize>(
    uart: &'u mut ReaderWriter,
    buffers: &'b mut Buffers<A, B>,
) -> Result<&'b str, ReadlineError<Error>>
where
    Error: eia::Error,
    ReaderWriter: eia::Read<Error = Error> + eia::Write<Error = Error>,
{
    let ret = Readline {
        uart: RefCell::new(uart),
        buffers,
        status: ReadlineStatus::Char,
    }
    .readline()
    .await?;
    Ok(core::str::from_utf8(ret).unwrap())
}

#[cfg(test)]
mod tests {
    use crate::{readline, test_reader_writer::TestReaderWriter, Buffers};

    #[tokio::test]
    async fn test_simple() {
        let buffer = [&b"hello\n"[..], &b"world\n"[..]].join(&b""[..]);

        let mut test_rw = TestReaderWriter::new(&buffer);
        let mut buffers: Buffers<8, 2> = Buffers::default();

        let result = readline(&mut test_rw, &mut buffers).await.unwrap();
        assert_eq!(result, "hello");
        assert_eq_u8(&test_rw.data_to_write, "hello");

        let result = readline(&mut test_rw, &mut buffers).await.unwrap();
        assert_eq!(result, "world");
        assert_eq_u8(&test_rw.data_to_write, "helloworld");

        assert!(test_rw.totally_consumed());
    }

    #[tokio::test]
    async fn test_history_simple() {
        let buffer = [
            &b"omg!\n"[..],
            &b"wtf?\n"[..],
            &b"\x1B[Abbq~\n"[..], // up arrow+enter+'bbq~'
        ]
        .join(&b""[..]);

        let mut test_rw = TestReaderWriter::new(&buffer);
        let mut buffers: Buffers<8, 2> = Buffers::default();

        let result = readline(&mut test_rw, &mut buffers).await.unwrap();
        assert_eq!(result, "omg!");
        assert_eq_u8(&test_rw.data_to_write, "omg!");

        let result = readline(&mut test_rw, &mut buffers).await.unwrap();
        assert_eq!(result, "wtf?");
        assert_eq_u8(&test_rw.data_to_write, "omg!wtf?");

        let result = readline(&mut test_rw, &mut buffers).await.unwrap();
        assert_eq!(result, "wtf?bbq~");

        assert!(test_rw.totally_consumed());
    }

    #[tokio::test]
    async fn test_history_up_down() {
        let buffer = [
            &b"yes!\n"[..],
            // up arrow, up arrow,
            // down arrow, down arrow
            &b"\x1B[A\x1B[B\n"[..],
        ]
        .join(&b""[..]);

        let mut test_rw = TestReaderWriter::new(&buffer);
        let mut buffers: Buffers<8, 4> = Buffers::default();

        let result = readline(&mut test_rw, &mut buffers).await.unwrap();
        assert_eq!(result, "yes!");
        assert_eq_u8(test_rw.data_to_write.as_ref(), "yes!");

        test_rw.data_to_write.clear();
        let result = readline(&mut test_rw, &mut buffers).await.unwrap();
        assert_eq!(result, "");
        assert_eq_u8(
            test_rw.data_to_write.as_ref(),
            "yes!\x08\x08\x08\x08    \x08\x08\x08\x08",
        );
    }

    #[tokio::test]
    async fn test_handle_delete_word() {
        let buffer = b"a b\x17\n\x1B[A\x17\n";
        let mut test_rw = TestReaderWriter::new(buffer);
        let mut buffers: Buffers<32, 4> = Buffers::default();
        let result = readline(&mut test_rw, &mut buffers).await.unwrap();
        assert_eq!(result, "a ");
        assert_eq_u8(test_rw.data_to_write.as_ref(), "a b\x08 \x08");

        test_rw.data_to_write.clear();

        std::println!("second line");
        let result = readline(&mut test_rw, &mut buffers).await.unwrap();
        assert_eq!(result, "");
        assert_eq_u8(test_rw.data_to_write.as_ref(), "a \x08\x08  \x08\x08");

        assert!(test_rw.totally_consumed());
    }

    #[tokio::test]
    async fn test_handle_delete_word_middle() {
        // "a b " <- <- CTRL+W ENTER
        let buffer = b"a b \x1B[D\x1B[D\x17\n";
        let mut test_rw = TestReaderWriter::new(buffer);
        let mut buffers: Buffers<32, 4> = Buffers::default();
        let result = readline(&mut test_rw, &mut buffers).await.unwrap();
        assert_eq!(result, "b ");
        assert_eq_u8(
            test_rw.data_to_write.as_ref(),
            "a b \x08\x08\x08\x08b   \x08\x08\x08\x08",
        );

        assert!(test_rw.totally_consumed());
    }

    #[track_caller]
    fn assert_eq_u8(actual: &[u8], expected: &str) {
        if actual != expected.as_bytes() {
            let actual = std::str::from_utf8(actual).unwrap();
            panic!("{:?} != {:?}", actual, expected);
        }
    }
}
