#![no_std]

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod test_reader_writer;

mod buffers;
mod line;
mod line_cursor;
mod line_diff;
mod util;

pub use buffers::Buffers;

mod readline;
pub use readline::{readline, ReadlineError};
