# Concurrency and conflicts

Concurrent changes are interpreted against a common immutable Value basis. Authority
establishes one ordering for the document; clients rebase optimistic work as that ordered
history arrives.

## Who wins competing intent

Transform rewrites concurrent edits so each lands on the element it was made against,
and every combination produces a result. Where two edits compete, such as two Sets of
one field or two insertions at the same position, the side with priority wins. In
synchronization, already committed changes have priority: Authority rebases a
submission over commits it has not seen, and clients rebase pending local work over
incoming commits the same way. The full resolution table is in
[Concurrent edits](/docs/core/concurrency).

A deletion or replacement discards concurrent work inside that element. Where losing
such work matters, prefer finer-grained edits over replacing a whole subtree.

## Guarantees and limits

Transform satisfies TP1: applying either concurrent
branch and then its transformed counterpart converges to the same content.
This is not a TP2 guarantee or a general peer-to-peer protocol. Use the centralized
ordering model and distinguish algorithm properties from your transport's ordering,
durability and retry guarantees.

See [Change algebra](/docs/core/algebra) for the return order and a runnable concurrent-edit example.
