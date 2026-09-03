# 稳定错误与 Wasm 资源生命周期

状态：Accepted

公共失败统一映射为 `CollaError`，稳定契约包含 `code`、`operation`、可选 `path` 和
冻结 `details`。Core 拥有错误分类，JavaScript facade 只负责跨运行时映射；错误消息、
内部 Rust enum 名称和 Wasm 异常形状不构成契约。

Wasm-backed JavaScript 对象默认依赖 JavaScript GC 与生成的 finalizer 回收。facade 不
重复维护 FinalizationRegistry；`dispose()` 与 `Symbol.dispose` 作为可选的确定性释放
入口保留，并保持幂等、独立 clone ownership 和 use-after-dispose 检查。库内部的临时
handle 仍在操作边界显式释放；事件监听器则通过 `unsubscribe()` 管理。
