//! Private scalar sequence algebra shared by Text and RichText.
#![allow(missing_docs)]
pub(crate) mod attrs;
pub(crate) mod change;
pub(crate) mod error;
pub(crate) mod op;
pub(crate) mod richtext;
pub(crate) mod value;
pub(crate) use change::*;
pub(crate) use error::*;
pub(crate) use value::Value;
