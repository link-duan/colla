# Recovery

Recovery-required means synchronization cannot safely continue from the retained basis.
Examples include unavailable history, structural rejection and an impossible rebase.
The session retains local work and stops outbound sending; local edits and export remain
available.

## Preserve first

Observe session.state.status and recoveryReason. Save the complete SessionCheckpoint and
an immutable snapshot for presentation or export. Communicate that local work is retained
but not being synchronized. A transport reconnect alone does not repair this state.

## Establish a new baseline

Fetch a fresh SyncSnapshot from the server. Stop the old writer and create a new session
with a fresh client identity. Compare the retained local work with the new confirmed
content and explicitly apply the changes the user or application chooses to keep.
Use stable IDs where they still exist; handle deleted targets and structural conflicts
as reconciliation decisions. Do not blindly replay a stale Change against a new base.

Replacing the fresh session's whole content with the old visible snapshot can overwrite
remote work. Content export is available for manual recovery, but it is not a generic
merge algorithm. Your application's schema determines a safe reconciliation UI.

## Distinguish ordinary restart

Restoring a valid SessionCheckpoint resumes its original client and request identity.
It does not automatically resolve a recovery-required checkpoint. Restoring a Value
creates standalone content and does not resume any synchronization state. Keep these
flows distinct in both storage and UI.

See [Revision gaps](./retries) for recoverable missing-message delivery and
[Persistence](/docs/production/persistence) for checkpoint responsibilities.
