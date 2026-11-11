#![doc = include_str!("../README.md")]
pub mod extra;
pub use extra::*;

#[cfg(test)]
#[macro_use]
extern crate arbor_test_support;

#[cfg(test)]
use arbor_test_support::harness::main;
