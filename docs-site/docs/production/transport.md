# Transport and authentication

Colla provides protocol state and bytes. Your service provides transport, identity,
authorization and the single ordering boundary for each document.

## Bind application identity to requests

Authenticate the connection or request and authorize access to the requested document.
Do not treat Submission.clientId or documentId as proof of permission. Bind a client ID
to one active writer and the authenticated application identity according to your service's
policy. Validate that a received message belongs to the intended document.

## Delivery and ordering

Send controlled Submission bytes to the Authority owner. Deliver ordered Commits
to every subscribed client, including the sender; return Rejections to their submitting
client. Follow the [durable adoption sequence](./persistence#server-durability). Use reconnection and backoff outside Document edits; duplicate messages are
safe but missing intervals require catch-up.

Use a binary-capable transport or an explicitly agreed encoding around the canonical
bytes. Do not hand-build JSON versions of protocol classes. Bound payload size before
buffering an entire untrusted message, and apply rate limits at the service boundary.

## Content safety

Map keys, strings and rich-text embeds are data, not sanitized HTML. Escape rendered
text and validate application-specific URLs, embed types and document schemas. Library
resource validation does not replace application permissions or content policy.

Presence, cursors shared outside document content and cross-document links need their
own application protocol. See [Errors and resource limits](./errors-limits) and
[Sync retries](/docs/sync/retries) for failure handling.
