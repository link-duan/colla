# Core concepts

Colla's core is a small, runtime-free model: immutable content, addresses into that
content, and edits that can be applied, inverted, composed and transformed. Everything
else in the library is built on these three concepts.

## Value, Path and Change

A **Value** is an immutable content tree: scalars, atomic strings, collaborative Text
and RichText, Lists and Maps. Values carry no identity; two trees with the same content
are equal. See [Values](./values).

A **Path** addresses an element inside one Value, using Map keys and List indexes.
Paths are interpreted against a specific content state and change as content is edited.
Positions inside Text and RichText are sequence coordinates, not Path segments. See
[Paths](./paths) and [Positions](./positions).

A **Change** is an ordered sequence of path-addressed operations. It describes an edit
without carrying the content it applies to. Every algebra function takes the base Value
explicitly. See [Changes](./changes) and [Change algebra](./algebra).

## How the layers fit together

| Layer                                    | Owns                                       | Built on           |
| ---------------------------------------- | ------------------------------------------ | ------------------ |
| Core: Value, Path, Change, algebra       | Immutable content and edits                | —                  |
| [Document](/docs/editing/)               | Current content, transactions, events      | Change, apply      |
| [History](/docs/history/)                | Undo and redo intent                       | invert, transform  |
| [SyncSession and Authority](/docs/sync/) | Confirmed base, pending work, commit order | compose, transform |

Most applications edit through a Document and never construct a Change by hand. The
core matters to them as the model: which value types to choose, how Paths behave, and
how concurrent edits resolve.

## When to use the core directly

Use Values and Changes directly when there is no live editing runtime, for example:

- validating or replaying received Changes on a server;
- storing content and edit logs with `encode` and `decode`;
- computing a merged result from two Changes made against the same base;
- testing an editor adapter against expected Change operations.

## Reading order

Start with [Values](./values) and [Paths](./paths) to model content, then
[Text and RichText](./text) and [Positions](./positions) for collaborative text.
[Changes](./changes), [Change algebra](./algebra) and [Concurrent edits](./concurrency)
explain what an edit is and how concurrent edits combine.
