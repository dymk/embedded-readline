use crate::line::LineError;

#[derive(Debug, PartialEq)]
pub enum ReadlineError<Error> {
    ReaderWriterError(Error),
    LineError(LineError),
    BufferFullError,
    UnexpectedEscape,
    UnexpectedCtrl,
    UnexpectedChar(u8),
}
