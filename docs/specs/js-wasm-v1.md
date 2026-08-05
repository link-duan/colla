## Problem Statement

Colla 目前只有 Rust 实现。JavaScript/TypeScript 应用无法直接复用已经定义完善的
Core Value、Change、ChangeBuilder、Operational Transformation 代数和规范二进制格式，
只能重复实现或通过非标准桥接使用。这会带来语义漂移、二进制不兼容、Unicode
坐标差异和两套 OT 实现的长期维护成本。

同时，直接暴露 wasm-bindgen 产物也不能提供稳定、符合 JavaScript 习惯的 API：
consumer 会被迫了解 Wasm 初始化、生成 class、手动 free、Rust Unicode scalar 坐标和
bundler target 差异。Colla 需要一个稳定的 npm package，复用 Rust core，隐藏 Wasm ABI，
并能在 Node.js、Vite 和 Rollup 中无特殊 Wasm 配置工作。

## Solution

发布与 Rust crate 同版本的 `@colla/core` ESM package。package 使用单一 Rust/Wasm binary
复用 Colla core，在其上提供手写 TypeScript facade。facade 公开不可变的 Value/Change
handles、从 Snapshot 开始的 fluent ChangeBuilder、包级 OT 代数函数、规范 codec、
ChangeView 检查和明确的 UTF-16/code point 坐标工具。

JavaScript 不公开 Context。Value 通过静态工厂创建，Builder 通过 `Value.change()` 从
Snapshot 创建，Apply/Compose/Invert/TransformPair 使用包级函数。默认 Limits 由冻结
常量提供，特殊 Limits 作为单次调用 options 传入，不引入可变全局状态。

Browser/default ESM 入口内嵌 base64 Wasm 并同步初始化；Node.js ESM 条件入口
读取同一份独立 Wasm binary。consumer 始终从同一 package 根入口导入，不需要
top-level await、Wasm plugin、资源复制配置或公开初始化步骤。

## User Stories

