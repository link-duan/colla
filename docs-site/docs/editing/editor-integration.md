# Editor integration

An editor adapter translates user intent into Transaction calls and committed model
changes into the editor's rendering model. Colla does not ship an editor-specific adapter.

## Input path

Represent editable fields as Text or RichText, not plain strings. Feed browser UTF-16
selection offsets into high-level text editors. Use one transaction for a logical atomic
action such as inserting a block and setting its reference. Use explicit History groups
for a multi-transaction gesture; grouping is not time-based.

Resolve a selection's stable element ID at the time of editing. If its target has been
deleted, choose an application fallback rather than editing a stale Path. IME composition,
grapheme navigation and selection restoration are adapter responsibilities.

## Output path

Subscribe to Document events. For a simple adapter, render event.after and restore the
selection deliberately. For incremental integration, process Edit Steps sequentially
against event.before and translate scalar positions using each intermediate content.
Move should relocate the existing UI node when possible, preserving its association
with the element ID.

Suppress editor change callbacks while applying model updates to avoid echo edits.
Listeners may read but cannot mutate Colla during dispatch. Do not create a second
Document to represent the same synchronized editor; use session.document.

## Example and limitations

The [editor adapter example](/docs/examples/editor) shows a UTF-16 input range and a
scalar model mirror. It is an adapter kernel, not a complete
contenteditable implementation. Add composition handling, rich-text schema validation,
selection behavior and browser tests for the editor you integrate.
