# Errors and resource limits

Public failures use CollaError with a stable code, operation and immutable details.
Some errors also identify the affected element. Match codes rather than parsing message
text; reason strings are diagnostic information, not a protocol contract.

## Respond at the right boundary

| Failure | Application response |
| --- | --- |
| invalid_argument / type_mismatch | Fix unsupported input or editor selection |
| invalid_utf16_boundary / out_of_bounds | Recalculate the range against current content |
| integer_overflow | Reject or revise the counter action |
| invalid_state | Fix scope, lifecycle or dispatch reentrancy |
| invalid_encoding / limit_exceeded | Reject the payload and report the source |
| missing_revision | Fetch the indicated interval and retry |
| structural_conflict / history_expired | Inspect session state and enter explicit recovery |

A failing local transaction leaves content, version, History and pending work unchanged.
Listener exceptions occur after commit and cannot roll it back. Remote failures must be
interpreted together with session.state; some enter recovery-required while retaining edits.

## Service limits

The [protocol reference](/reference/protocol#validation) lists decoder ceilings and
validation rules. Choose lower service payload limits according to your application's
memory budget and document sizes, and enforce them before buffering a full request.
Add quotas and rate limits at the service boundary.

A payload can pass codec validation and still violate your application's schema or
permissions. See [Transport and authentication](./transport) for those checks and the
[error glossary](/reference/glossary#error-codes) for the complete code list.
