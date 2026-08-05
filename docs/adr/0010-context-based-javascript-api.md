---
status: superseded by ADR-0035
---

# JavaScript API 使用显式 Context

调用方通过同步 `new Context({ limits })` 创建 Context。Wasm 模块由 package
入口在 Context 创建前准备完成，不把 realm 级初始化与可反复创建的领域
实例合并为异步工厂。v1 不提供公开的 `initializeWasm()` 或 `slim` 入口。
context 固定 Limits 并作为
Value、Change 和 ChangeBuilder 的创建与运算入口，避免每次重复传 Limits 或
依赖可变全局配置。Value 与 Change 不绑定创建它们的 Context；同一 realm
内任意存活 Context 都可以对它们运算，并使用接收方 Context 的 Limits。
Builder 只在构建期间依赖创建它的 Context，`build()` 返回的 Change 随后独立。
跨 Worker、realm 或进程时通过规范二进制转移。context 不替代句柄的主动
释放责任。
Context 本身也是可主动释放的 Wasm-backed handle，提供幂等
`dispose()` 与 `Symbol.dispose`；释放后调用 Context 方法抛出 `invalid_state`。
该操作只释放 Context 实例及其 Limits，不卸载 realm 级 Wasm module，也不
级联释放已创建的 Value 或 Change。
这些已有句柄在 Context 释放后仍可以执行自身的读取、编码和释放，
但不能再通过已释放 Context 运算。需要继续运算时，可直接传给同一 realm
内的新 Context，不需要先 encode/decode，也不引入隐式所有权转移或 Context
延迟释放。
