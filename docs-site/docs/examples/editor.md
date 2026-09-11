# Editor adapter

Translate a browser selection into a text edit and update an immutable model mirror from the resulting Edit Steps. The input and output use their respective coordinate systems.

## Example

<<< ../../examples/editor.ts

## Expected behavior

Replacing the emoji in `A😀B` yields `A🐬B`. The browser selection uses UTF-16 offsets 1 through 3; the returned Edit Steps use Unicode scalar positions.

## Use it in your application

Send browser offsets into high-level text editors. Replay Edit Steps against the prior model, or translate them sequentially when updating an external editor. The listener only updates the mirror and does not create another local edit. Add your editor’s rendering, selection and composition handling as described in [Editor integration](/docs/editing/editor-integration).
