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
