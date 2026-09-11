# Change algebra and structural concurrency

All public algebra takes a Value base: `apply(base, change)`,
`invert(base, change)`, `compose(base, first, second)` and
`transform(base, left, right, priority)`. Transform returns left-after-right and
right-after-left, in that order, in both languages.

Changes are ordered identity-addressed operations. Each sequence index is
relative to the state at that operation. Move is native and retains its source
identity. Apply is atomic; compose must equal sequential apply; inversion must
restore content and all original identities. For mergeable concurrent inputs:

```
apply(apply(base, left), rightAfterLeft)
  == apply(apply(base, right), leftAfterRight)
```

| Concurrent intents | Required behavior |
| --- | --- |
| Move + source/descendant content edit | Content follows the source ID |
| Two moves of one source | Priority selects location; mergeable edits remain |
| Deletion of source or baseline ancestor | Deletion wins; no resurrection |
| Independent moves | Both retained |
| Combined ownership cycle | structural_conflict |
| Destination parent deleted/invalid | structural_conflict |
| Unsafe occupied Map destination | structural_conflict |

Destination List indexes are interpreted after removing the source. Map keys
must be free. Conflicts cannot silently discard unrelated subtrees. Multi-step
transform preserves original Move associations, including temporary moves used
in Map permutations. Producing the same list order by moving unrelated
neighbors is not a valid replacement for the original Move intent.

Text and RichText retain their span-based sequence algebra; properties cover
Unicode scalars, embedded Values and concurrent attributes. Int Add checks i64
overflow, including intermediate steps of a composed Change. Normalizing a
valid sequence may remove Noop and compact adjacent operations; optimization
must not hide an invalid intermediate operation.

TP2 and arbitrary peer-to-peer merging are not promised. Authority orders
commits; already committed changes have priority when rebasing submissions.
History also gives remote changes priority. See [runtime](document-model.md)
and the shared [fixtures](../golden/README.md).
