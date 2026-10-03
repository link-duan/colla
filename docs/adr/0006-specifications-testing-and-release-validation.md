# 规范、测试与发布验收边界

状态：Accepted

书面规范是公共行为和 wire 契约的事实来源；golden fixtures 是经过评审的固定回归
证据，用于验证规范字节、OT 结果和稳定错误分类。fixture 不替代规范，也不作为多个
独立实现互证的依据。

发布验收以真实 Rust crate 和 workspace 外安装的 npm tarball 为边界，验证跨语言行为、
Wasm 产物和支持的运行时/打包场景。Rust crate 与 npm package 从同一不可变版本构建；
API 和 wire 调整通过规范和 CHANGELOG 直接说明。

验收覆盖路径变换、ListMove、Unicode 边界、代数性质、多操作并发与至少三个客户端随机模拟，
包括重复、延迟、丢失、revision 缺口、双端重启、历史裁剪、恢复和分组撤销。共享 fixtures、
畸形输入 fuzz、长期内存及真实产物安装测试共同提供证据。Rust tests/clippy/fmt/doc、
JavaScript 类型及产物测试、Node/浏览器/Worker/打包器验证和文档检查均为交付门槛。
产物与依赖遵守[资源约束](../internal/documentation.md#artifact-constraints)。
