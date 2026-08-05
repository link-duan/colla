# Rust Context 与 JavaScript 函数式 facade 分离

Rust 通过 `Context::new(limits)` 组织固定 Limits 与核心运算，并可保留
现有 inherent methods 与自由函数作为底层 API。JavaScript 不暴露 Context；
Apply、Compose、TransformPair 和 Invert 使用包级函数，Value/Change 创建使用
类型上的静态入口，ChangeBuilder 由 Snapshot 的 `change()` 方法创建。两端共享
核心语义而不强求 facade 形状一致，且都不引入 Document 或 Session 状态。
