# Change algebra and concurrency

All public algebra takes a Value base: `apply(base, change)`,
`invert(base, change)`, `compose(base, first, second)` and
`transform(base, left, right, priority)`. Transform returns left-after-right and
right-after-left, in that order, in both languages.

Changes are ordered path-addressed operations. Each path and sequence position is
relative to the content produced by the preceding operations. Apply is atomic;
compose must equal sequential apply; inversion must restore the base content. For
all valid concurrent inputs (TP1):

```
apply(apply(base, left), rightAfterLeft)
  == apply(apply(base, right), leftAfterRight)
```

Transform rewrites each operation's path across the other side's structural
operations. Concurrent edits always produce a result; there is no structural
conflict for these types.

| Concurrent intents                           | Result                                                  |
| -------------------------------------------- | ------------------------------------------------------- |
| Delete or Set P + any operation inside P     | Delete/Set wins; the inner operation is discarded       |
| Delete P + Set P                             | Delete wins                                             |
| Set P + Set P, including Map member creation | Priority selects the value                              |
| Insert at the same List position             | Priority's element comes first                          |
| ListMove + edit inside the moved element     | The edit follows the element                            |
| ListMove + Delete of the moved element       | Delete wins                                             |
| Two ListMoves of one element                 | Priority selects the destination                        |
| Text/RichText + Text/RichText on one value   | Sequence algebra; Priority orders insertions at one gap |
| Add + Add / Add + Set                        | Both additions apply / Set wins                         |

A ListMove destination index is interpreted after removing the moved element. For
structural edits on the same List, each insertion or move destination is anchored
before its right neighbor; a neighbor that the other side deletes or moves is
replaced by the next stable element.

Text and RichText retain their span-based sequence algebra; properties cover
Unicode scalars, embedded Values and concurrent attributes. Int Add checks i64
overflow, including intermediate steps of a composed Change. Normalizing a
valid sequence may remove Noop and compact adjacent operations; optimization
must not hide an invalid intermediate operation.

TP2 and arbitrary peer-to-peer merging are not promised. Authority orders
commits; already committed changes have priority when rebasing submissions.
History also gives remote changes priority. See [runtime](document-model.md)
and the shared [fixtures](../golden/README.md).
