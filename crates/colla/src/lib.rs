#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

/// RichText attribute values, attribute sets, and formatting patches.
pub mod attrs;
/// Recursive Change types and typed sequence operations.
pub mod change;
/// Canonical binary encoding and strict decoding.
pub mod codec;
/// Errors returned by construction, algebra, codecs, and coordinate conversion.
pub mod error;
/// Resource limits applied when decoding untrusted input.
pub mod input_limits;
/// Apply, Compose, Invert, and Transform operations.
pub mod op;
/// Snapshot-relative paths used for lookup and error reporting.
pub mod path;
/// RichText spans, content, metrics, and coordinate conversion.
pub mod richtext;
/// Immutable Value types and their constructors and observers.
pub mod value;

pub use attrs::{AttrChange, AttrPatch, AttrValue, Attrs};
pub use change::{
    Change, ChangeKind, IntChange, ListChange, ListOp, MapChange, MapEntryChange, RichTextChange,
    RichTextOp, TextChange, TextOp, TieBreak,
};
pub use error::{
    ApplyError, CodecError, ComposeError, InvertError, TransformError, Utf16PositionError,
    ValueError,
};
pub use input_limits::InputLimits;
pub use op::{apply, compose, invert, transform_pair};
pub use path::{Path, PathSeg};
pub use richtext::{RichContent, RichSpan, RichText, RichTextChunk};
pub use value::{FiniteF64, List, Map, Text, Value, ValueKind, ValueType};
