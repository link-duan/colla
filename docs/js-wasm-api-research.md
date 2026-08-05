# JavaScript/Wasm API 横向调研

调研日期：2026-08-04。

## 结论

“所有公开值都做成 Wasm 句柄”不是普适最佳实践。更稳定的经验是按数据的使用
方式选择边界：一次性转换适合纯函数和普通 JS 数据；需要反复运算、内部共享或
昂贵状态的数据适合句柄；持久化和跨线程传递适合 `Uint8Array`。

Colla 同时具备这三种需求，因此推荐混合边界：

- TypeScript facade 是唯一稳定的公共 API，wasm-bindgen 输出属于内部 ABI。
- Value、Change 和 ChangeBuilder 在重复代数运算中使用 Wasm 句柄。
- `fromJS`/`toJS` 用于应用边界，不作为每次 Apply/Compose 的隐式转换步骤。
- 规范二进制 codec 以 `Uint8Array` 暴露，用于网络、存储、worker 和跨语言测试。
- 正常路径必须主动调用 `dispose()`/`Symbol.dispose`；`FinalizationRegistry` 只作为
  遗漏释放时的非确定性安全网。
- 不在 v1 模仿 Automerge 的 Proxy 文档体验，因为 Colla 不是带身份和历史的
  stateful document engine，这会引入大量属性访问跨 Wasm 边界和额外语义。

## 对比

| 项目 | 主要 API 形态 | 生命周期 | 数据边界 | 对 Colla 的启示 |
| --- | --- | --- | --- | --- |
| Automerge 3.4 | 只读 JS 文档外观 + 函数式 `change(doc, fn)`，内部持有 Wasm backend | `free(doc)`；支持 FinalizationRegistry 时可自动回收 | POJO 初始化、只读文档视图、`Uint8Array` changes/save | 最接近“JS facade 隐藏 Wasm 句柄”；但其 Proxy/文档体验依赖完整 CRDT document engine |
| Loro 1.13 | `LoroDoc`、`LoroText`、`LoroMap` 等状态型类 | 各类暴露 `free()` | `toJSON()` 与 `Uint8Array` import/export | 重复编辑的协同状态保留在 Wasm 中，避免每次复制完整文档 |
| Ywasm 0.27 | `YDoc`、共享类型和显式 transaction 类 | `free()`、`destroy()`、`Symbol.dispose` | JS 值与 binary updates | 句柄适合有身份、事务和订阅的数据；显式资源管理值得借鉴 |
| DuckDB-Wasm 1.33 | TypeScript `AsyncDuckDB`/Connection facade，Wasm 与 worker 被隐藏 | `close()`、`terminate()` | Arrow、buffer、stream | 复杂 Wasm 应提供领域化 facade，而不是公开生成器 ABI |
| SWC Wasm 1.15 | `parse`、`print`、`transform` 等纯函数 | 无用户可见句柄 | string、typed AST、plain options | 一次性计算用纯函数最自然；不适合据此设计反复 Apply/Compose 的 Colla |
| brotli-wasm 3.0 | buffer 纯函数；流式场景另用 class | 流对象有状态 | `Uint8Array` | 同一包可以按一次性与增量场景同时提供函数和句柄 |
| resvg-wasm 2.6 | `Resvg` 和 render result 类 | 生成类暴露 `free()` | string/`Uint8Array` 输入，`Uint8Array` 输出 | 直接暴露 wasm-bindgen 类可用，但把 `free()` 和生成细节泄漏给了消费者 |

## Wasm 工具链事实

wasm-bindgen 会把导出的 Rust struct 表示为生成的 JavaScript class。它会在运行时
支持时使用 weak references/`FinalizationRegistry` 自动回收，但官方仍明确支持
显式释放。wasm-pack 的 bundler、Node.js、web 等 target 会生成不同的加载代码，
因此不应把某一 target 的生成入口当作稳定的跨运行时公共 API。

## 被拒绝的 Context/factory 方案

Wasm 模块初始化是进程或 JavaScript realm 级的一次性动作。将它合并进
`await createContext()` 会混淆生命周期，并让普通 Value/Change 创建不必要地
传播异步性。进一步设计后，Colla 明确拒绝在 Rust 或 JavaScript 中引入 Context：
无 Document/Session 状态的核心运算使用包级函数，Value/Change codec 使用类型与实例
方法，输入限制只存在于明确的外部输入入口。

| 库 | 版本 | 默认 Wasm 准备 | 领域实例创建 | 可选入口 |
| --- | ---: | --- | --- | --- |
| Automerge | 3.4.0 | 条件入口在模块求值时自动初始化；browser bundler 使用 Wasm asset，通用 fallback 使用 base64，Node 使用单独入口 | `init()` / `from()` 同步 | `slim` 入口提供 `initializeWasm()` |
| Loro | 1.13.9 | 按 browser、bundler、base64、Node 分发不同入口 | `new LoroDoc()` 同步 | `web` 入口显式异步初始化 |
| ywasm / Yrs | 0.27.3 | Node CommonJS 导入时同步读取并实例化 Wasm | `new YDoc()` 同步 | 无适合现代 browser bundler 的条件入口 |

