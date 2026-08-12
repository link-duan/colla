# Rust 与 JavaScript 使用函数式公共 API

Rust 与 `colla-ot` 使用相同的函数式公共概念。Apply、Compose、Invert 和
TransformPair 使用包级函数；Value 与 Change 通过类型方法 decode、通过实例方法
encode。JavaScript Change 通过 `Change.fromJS()` 或纯 TypeScript
`Change.build()` 创建；Rust 使用 typed constructors。Rust 可以保留等价的 inherent
和 codec 底层入口，但不会保留无语义的兼容参数。

`InputLimits` 只约束外部 Value/Change 输入。默认 decode 使用默认限制，显式入口允许
覆盖；`Change.fromJS()` 与 `Change.build()` 也接受输入限制，并对规范化前的原始
ChangeInput 计数。代数运算和 ChangeView 不接收限制，运算结果不受输入策略限制。
