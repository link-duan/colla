---
title: Protocol reference
description: Canonical Core bodies and the COLLAS/COLLAU Snapshot and Update envelopes.
---

# Protocol reference

<p class="eyebrow">Reference</p>
<p class="lead">Colla defines deterministic Value and Change bytes, then wraps them in small versioned envelopes for local Snapshots and application Updates. The body codec is shared by Rust and JavaScript; transport policy remains yours.</p>

Use this page when implementing storage, a server boundary, or a second
language binding. For the semantic model behind the bytes, read [Core
concepts](/docs/core/values) and the [OT guide](/docs/ot/).

## Four wire shapes

| Shape | Payload | Versioned? | Intended use |
| --- | --- | --- | --- |
| Value body | One Core `Value` | No | Nested content at a Core boundary. |
| Change body | One Core `Change` | No | An operation relative to a base Value. |
| `COLLAS` Snapshot | `revision: u64` + `content: Value` | Protocol `1` | Local content checkpoint and restore. |
| `COLLAU` Update | `revision: u64` + `updateId: u64` + `change: Change` | Protocol `1` | Application-owned transport of one operation. |

The body formats are versionless by design. Do not infer a document identity,
author, timestamp, or server sequence from a Value or Change body.

## Core Value tags

The first byte of a Value body selects one closed variant:

| Tag | Value kind | Payload |
| ---: | --- | --- |
| `0x00` | `Null` | None. |
| `0x01` | `Bool` | One byte, `0x00` or `0x01`. |
| `0x02` | `Int` | Signed `i64`, zig-zag then shortest unsigned varint. |
| `0x03` | `Float` | Little-endian IEEE-754 `f64`; only finite values are valid. |
| `0x04` | `String` | UTF-8 byte length and bytes; atomic text. |
| `0x05` | `Text` | UTF-8 byte length and bytes; collaborative text. |
| `0x06` | `RichText` | Span count followed by RichText spans. |
| `0x07` | `List` | Element count followed by Values. |
| `0x08` | `Map` | Entry count followed by key/Value pairs. |

Map keys are UTF-8 strings in strictly increasing lexical order. A duplicate or
out-of-order key is not a canonical body. `Text` lengths count Unicode scalar
values in Change operations, while the encoded content itself is UTF-8.

## Core Change tags

| Tag | Change kind | Payload |
| ---: | --- | --- |
| `0x00` | `Noop` | None. |
| `0x01` | `Replace` | One Value. |
| `0x02` | `Map` | Ordered map entry changes. |
| `0x03` | `List` | Sequence operation stream. |
| `0x04` | `Text` | Retain/insert/delete stream. |
| `0x05` | `RichText` | Content and attribute operation stream. |
| `0x06` | `Int` | Signed addition delta as zig-zag varint. |

Map entries use `Insert`, `Delete`, or recursive `Modify`. List operations use
`Retain`, `Insert`, `Delete`, or one-element `Modify`. Text operations use
`Retain`, `Insert`, and `Delete`. RichText operations use `Retain` with an
optional attribute patch, `Insert` of text or one atomic embed, and `Delete`.

Constructors produce the semantic canonical form: zero-length operations and
empty inserts are removed, compatible adjacent operations are merged, and
trailing plain retains are omitted. `Change` does not include a base revision,
old values, author, or operation ID.

## RichText payloads

RichText is encoded as a span count followed by spans. A span carries either
text content or one atomic embed, then its canonical attributes:

| Span content | Meaning |
| --- | --- |
| Text | UTF-8 text; adjacent text with equal attributes is merged by constructors. |
| Embed | One nested Value; an embed has sequence length one and is not recursively edited in RichText. |

Attribute values are Bool, signed Int, finite Float, or String. Attribute keys
are strictly increasing. A retain patch contains explicit `Set` and `Remove`
actions; `Null` is not a deletion sentinel.

## Local envelopes

Both high-level envelopes have the same eight-byte header:

```text
bytes 0..5  magic ASCII (`COLLAS` or `COLLAU`)
bytes 6..7  protocol version, unsigned u16 little-endian (`1`)
bytes 8..   cocodec tuple payload
```

The payloads are:

```text
Snapshot: (revision: u64, content: Value)
Update:   (revision: u64, updateId: u64, change: Change)
```

