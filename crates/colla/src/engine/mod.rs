//! Identity-aware content, native movement, references, and collaboration.

mod change;
mod codec;
mod editing;
mod error;
mod history;
mod identity;
mod rich;
mod sync;
mod transform;
mod value;

pub use change::{apply, compose, invert, Change, Destination, Operation};
pub use editing::{Document, EditResult, Origin, Transaction};
pub(crate) use error::Error;
pub use error::{CollaError, ErrorCode, Result};
pub use history::{History, HistoryCheckpoint};
pub use identity::{ElementId, IdAllocator};
pub use rich::RichOp;
pub use sync::{
    Authority, AuthorityCheckpoint, Commit, Rejection, ServerMessage, SessionCheckpoint,
    Submission, SyncSession, SyncSnapshot,
};
pub use transform::{transform, Priority};
pub use value::{Attr, AttrPatch, Attrs, Body, Location, Path, Ref, RichSpan, Segment, Value};

/// Private facade support, enabled only by the binding crate.
#[cfg(feature = "bindings")]
#[doc(hidden)]
pub mod binding {
    use super::*;
    pub fn begin(document: &Document, group: Option<String>) -> Result<Transaction> {
        let mut transaction = document.begin()?;
        transaction.group = group;
        Ok(transaction)
    }
    pub fn commit(transaction: &mut Transaction) -> Result<Option<EditResult>> {
        transaction.commit()
    }
    pub fn finish(transaction: &mut Transaction) {
        transaction.finish();
    }
    pub fn import_set(value: &Value, id: ElementId) -> Result<Value> {
        value.copy_into(Some(id))
    }
}
