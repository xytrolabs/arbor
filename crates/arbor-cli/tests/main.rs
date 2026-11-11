#![allow(clippy::unwrap_used)]

mod commands;
mod completions;
mod highlight_use;
mod last_result;

#[macro_use]
extern crate arbor_test_support;
use arbor_test_support::harness::main;
