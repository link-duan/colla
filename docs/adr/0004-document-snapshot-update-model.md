# Document 与 Snapshot/Update 模型

状态：Accepted

JavaScript API 提供可变的 `Document`，以及用于本地持久化和应用层传输的
`Snapshot`、`Update`。Snapshot 表示完整可见内容及其 revision；Update 表示基于某个
revision 的一个 Core Change，并携带当前 Document 实例内用于确认关联的 updateId。
模型不引入 client identity，也不把 pending 或同步会话状态写入 Snapshot。

Document 负责本地编辑的乐观应用、pending 变更重基、按服务端顺序应用 remote Update、
acknowledgement 和 change/error 事件。传输、排序、重试、全局去重、会话和编辑器适配
由应用层负责。字段、版本、magic、事件和状态转换以 `docs/document-model.md` 为准。
