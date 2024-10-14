#![no_std]

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod test_reader_writer;

mod buffers;
mod readline;
mod util;

pub use buffers::Buffers;
pub use readline::{readline, ReadlineError};
