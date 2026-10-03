# 公共 API 与跨运行时边界

状态：Accepted

Colla Core 是 Rust 实现的 Value、Change 和 OT 基础能力；私有 Wasm 层只提供跨语言
边界，JavaScript facade 负责面向消费者的 API。Rust Core 不依赖 Wasm 或 JavaScript，
公共 JavaScript API 不暴露生成类、指针、初始化函数或内部 ABI。

JavaScript 统一提供单一同步 ESM 包入口 `colla-ot`，同时暴露 Document 状态模型与
不可变 Value/Change 及 OT 代数能力。运行时相关的 Wasm 加载方式属于内部实现，内部
共享同一个 Wasm runtime。

JavaScript 普通 string 表示原子 String，Text/RichText 使用显式 marker。输入形状、
资源限制和稳定错误分类属于公共边界；规范化、语义校验、OT 及所有编码由 Rust 拥有。
Wasm 边界传递结构化数据，JavaScript 不复制 varint、tag、排序或字节布局规则。

Change 构造不依赖内容快照；apply、投影及坐标转换在需要时显式接收内容基准。
低层 Text/RichText 操作和 Edit Steps 使用 Unicode scalar 坐标，JavaScript 日常编辑
使用 UTF-16 坐标。

公共错误通过 CollaError 的 code、operation 和冻结 details 表达。错误消息、内部
Rust enum 名称和 Wasm 异常形状不构成匹配契约。不可变对象依赖 JavaScript GC 与
wasm-bindgen 生成的 finalizer 回收，不公开 dispose 或 clone 句柄生命周期；运行态
提供幂等 close，监听器通过取消订阅管理。
