# ListMove

Move a task to a new position in the same List. The destination index is interpreted after removing the task.

## Example

<<< ../../examples/list-move.ts

## Expected behavior

Moving index 0 to index 1 in a two-task List swaps them. A concurrent edit to the moved task's title follows the task to its new position when the two changes are transformed.

## Use it in your application

ListMove reorders elements within one List. To move content to another parent, delete it and insert it at the destination; concurrent edits do not follow that pair. See [Move, Copy and Set](/docs/core/move-copy-set) for copying and replacement semantics.
