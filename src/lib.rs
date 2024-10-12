#![no_std]

#[cfg(test)]
extern crate std;

mod readline;
#[cfg(test)]
mod readline_tests;

pub use readline::{readline, ReadlineError};
