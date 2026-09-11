#![deny(missing_docs)]
#![doc = include_str!("../README.md")]
mod engine;
mod sequence;
pub use engine::*;
pub use sequence::change::{TextChange, TextOp};