1. As a TypeScript application developer, I want to install one `@colla/core` package, so that I can use Colla without building Rust locally.
2. As a JavaScript application developer, I want the npm package to reuse the Rust core, so that JavaScript and Rust do not drift into different OT semantics.
3. As a Vite user, I want the package to work without a Wasm plugin, so that adopting Colla does not require custom bundler configuration.
4. As a Rollup user, I want the package to work with only normal node module resolution, so that I do not need resource-copy or top-level-await plugins.
5. As a Node.js ESM user, I want synchronous imports and operations on Node.js 20+, so that Colla fits ordinary server code.
6. As a browser developer, I want the same public imports in browser and Node.js, so that environment packaging details stay internal.
7. As a web worker developer, I want the browser entry to avoid DOM dependencies, so that I can use Colla in Dedicated and Shared Workers.
8. As a library consumer, I want Wasm initialization to be hidden, so that I do not need to call `init()` or await a factory.
9. As a library consumer, I want to create a Value without first creating a Context, so that simple operations do not require ceremonial setup.
10. As a developer creating a Snapshot, I want `Value.fromJS()` to validate inputs, so that every resulting Core Value is valid and canonical.
11. As a developer loading persisted state, I want `Value.decode()` to enforce canonical bytes and Limits, so that malformed data is rejected before entering the core.
12. As a developer loading persisted changes, I want `Change.decode()` to enforce canonical bytes and Limits, so that invalid operations cannot enter algebra functions.
13. As a developer persisting state, I want Value and Change handles to encode to fresh `Uint8Array` values, so that the bytes do not depend on Wasm memory lifetime.
14. As a JavaScript developer, I want ordinary strings to remain atomic Colla String values, so that character-level collaboration is always explicit.
15. As a developer needing collaborative text, I want a `text()` marker, so that Text and String round-trip without ambiguity.
16. As a developer using 64-bit integers, I want JavaScript bigint to map losslessly to Colla Int, so that i64 precision is preserved.
17. As a developer using floating-point values, I want JavaScript number to map to finite Colla Float, so that NaN and Infinity cannot corrupt canonical data.
18. As a developer constructing maps, I want unsafe objects, accessors, symbols, class instances and cycles rejected, so that Core Map has a deterministic boundary.
19. As a developer reading maps, I want frozen null-prototype records, so that arbitrary keys such as `__proto__` remain data rather than prototype behavior.
20. As a developer reading lists and records, I want recursively frozen ValueData, so that the JavaScript view preserves Value immutability.
21. As a developer editing a Snapshot, I want to start a Builder from `base.change()`, so that the relationship between Change and its base Snapshot is explicit.
22. As a developer chaining edits, I want a fluent Builder, so that multiple map, list, text, rich-text and integer edits form one Change.
23. As a developer using scoped builders, I want callback failures to roll back that callback, so that partially applied edits do not leak into the Builder.
24. As a developer retaining a Builder, I want it to own a cheap Snapshot clone, so that disposing the original Value does not invalidate the Builder.
25. As a developer finishing a Builder, I want `build()` to consume it, so that a linear mutable resource cannot be reused accidentally.
26. As a map editor, I want `set()` to perform snapshot-aware upsert, so that I do not need separate insert and replace APIs.
27. As a map editor, I want deleting a missing key to be a Noop, so that cleanup code remains simple and deterministic.
28. As a list editor, I want insert, set and half-open delete operations, so that the API is minimal and consistent.
29. As a text editor, I want insert, delete and replace in UTF-16 coordinates, so that Colla matches JavaScript strings and mainstream editor APIs.
30. As a rich-text editor, I want explicit insertText, insertEmbed, delete and format operations, so that the API does not invent unsupported atomic replace semantics.
31. As a rich-text editor, I want embeds to count as one unit, so that cursor and range behavior is deterministic.
32. As an editor integrator, I want UTF-16 positions inside surrogate pairs rejected, so that Colla never silently rounds or splits a Unicode character.
33. As an editor integrator, I want explicit UTF-16/code point resolution functions, so that I can translate editor selections to the core coordinate system.
34. As an OT consumer, I want `apply()` to return a new immutable Value without consuming inputs, so that I can retain the base and Change.
35. As an OT consumer, I want `compose()` to return a new Change without consuming inputs, so that sequential operations remain reusable.
36. As an OT consumer, I want `invert()` to explicitly receive the base Snapshot, so that inverse construction never depends on hidden state.
37. As an OT consumer, I want `transformPair()` to require deterministic left/right ordering, so that every participant can make the same conflict choice.
38. As an OT consumer, I want transformed results to correspond positionally to left and right inputs, so that the pair cannot be accidentally swapped.
39. As a debugger, I want a stable ChangeView based on Change and Snapshot, so that I can inspect user-level effects without exposing the Rust operation tree.
40. As a test author, I want ChangeView positions in JavaScript UTF-16 coordinates, so that assertions match application behavior.
41. As a test author, I want Noop inspection to return an empty view, so that Noop does not require a special entry variant.
42. As an adapter author, I want ChangeView to expose flat semantic entries, so that I can consume changes without understanding retain/modify internals.
43. As a consumer handling failures, I want one CollaError class with stable code, operation, path and details, so that recovery does not depend on messages.
44. As a TypeScript consumer, I want `error.is(code)` to narrow details, so that error handling remains type-safe.
45. As a consumer handling malformed bytes, I want codec failures grouped under `invalid_encoding`, so that Rust decoder refactors do not break my code.
46. As a consumer enforcing resource policy, I want frozen default Limits and per-call overrides, so that I can bound work without mutable global configuration.
47. As a long-running application, I want Value, Change and Builder handles to support deterministic disposal, so that Wasm memory is released promptly.
48. As a developer using Explicit Resource Management, I want handles to implement Symbol.dispose, so that `using` can release resources automatically.
49. As a developer who forgets disposal, I want FinalizationRegistry to be a fallback, so that leaks are mitigated without treating GC timing as correctness.
50. As a developer sharing immutable handles, I want cheap Value and Change clones, so that independent owners can dispose in any order.
51. As a release engineer, I want Rust and npm packages to share one SemVer, so that bugs and wire behavior map to one implementation version.
52. As a release engineer, I want browser and Node entries to execute the same Wasm binary, so that runtime-specific builds cannot drift.
53. As a release engineer, I want compatibility tests to install the real npm tarball, so that workspace linking cannot hide missing files or exports.
54. As a maintainer, I want wasm-bindgen output to remain private, so that generated names and directory layouts can change without a public breaking change.
55. As a maintainer, I want Rust core isolated from wasm-bindgen dependencies, so that native users do not pay for the JavaScript wrapper.
56. As a maintainer, I want Cargo and pnpm workspaces at the repository root, so that Rust and JavaScript releases can be coordinated from one monorepo.
57. As a maintainer, I want cross-language golden tests to block releases, so that canonical bytes and OT results remain identical.
58. As a maintainer, I want size and performance baselines before setting budgets, so that regression limits are evidence-based rather than arbitrary.

