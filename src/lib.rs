#![no_std]

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod readline_tests;

mod buffers;
mod readline;

pub use buffers::Buffers;
pub use readline::{readline, ReadlineError};
