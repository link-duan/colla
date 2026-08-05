# ChangeBuilder 使用 scoped fluent DSL

JavaScript `snapshot.change(options?)` 返回线性 fluent Builder；根级链通过 `text`、
`richText`、`map`、`list`、`int` 和 `replace` 进入同步 scoped callback，类型专属操作
在临时类型化 ChangeBuilder 上链式表达。该 Builder 不能逃逸 callback、无需单独
释放，且一次
callback 内的操作作为事务提交：任一步失败则本次 callback 不产生修改。Builder
内部可以原地更新以减少 Wasm 分配，但 Snapshot 保持不变，`build()` 最终产生不可变
Change 并消费 Builder。v1 不公开扁平的 Rust 风格 `textInsert(path, ...)` 方法。

根 Builder 在 `snapshot.change()` 时持有 Snapshot 的廉价 Rust `Arc` clone，不借用
JavaScript Value facade；因此创建 Builder 后可以立即释放原 Value。`build()` 或幂等
`builder.dispose()` 释放 Builder 持有的 Snapshot 与其他 Wasm 资源。
