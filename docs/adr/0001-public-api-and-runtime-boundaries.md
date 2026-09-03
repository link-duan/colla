# 公共 API 与跨运行时边界

状态：Accepted

Colla Core 是 Rust 实现的 Value、Change 和 OT 基础能力；私有 Wasm 层只提供跨语言
边界，JavaScript facade 负责面向消费者的 API。Rust Core 不依赖 Wasm 或 JavaScript，
公共 JavaScript API 不暴露生成类、指针、初始化函数或内部 ABI。

JavaScript 统一提供单一同步 ESM 包入口 `colla-ot`，同时暴露 Document 状态模型与
不可变 Value/Change 及 OT 代数能力。运行时相关的 Wasm 加载方式属于内部实现，内部
共享同一个 Wasm runtime。
