# Protocol and encoding

This reference covers object fields, the binary envelope, canonical encoding rules and
validation. It is not a complete positional payload layout for implementing another codec.
Use the library codecs to exchange data.

## Object fields

| Object | Readable fields / preserved state |
| --- | --- |
| Value | Owning identity and Body tree |
| Change | Ordered identity-targeted operations |
| SyncSnapshot | documentId: string, revision: bigint, value: Value |
| Submission | documentId, clientId: string, sequence, baseRevision: bigint, change: Change |
| ServerMessage Commit | type: commit, documentId, clientId, sequence, revision, change |
| ServerMessage Rejection | type: rejection, documentId, clientId, sequence, reason: CollaError |
| SessionCheckpoint | Confirmed basis, writer/sequence, original request and basis, rebased pending, buffer, visible version, retained rebase commits, enabled History |
| HistoryCheckpoint | Undo/redo stacks, capacity/grouping and exact content basis |
| AuthorityCheckpoint | History floor snapshot, retained contiguous commits and deduplication receipts |

JavaScript ServerMessage uses optional revision/change/reason fields; inspect type before
accessing them. Rust uses Commit and Rejection enum variants. Checkpoint internals are
opaque in the JavaScript facade; their encoded contents are not an application mutation API.
All protocol u64 values are bigint in JavaScript. CollaError details are string-valued,
including diagnostic revision intervals; convert explicitly when passing them to bigint APIs.

## Binary envelope

Rust is the sole wire implementation. The common header is five ASCII bytes
`COLLA`, a little-endian u16 equal to 2, then one type byte. A canonical cocodec
payload follows. Version 1 and unversioned legacy bodies are rejected.

| Tag | Object |
| --- | --- |
| 1 | Value |
| 2 | Change |
| 3 | SyncSnapshot |
| 4 | Submission |
| 5 | ServerMessage (Commit or Rejection) |
| 6 | SessionCheckpoint |
| 7 | HistoryCheckpoint |
| 8 | AuthorityCheckpoint |

## Canonical representation

Records contain required positional fields, never defaults for missing IDs or
revisions. Integers use cocodec canonical varint/zigzag; tagged unions have
explicit stable numeric tags. Maps are sorted with unique keys. Every owning
Value writes its ElementId before its Body. An ID is a length-prefixed 16-byte
namespace followed by a positive u64 sequence. Ref contains a target ID without
expanding or validating target existence. Parent and reverse-reference indexes
are derived and omitted.

Body tags are Null 0, Bool 1, Int 2, Float 3, String 4, Text 5, RichText 6, Ref 7,
List 8 and Map 9. Operation tags are Insert 0, Delete 1, Set 2, Move 3, Text 4,
Add 5 and RichText 6. Public low-level sequence positions are scalars. Use
controlled Value/Change construction and the Rust codecs rather than assembling
bytes or protocol field objects in application code.

## Validation

Decoding rejects wrong types/versions, trailing data, nonminimal encodings,
unknown tags, invalid float values, duplicate owner IDs, invalid u64 protocol
fields, malformed structures and limits. Ownership depth is bounded to 100;
nodes/operations to 1,000,000; individual strings to 16 MiB of UTF-8.
There is no fixed total byte-size limit on an envelope. Decoder recursion and allocation are bounded before trusting lengths.
Values and Changes are validated again at the controlled public boundary.

SessionCheckpoint checks its visible content against confirmed + rebased
in-flight + buffer unless recovering, and verifies the original request against
its original content basis and retained intervening commits. History restores
only against its exact content basis. Authority checkpoints validate the history
floor, applicable contiguous log and corresponding request receipts.

`encode()` returns an independent byte array. String and Text constructors enforce
the same individual content limit as decoding. Decode retains no caller-mutable
buffer. Shared fixed v2 examples live in [golden/v2.json](https://github.com/link-duan/colla/blob/master/golden/v2.json).
Changing these bytes requires explicit fixture review and a format decision;
tests are regression evidence, not a second codec implementation.


## Workflows

For request identity and confirmation, see [Submissions and commits](https://link-duan.github.io/colla/docs/sync/submissions).
For stable retries and missing revisions, see [Retries and revision gaps](https://link-duan.github.io/colla/docs/sync/retries).
Storage and adoption ordering are covered in [Persistence and restart](https://link-duan.github.io/colla/docs/production/persistence).
