#![no_std]

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod test_reader_writer;

mod line;
mod line_diff;
mod util;

mod buffers;
mod readline;
mod readline_error;

pub use buffers::Buffers;
pub use readline::readline;
pub use readline_error::ReadlineError;