## Implementation Decisions

- The repository will become a dual Cargo and pnpm workspace. The publishable Rust core, private Wasm wrapper and publishable npm facade are separate modules with explicit dependency direction.
- The Rust core remains free of wasm-bindgen dependencies. A private Wasm crate depends on the Rust core, and the TypeScript facade depends on generated private bindings.
- Rust retains a Context abstraction for immutable Limits and operation delegation. JavaScript does not expose Context and is not required to mirror the Rust facade shape.
- The npm package name is `@colla/core`. Versioning is aligned exactly with the Rust crate, and the two artifacts are released atomically.
- The package is ESM-only in v1 and only exposes its root export. Browser, Node, Wasm and internal subpaths are not public.
- Browser/default ESM embeds the Wasm binary as base64 and initializes synchronously during module evaluation. Node.js ESM reads the same binary from a package-relative asset and initializes synchronously.
- The package does not use top-level await or synchronous XHR and does not expose public Wasm initialization. The root module is explicitly side-effectful.
- The stable public API is a handwritten TypeScript facade. wasm-bindgen classes, initialization functions, filenames, generated declarations and error shapes are private ABI.
- Value and Change are immutable Wasm-backed handles with private constructors. They are created through named static factories, Builder output or algebra output.
- `Value.fromJS()` creates Core Value from validated JavaScript input. `Value.decode()` and `Change.decode()` create handles from canonical binary under default or per-call Limits.
- `Value.change()` creates the root ChangeBuilder from a Snapshot. The Builder owns a cheap Rust Arc clone of the Snapshot and does not borrow the JavaScript Value handle.
- The root Builder is linear and disposable. Successful `build()` consumes it. Scoped builders are valid only during synchronous callbacks and commit transactionally.
- Map Builder provides snapshot-aware set and delete. List Builder provides insert, set and half-open delete. Text Builder provides insert, delete and Delete+Insert replace sugar. RichText Builder provides explicit insertText, insertEmbed, delete and format. Int Builder provides checked i64 add.
- JavaScript Text and RichText positions use UTF-16 code units. Rust core and wire use Unicode scalar positions. Invalid surrogate interiors are rejected rather than rounded. RichText Embed counts as one.
- Apply, Compose, Invert, TransformPair and Change inspection are package-level functions. Algebra inputs are never consumed.
- TransformPair requires an explicit left-first or right-first order and returns a readonly pair whose elements correspond to the left and right inputs.
- Value and Change encode methods return fresh JavaScript-owned byte arrays. Decode only reads input during the call and never retains the input buffer.
- ValueData is recursively frozen. Core Map becomes a null-prototype record. Value path reads return copied plain data and do not create Wasm child handles.
- ChangeView is a stable, flat, ordered and recursively frozen Snapshot-relative projection. It uses user semantic operations and JavaScript coordinates, is not a construction API, and is not a persistence format.
- One CollaError class exposes stable reason-oriented lower_snake_case codes, a separate lower_snake_case operation, optional Path and code-specific frozen details. Message text and Rust enum names are not contractual.
- Value, Change and root Builder provide idempotent dispose and Symbol.dispose. FinalizationRegistry is only a nondeterministic fallback. Value and Change provide cheap independent clones; Builder does not.
- Default Limits are exported as a frozen value. Per-call options can provide partial overrides, are read synchronously and are not retained. No mutable global Limits exist.
- The supported baseline is Node.js 20+, Vite 5+ and Rollup 4+. Browser support covers main thread, Dedicated Worker and Shared Worker. CommonJS, Deno, Bun, Service Worker and edge workers are not v1 commitments.
- The first implementation records size and performance baselines. Absolute budgets and regression thresholds are set only after repeatable measurements exist.

