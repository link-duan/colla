# JavaScript 使用稳定错误与显式资源生命周期

公共失败统一映射为 `CollaError`，稳定字段为 reason-oriented code、operation、可选 path
和冻结 details；message、Rust enum 名及 wasm-bindgen 异常形状不构成契约。
`limit_exceeded` 只来自受 `InputLimits` 约束的 Value/Change 输入入口，不属于代数错误。

Value 和 Change 提供幂等 dispose 与 `Symbol.dispose`，clone 拥有独立释放权。纯
TypeScript Builder 没有 Wasm handle 或资源生命周期。FinalizationRegistry 只作为遗忘
主动释放时的非确定性兜底，不参与正确性或正常内存控制。