In an Update, `revision` is the base revision at which the Change was created.
In a Snapshot, `revision` is the visible content revision. `updateId` starts at
`1` for each JavaScript `Document` instance and is an instance-local FIFO
acknowledgement correlation value. It is not global identity and is not stored
in a Snapshot.

JavaScript example:

```ts
import { Document, Snapshot, Update } from 'colla-ot'
import { Change } from 'colla-ot/core'

const document = Document.fromJS('Draft')
const snapshotBytes = document.snapshot().encode()
const snapshot = Snapshot.decode(snapshotBytes)

const change = Change.build(builder => builder.replace('Draft v2'))
const update = document.applyLocal(change)
const updateBytes = update.encode()
const received = Update.decode(updateBytes)
```

Rust exposes the same envelope types as `Snapshot::new`/`decode` and
`Update::new`/`decode`. See the [Rust API](/reference/rust) for signatures.

## Strict decoding

The shared decoder must consume one complete value, change, or envelope. It
rejects:

- wrong envelope magic or unsupported protocol version;
- truncated input, lengths beyond the remaining bytes, and unknown tags;
- invalid UTF-8, non-minimal varints, invalid bool bytes, and out-of-range integers;
- duplicate or out-of-order map/attribute keys;
- non-finite floats and other invalid Value construction data;
- trailing bytes after a complete body or envelope;
- recursion depth or other built-in resource limits being exceeded.

Rust reports these as `CodecError`; the JavaScript facade maps them to
`CollaError` with `code: 'invalid_encoding'` or `code: 'limit_exceeded'`. Error
messages and byte offsets are diagnostics, not a cross-version wire contract.
See [Glossary and errors](/reference/glossary) for the complete taxonomy.

### Byte canonical versus semantic normalization

The codec enforces byte-level rules such as shortest varints, valid UTF-8, key
ordering, known tags, and complete input. Semantic normalization belongs to
constructors and algebra: they remove empty operations, merge compatible
operations, and collapse no-op changes. A decoder should therefore not be used
as a substitute for Change construction validation; decode, then apply against
a concrete base when compatibility matters.

`cocodec` supplies fixed recursion and safe length handling for byte decoding.
The JavaScript `InputLimits` policy applies to structured `fromJS` input, not to
the canonical body or to OT results. See [JavaScript API](/reference/javascript)
for the default structured-input limits.

## Application responsibilities

The Colla envelopes intentionally leave these concerns to an outer protocol:

| Concern | Provide outside the envelope |
| --- | --- |
| Identity | Document ID, tenant, author, client/session ID. |
| Ordering | Server sequence, confirmed revision policy, and delivery order. |
| Reliability | Retries, acknowledgements, deduplication, and replay handling. |
| Security | Authentication, authorization, signatures, and encryption. |
| Storage | Snapshot cadence, outbound queue, history, and migrations. |
| Encoding policy | Compression, checksums, framing, and content type. |
| UI state | Presence, cursors, selections, and editor-specific formats. |

The JavaScript `Document` helper accepts server-ordered Updates at the next
confirmed revision, rebases pending local edits with a fixed `left-first`
tie-break, and acknowledges pending Updates in FIFO order. It does not supply a
network transport, session protocol, global deduplication, or crash-recovery
queue. Read the [Document API](/docs/document/) before designing that
outer protocol.

## Compatibility checklist

When implementing another reader or writer:

1. Treat all integers and revisions as unsigned/signed widths shown above.
2. Encode map and attribute keys in strict lexical order.
3. Use shortest varints and valid UTF-8; consume the entire input.
4. Keep Value/Change bodies separate from `COLLAS`/`COLLAU` envelopes.
5. Preserve `updateId` as local correlation only; never use it as global identity.
6. Add application metadata in an outer frame, not by changing a body silently.
7. Run the shared golden fixtures and round-trip tests before shipping.

## Related pages

- [JavaScript API](/reference/javascript)
- [Rust API](/reference/rust)
- [Glossary and errors](/reference/glossary)
- [Core model](/docs/core/values)
- [OT guide](/docs/ot/)
- [Document API](/docs/document/)
- [Canonical source specification](https://github.com/link-duan/colla/blob/master/docs/binary-format.md)
