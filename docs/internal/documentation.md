# Documentation maintenance

The public site teaches usage; package READMEs provide a short introduction and links.
API references list signatures and failure conditions. Domain specifications and ADRs
record contracts and decisions. Keep historical verification reports dated rather than
rewriting them as current evidence.

## Edit a topic

Give each article one clear question to answer. Put a rule's complete explanation in
one topic and link to it from related workflows. Keep results, events and lifecycle
separate; do not merge independent topics merely because their pages are short.

`docs-site/topics.json` records required coverage independently of the sidebar in
`docs-site/.vitepress/navigation.mjs`. Update both when adding or removing a topic.
`docs/binary-format.md` is included in the site's Protocol page; edit its content once.
Full-file Markdown includes are checked for missing files and cycles.

## Examples

Public examples explain behavior with labelled output. Keep assertions, retry variants
and failure scenarios in `docs-site/tests/examples/`. Site snippets are imported from
`docs-site/examples/` or the crate examples. Standalone TypeScript fences in tutorials
and READMEs are extracted, type-checked and run too; Reference type declarations are
lookup fragments rather than programs.

Do not add source installation to the public guide. Developer validation runs against
the workspace package, while user installation uses npm or Cargo.

## Validate changes

The developer toolchain requires Node.js 22+, pnpm, Rust, wasm-pack and the
`wasm32-unknown-unknown` target. From a prepared workspace:

```sh
pnpm --filter colla-docs test:examples
pnpm --filter colla-docs test:rust-examples
cargo test -p colla --doc
pnpm docs:check
pnpm docs:dev
```

Check article boundaries and topic coverage manually; word count is not a quality gate.
Inspect desktop and mobile navigation, long signature tables, code blocks and included
protocol headings. The Documentation workflow runs the automated checks above except
the interactive development server.
