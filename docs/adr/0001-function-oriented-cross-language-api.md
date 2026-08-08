# Rust 与 JavaScript 使用函数式公共 API

Rust 与 `colla-ot` 使用相同的函数式公共概念。Apply、Compose、Invert 和
TransformPair 使用包级函数；Value 与 Change 通过类型方法 decode、通过实例方法
encode，Builder 从 Snapshot 的 `change()` 创建。Rust 可以保留等价的 inherent 和
codec 底层入口，但不会保留无语义的兼容参数。

`InputLimits` 只约束外部 Value/Change 输入。默认 decode 使用默认限制，显式入口允许
覆盖；代数运算、ChangeView 和 Builder 不接收限制。Builder 直接收到的 ValueInput
由消费方保证规模可信，但仍必须通过 Core 合法性验证。运算结果不受输入策略限制。
