# JavaScript 不公开 Context

`@colla/core` 不导出 JavaScript Context；Wasm 在 package 入口同步准备完成后，
Value 通过 `Value.fromJS()` 或 `Value.decode()` 创建，ChangeBuilder 通过
`base.change(options?)` 创建，Change 也可以通过 `Change.decode()` 创建。Apply、
Compose、Invert、TransformPair 和 Change 检查使用包级函数，特殊
Limits 通过可选、不可变的单次调用 options 传入。这避免为创建 Value
或 Change 先构造无文档状态的 Context，并使 Builder 直接表达“相对此
Snapshot 构造 Change”。Rust 仍保留 Context 以组织 Limits 和核心运算，Wasm
wrapper 可在内部委托它，不要求两端 facade 形状一致。

Value 与 Change 的 constructor 不对消费者公开。Value 的创建入口为
`Value.fromJS(input, options?)` 与 `Value.decode(bytes, options?)`，Change 的解码入口为
`Change.decode(bytes, options?)`；Builder 和代数运算返回的句柄由 facade 内部构造。
这不使 `new Value(input)` 或 `new Change(bytes)` 承担多种不明确的输入语义。
