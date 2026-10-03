//! Centralized synchronization: wire records, the ordering Authority and the
//! client SyncSession runtime.

mod authority;
mod protocol;
mod session;

use super::{Error, ErrorCode, Result};

pub use authority::{Authority, AuthorityCheckpoint};
pub use protocol::{Commit, Rejection, ServerMessage, Submission, SyncSnapshot};
pub(crate) use session::SessionData;
pub use session::{SessionCheckpoint, SyncSession};

pub(super) fn identity(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 4096 {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "document/client identity must contain 1..4096 UTF-8 bytes",
        ));
    }
    Ok(())
}
