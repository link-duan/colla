# Text coordinates

Colla has three distinct address systems: element IDs, Paths, and positions within
Text/RichText. A text offset is not an element ID or a List index.

## Coordinate units

| Surface | Position unit |
| --- | --- |
| JavaScript TextEditor / RichTextEditor | UTF-16 code units |
| Low-level Change and EditStep | Unicode scalars |
| Rust text editing | Unicode scalars |
| RichText embed | One position in either sequence system |

For `A😀B`, UTF-16 boundaries are 0, 1, 3, 4; scalar boundaries are 0, 1, 2, 3.
Neither system counts user-perceived grapheme clusters. Combining marks can occupy
multiple scalar positions even when rendered as one glyph.

## Convert using the working content

To convert a scalar position to UTF-16, walk that many code points in the current
string and sum each code point's JavaScript string length. To convert the other way,
walk code points until their cumulative UTF-16 length equals the requested offset;
reject an offset inside a surrogate pair. For RichText, include embeds as length one.

Repeat conversion against the content before each operation. Earlier insertions and
deletions shift later positions. Do not convert every step against the transaction's
original snapshot or pass browser offsets directly to `Change.create`.

## Practical adapter choice

Send browser selection ranges into high-level editors, which perform boundary checking.
For an incremental UI projection, process Edit Steps sequentially from `event.before`.
A simpler adapter can rerender from `event.after`, at the cost of explicitly restoring
selection and composition state. The [editor example](/docs/examples/editor) maintains
a model mirror without incorrectly interpreting scalar steps as browser offsets.
