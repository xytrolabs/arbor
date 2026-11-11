//! Re-exports signature-aware named flag helpers from `arbor-protocol`.
//!
//! Kept as a thin module so existing `crate::named_flags::…` call sites stay stable.

pub use arbor_protocol::engine::named_flags::*;
