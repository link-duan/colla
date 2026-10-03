//! Path-addressed content, collaborative editing, and synchronization.

mod change;
mod codec;
mod editing;
mod error;
mod history;
mod rich;
mod sync;
mod transform;
mod value;

pub use change::{apply, compose, invert, Change, Operation};
pub use editing::{Document, EditResult, Origin, Transaction};
pub(crate) use error::Error;
pub use error::{CollaError, ErrorCode, Result};
pub use history::{History, HistoryCheckpoint};
pub use rich::RichOp;
pub use sync::{
    Authority, AuthorityCheckpoint, Commit, Rejection, ServerMessage, SessionCheckpoint,
    Submission, SyncSession, SyncSnapshot,
};
pub use transform::{transform, Priority};
pub use value::{Attr, AttrPatch, Attrs, Body, Path, RichSpan, Segment, Value};

/// Private facade support, enabled only by the binding crate through the
/// unstable `__bindings` feature. Exempt from semver.
#[cfg(feature = "__bindings")]
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
}
