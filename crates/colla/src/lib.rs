#![deny(missing_docs)]
// cocodec-derive expands `Self { field: field }`; remove once
// https://github.com/link-duan/cocodec/issues/2 is released.
#![allow(clippy::redundant_field_names)]
#![doc = include_str!("../README.md")]
mod engine;
mod sequence;
pub use engine::*;
pub use sequence::change::{TextChange, TextOp};
