# Move and Ref

Move a task from the todo List into the done List. Store a Ref to that task before moving it, so the selected item continues to identify the same element after its location changes.

## Example

<<< ../../examples/move-ref.ts

## Expected behavior

The task’s Path becomes `['done', 0]`, while its ElementId remains unchanged. Resolving a Ref to that ID still returns the task.

## Use it in your application

Use IDs for relationships that should follow an element. Paths describe a location in one snapshot and can change after insertions or moves. The destination parent must exist, and a List destination index is interpreted after source removal. See [Move, Copy and Set](/docs/core/move-copy-set) for copying and replacement semantics, and [References](/docs/core/references) for dangling targets.
