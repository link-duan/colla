# 规范、测试与发布验收边界

状态：Accepted

书面规范是公共行为和 wire 契约的事实来源；golden fixtures 是经过评审的固定回归
证据，用于验证规范字节、OT 结果和稳定错误分类。fixture 不替代规范，也不作为多个
独立实现互证的依据。

发布验收以真实 Rust crate 和 workspace 外安装的 npm tarball 为边界，验证跨语言行为、
Wasm 产物和支持的运行时/打包场景。Rust crate 与 npm package 从同一不可变版本构建；
早期版本的 API 或 wire 破坏性调整通过规范和 CHANGELOG 直接说明，不额外维护迁移层。
