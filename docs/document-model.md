# Document 高层模型

本文定义 `colla-ot` 高层 Document API 使用的本地内容模型。Core 的 Value、Change
和 OT 语义仍以[核心数据模型](data-model.md)为准。

本文是协议与行为规范。面向应用开发者的创建、恢复、编辑、同步和持久化示例见
[`colla-ot` JavaScript 使用指南](../packages/core/README.md#high-level-document-api)。

## 1. 概念

高层 API 只有三个主要对象：

- `Document`：当前可见内容与本地运行态；
- `Snapshot`：revision 加完整 Core Value 的持久化内容快照；
- `Update`：revision、updateId 和 Core Change 的可交换变更。

`pending`、`local`、`remote` 和 rebase 是 Document 内部状态，不是独立公共类型。

## 2. Snapshot

Snapshot payload 为：

```text
revision: u64
content: Value
```

本地 envelope：

```text
magic: "COLLAS" (6 ASCII bytes)
protocolVersion: u16 little-endian
cocodec((revision, content))
```

当前协议版本为 `1`，payload 是按顺序编码的 cocodec tuple，不包含额外的 enum tag。
项目仍处于早期开发阶段，不承诺历史 bytes 兼容；协议字段变化可直接调整版本或格式。
解码必须拒绝错误 magic、未知版本、非法 payload 和尾随字节。

Snapshot 只恢复内容和 revision，不恢复 pending、ack、rebase 或其他同步运行态。

## 3. Update

Update payload 为：

```text
revision: u64
updateId: u64
change: Change
```

本地 envelope：

```text
magic: "COLLAU" (6 ASCII bytes)
protocolVersion: u16 little-endian
cocodec((revision, updateId, change))
```

`revision` 是 Update 生成时的基准 revision。`updateId` 由 Document 实例从 `1` 开始
单调生成，仅用于本地 pending 与 ack 关联，不承诺跨客户端全局唯一。

## 4. API 分层

高层入口：

```ts
import { Document, Snapshot, Update } from "colla-ot"
```

低层入口：

```ts
import { Change, ValueHandle, apply, transformPair } from "colla-ot/core"
```

高层 change 事件携带 editor-oriented edit steps、origin 和 revision，不暴露 Wasm
Change handle。通过 `on("change", listener)` 和 `on("error", listener)` 订阅事件；
change listener 的异常通过 error 事件报告，不影响已提交的 apply 操作。

`applyRemote` 只接受 `confirmedRevision` 的下一条 Update。远程 Update 应由服务端或
其他上层同步协议串行排序；Document 内部使用固定 `left-first` tie-break，不保证两个
客户端直接交换并发 Update 时自动收敛。ack 按 pending FIFO 顺序处理，Snapshot 不恢复
pending，因此恢复后必须由上层重新建立发送/确认状态。
