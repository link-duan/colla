# Map 成员由 Set 写入，Insert 仅用于 List

状态：Accepted · 取代 [0008](0008-path-addressed-core.md) 中同一 Map 键并发 Insert 的规则

此前 Set 只替换已存在的目标，新增 Map 成员需要 Insert。日常 API `tx.set` 是 upsert，
只能在调用时按工作内容把写入转换为 Insert 或 Set：同名不同义，EditResult 中的操作
与用户调用不一致，规范文档也因此出现过错误。`tx.copy` 同样只是读取后写入的语法糖。

## 决策

- **Set 写 Map 成员时是 upsert**：成员缺失则插入，存在则替换；父节点必须是已存在的
  Map。写 List 元素或根时仍是替换，目标必须存在。
- **Insert 只用于 List 间隙**。路径以 Map 键结尾的 Insert 在构造时报
  `invalid_argument`，解码时视为非规范编码；不做隐式转换。
- `tx.set` 与 Set 操作一一对应。Delete Map 成员的逆为 Set 原值；Set 新建成员的逆为
  Delete。
- 移除 `tx.copy`。Value 不可变且无身份，写入已有 Value 即复制其内容。
- 编辑错误在 details 中携带 `path`；常见误用（缺少父节点、用 Set 追加 List）附带
  `hint`。

## 并发等价性

并发双方基于同一 base，对同一 Map 键要么都认为存在、要么都认为缺失，因此同一路径
只会出现：

- Set 对 Set（含双方都新建）：按 Priority，与原先"并发 Insert 变换为 Set"结果相同；
- Set 对 Delete（键已存在）：Delete 胜，规则不变；
- 父节点被 Delete 或 Set：内部操作被丢弃，规则不变。

裁决仅依赖两个操作本身，无需回查 base；transform 因此删去 Map Insert 特例。

## 后果

操作集合与主流项目（Yjs、Automerge、JSON Patch）的心智模型一致；Change 中看到的
操作就是用户调用的操作。代价是失去"写入本应缺失的键"这一调试性校验，且属于破坏性
变更：Map Insert 字节不再可解码，golden fixtures 中 Delete 的逆由 Insert 变为 Set。