## Testing Decisions

- The primary acceptance seam is the release-artifact boundary: the publishable Rust crate and `@colla/core` installed from a real npm tarball at the same version. Tests should prefer this seam over generated binding or facade internals.
- A good acceptance test observes public behavior: canonical bytes, ValueData, ChangeView, CollaError fields, OT results, resource lifecycle and consumer bundling. Tests must not assert wasm-bindgen names, private pointer values or generated directory layout.
- Existing Rust codec, algebra, core-model and property tests remain the lower-level prior art. New Rust Context and scoped Builder tests extend these seams rather than replacing them.
- Cross-language golden tests are a release gate. They cover Rust encode to JavaScript decode, JavaScript encode to Rust decode, byte-for-byte Value/Change equality, and identical Apply/Compose/TransformPair/Invert results.
- Cross-language tests cover UTF-16/code point conversion, surrogate rejection, RichText Embed length and half-open ranges.
- Both runtimes must consistently reject non-canonical varints, invalid UTF-8, unknown tags, trailing bytes, non-canonical operations and Limits violations.
- Packaging fixtures install the packed npm artifact outside the pnpm workspace. They execute real Value construction, Builder, Apply, codec and disposal operations.
- Compatibility fixtures cover the minimum and latest stable Vite versions in development, build and SSR modes; the minimum and latest stable Rollup versions; and Node.js 20+ ESM.
- Rollup fixtures may use ordinary node module resolution but cannot use Wasm, asset-copy or top-level-await plugins.
- Browser tests cover main thread, Dedicated Worker and Shared Worker without DOM-dependent initialization.
- Lifecycle tests cover idempotent disposal, use-after-dispose, clone independence, Builder consumption, scoped Builder escape, Builder independence from the original Value, and failure without partial transform outputs.
- Input-boundary tests cover plain records, null-prototype records, arrays, cycles, accessors, symbol keys, class instances, Date, Set, JavaScript Map, NaN, Infinity, unsafe integer convenience input and out-of-range bigint.
- Builder tests cover transactional callbacks, Noop normalization, snapshot-aware map behavior, list set semantics, text replace expansion and explicit RichText replacement composition.
- Error contract tests assert code, operation, path and required details fields, never message prose or Rust enum formatting.
- CI records raw Wasm size, base64 browser entry size, package size, synchronous initialization time and representative conversion/algebra benchmarks. Thresholds are added after the initial repeatable baseline.

## Out of Scope

- A JavaScript Context object or public Wasm initialization API.
- CommonJS, Deno, Bun, Service Worker, Cloudflare Workers or other edge runtime guarantees.
- Slim, web, node, wasm or internal public package subpaths.
- Document, Session, client identity, operation identity generation, version vectors, history, offline queues, networking or synchronization protocols.
- Editor-specific Quill Delta, ProseMirror, HTML or JSON adapters.
- Cursor, selection, presence or awareness models.
- Mutable Value access, Proxy-based document access or Wasm child handles for path reads.
- Atomic List Move, RichText generic insert, RichText replace or Float Add.
- Recursively editable RichText Embed data; independently editable embed state must live elsewhere and be referenced by stable ID.
- Construction of Change from ChangeView or plain operation objects.
- Wire envelopes containing magic, protocol version, compression, CRC, document ID, author or operation metadata.
- Public stability for wasm-bindgen output, internal ABI or generated file layout.
- Absolute bundle-size or performance thresholds before the first repeatable implementation baseline.

## Further Notes

- The highest-level test seam described above has already been accepted during design: release validation uses the real Rust crate and packed npm artifact, with cross-language golden tests and consumer bundler fixtures.
- The browser base64 strategy intentionally trades approximately 33% raw representation growth and temporary decode memory for synchronous zero-configuration bundler compatibility.
- FinalizationRegistry must never be used for correctness or normal memory control; documentation and examples should prefer explicit disposal or `using`.
- Adding new stable error operation values is allowed. Existing values and required details fields remain compatible.
- Detailed optional error fields, exact build CLI flags and performance thresholds are implementation-stage decisions as long as they preserve this public contract.