Automerge 最值得借鉴：默认入口让业务 API 保持同步，另以 `slim` 入口允许
应用控制 Wasm 来源和异步初始化。Loro 的部分 browser 入口为保持同步 API
使用同步 XHR，会阻塞主线程，不应模仿。其文档也说明 plain Rollup
不会自动复制 `new URL(...wasm)` 指向的资源，因此 raw Wasm asset 方案仍需
通过真实打包 fixture 验证。

若默认入口使用 base64 内嵌 Wasm，可以避免 top-level await 和 asset
发射差异，但会增加约 33% 未压缩体积，并带来解码阶段的额外内存。
高相关的已发布包采用“零配置默认入口 + 显式异步精简入口”来分担
两类需求。

Colla v1 选择优先保证默认入口的零配置与同步性：browser/default ESM
使用 base64 内嵌 Wasm，Node.js 20+ ESM 使用独立 `.wasm` 和条件导出。
v1 暂不提供 `slim` 或公开的异步初始化入口。

首个可用 wrapper 产物完成前不预设任意的绝对体积或耗时预算。CI 应先
持续记录 browser base64 入口体积、原始 Wasm 体积、同步初始化耗时和
核心运算 benchmark；在可重复基线建立后，再设定相对回归门禁与绝对预算。

## 文本索引单位对比

使用 `"😀a"` 对公开 API 的 insert/delete 行为进行验证。该字符串包含 2 个
Unicode scalar values、3 个 UTF-16 code units 和 5 个 UTF-8 bytes。

| 项目 | 公开索引单位 | surrogate pair 行为 | 结论 |
| --- | --- | --- | --- |
| Loro 1.13.9 | 默认 UTF-16；另提供 UTF-8 方法和 `convertPos(unicode/utf16/utf8)` | 落在 surrogate 中间的 insert/delete 报错 | 最适合 JS facade 借鉴：JS-native 默认值、严格边界、显式转换 |
| Automerge 3.4.0 | 实测为 UTF-16 坐标 | 不拆分 emoji；位于 pair 内部的索引会对齐到字符边界 | 兼顾 JS 坐标与 Unicode 完整性，但隐式对齐不如明确拒绝可预测 |
| Yjs | UTF-16 | 允许在 surrogate pair 中间操作，可能产生孤立 surrogate | 与 JS 原生索引完全一致，但可能破坏 Unicode 字符完整性 |
| Ywasm 0.27.3 | UTF-8 bytes | 文档明确 index/length 为 UTF-8 字节数 | 直接暴露 Rust/Yrs 内部坐标，跨 JS 编辑器时需要调用方转换 |

Colla 的建议因此是：JavaScript facade 默认使用 UTF-16，与编辑器和 JS 字符串
坐标一致；转换到核心 Unicode scalar index 时严格拒绝 surrogate pair 中间位置，
不做 Automerge 式隐式取整；另提供显式 UTF-16/Core index 转换工具。Rust API 与
二进制格式继续使用 Unicode scalar value。

## 对 Colla 原建议的修正

保留“TypeScript facade + 内部 Wasm 句柄”，但不把所有交互都限制为类方法，也不
直接公开 wasm-bindgen 类。建议的公共形状是：

```ts
const base = Value.fromJS({ title: text("Draft") })
const change = base
  .change()
  .text(["title"], text => text.insert(5, " v2"))
  .build()
const next = apply(base, change)

const bytes = change.encode()
const decoded = Change.decode(bytes)
const value = next.toJS()
```

所有拥有 Wasm 资源的句柄都应在正常路径显式 `dispose()`；支持 Explicit Resource
Management 的环境可以使用 `Symbol.dispose`。`FinalizationRegistry` 不保证及时
执行，只负责兜住遗漏释放，不能用于正确性或常规内存控制。

Apply、Compose、Transform 和 Invert 以包级函数提供；参数仍是句柄，
不隐式往返整个 JS 对象树。

## 来源

- [wasm-bindgen: Exported Rust Types](https://rustwasm.github.io/docs/wasm-bindgen/reference/types/exported-rust-types.html)
- [wasm-bindgen: Support for Weak References](https://rustwasm.github.io/docs/wasm-bindgen/reference/weak-references.html)
- [wasm-pack build targets](https://rustwasm.github.io/docs/wasm-pack/commands/build.html)
- [@automerge/automerge](https://www.npmjs.com/package/@automerge/automerge)
- [loro-crdt](https://www.npmjs.com/package/loro-crdt)
- [ywasm](https://www.npmjs.com/package/ywasm)
- [@duckdb/duckdb-wasm](https://www.npmjs.com/package/@duckdb/duckdb-wasm)
- [@swc/wasm-web](https://www.npmjs.com/package/@swc/wasm-web)
- [brotli-wasm](https://www.npmjs.com/package/brotli-wasm)
- [@resvg/resvg-wasm](https://www.npmjs.com/package/@resvg/resvg-wasm)
