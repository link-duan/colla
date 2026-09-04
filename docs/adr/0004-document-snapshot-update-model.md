# Document 与 Snapshot/Update 模型

状态：Accepted

JavaScript API 提供可变的 `Document`，以及用于本地持久化和应用层传输的
`Snapshot`、`Update`。Snapshot 表示完整可见内容及其 revision；Update 表示基于某个
revision 的一个 Core Change，并携带当前 Document 实例内用于确认关联的 updateId。
模型不引入 client identity，也不把 pending 或同步会话状态写入 Snapshot。

### 核心架构决策

1. **纯 JS 零句柄边界**：`Snapshot` 与 `Update` 是 100% 纯 JavaScript 不可变 POJO 对象，
   自带预编码的 `.bytes: Uint8Array`。应用层在快照存储与网络同步中无需关注 WebAssembly 资源释放
   或生命周期管理（无 `dispose()`、`clone()`、`encode()` 负担）。
2. **原子事务驱动**：本地状态修改统一通过 `doc.transact(tx => ...)` 执行，支持声明式的路径自动补全
   与多步骤原子合成，输出单一规范 `Update`。
3. **响应式订阅与异常隔离**：通过 `doc.subscribe(...)` 统一变更观察与异常隔离，提供标准的取消订阅闭包。
4. **累积确认机制**：`doc.ack(updateId)` 支持累积确认，自动释放直到指定 ID 的全部 pending 操作，
   大幅简化网络控制器对批量 ACK 的处理。
5. **直接线缆消费**：`doc.applyRemote` 原生支持直接接收网络传输的二进制字节流（`Uint8Array`）。

传输、排序、重试、全局去重、会话和编辑器适配由应用层负责。字段、版本、magic、事件和状态转换以
`docs/document-model.md` 为准。
