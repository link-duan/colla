# Document model

本文定义 `colla-ot` Document 使用的本地内容模型。Value、Change 和 OT
语义仍以[核心数据模型](data-model.md)为准。

本文是协议与行为规范。面向应用开发者的创建、恢复、编辑、同步和持久化示例见
[`colla-ot` JavaScript 使用指南](../packages/core/README.md#manage-document-state)。

## 1. 概念

Document 模型只有三个主要对象：

- `Document`：当前可见内容与本地协同运行态状态机；
- `Snapshot`：纯 JavaScript 不可变快照对象，携带 `revision`、完整可见内容的 JavaScript 投影值及预编码的二进制 bytes；
- `Update`：纯 JavaScript 不可变增量更新对象，携带 `revision`、`updateId` 及预编码的二进制 bytes。

`Snapshot` 与 `Update` 属于纯数据信封，在 JavaScript 层不持有任何 WebAssembly 资源句柄，无需手动 `dispose()`。
`pending`、`local`、`remote` 和 rebase 是 Document 内部运行态，不是独立公共类型。

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

在 JavaScript API 中，`Snapshot` 为只读纯对象：

```ts
interface Snapshot {
  readonly revision: bigint
  readonly bytes: Uint8Array
  readonly value: Value
}
```

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

在 JavaScript API 中，`Update` 为只读纯对象：

```ts
interface Update {
  readonly revision: bigint
  readonly updateId: bigint
  readonly bytes: Uint8Array
}
```

## 4. JavaScript API

所有公共 JavaScript 符号都从同一个包入口导入：

```ts
import {
  Document, Snapshot, Update, Change, ValueHandle, apply, transformPair,
} from "colla-ot"
```

### 4.1 事务变更

本地状态修改必须通过原子事务执行：

```ts
const update = document.transact(tx => {
  tx.set(path, value)
  tx.text(path, textOps)
  tx.list(path, listOps)
  tx.delete(path)
})
```

事务上下文自动处理路径层级补全，所有操作在提交时合成（compose）为单一规范 Change 并生成对应的 `Update`。

### 4.2 事件订阅与错误隔离

Document change 事件携带 editor-oriented edit steps、origin 和 revision，不暴露 Wasm
Change handle。通过 `document.subscribe(subscriber)` 订阅变更：

- `subscriber` 可以是单函数 `(event: DocumentChangeEvent) => void`；
- 也可以是对象 `{ onChange: (event) => void, onError?: (event) => void }`。

`subscribe()` 返回取消订阅闭包 `() => void`。change 监听器的异常通过 onError 报告，
不影响已提交的事务与内部状态。

### 4.3 同步与确认

- `applyRemote` 接受 `confirmedRevision` 的下一条 `Update` 实例或直接接受原始二进制 `Uint8Array`。
  远程 Update 应由服务端或其他上层同步协议串行排序；Document 内部使用固定 `left-first` tie-break
  对本地 pending 变更进行重基。
- `ack(updateId)` 支持累积确认（cumulative ACK），自动确认并清除直到该 `updateId` 的所有本地
  pending Update，推进 `confirmedRevision`。
- Snapshot 不恢复 pending，因此恢复后必须由上层重新建立发送/确认状态。

### 4.4 状态查询

Document 提供零句柄开销的内容与运行态查询：
- `get(path)`、`has(path)`、`kind(path)`：直接读取可见内容；
- `revision`、`confirmedRevision`：当前可见版本与已确认基准版本；
- `hasPending`、`pendingCount`：本地待确认队列状态。
