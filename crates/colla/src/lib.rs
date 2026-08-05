//! Colla: operational transformation for immutable nested values.

pub mod attrs;
pub mod builder;
pub mod change;
pub mod codec;
pub mod error;
pub mod limits;
pub mod op;
pub mod path;
pub mod richtext;
pub mod value;

pub use attrs::{AttrChange, AttrPatch, AttrValue, Attrs};
pub use builder::ChangeBuilder;
pub use change::{
    Change, ChangeKind, IntChange, ListChange, ListOp, MapChange, MapEntryChange, RichTextChange,
    RichTextOp, TextChange, TextOp, TieBreak,
};
pub use error::{
    ApplyError, BuildError, CodecError, ComposeError, InvertError, TransformError, ValueError,
};
pub use limits::Limits;
pub use op::transform_pair;
pub use path::{Path, PathSeg};
pub use richtext::{RichInsert, RichSpan, RichText};
pub use value::{FiniteF64, List, Map, Text, Value, ValueKind, ValueType};
