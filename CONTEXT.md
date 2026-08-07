# Colla

Colla 定义不可变嵌套文档及其 Operational Transformation 领域语言，使不同语言
的实现共享同一套变更语义。

## Language

**Colla Core**:
定义 Core Value、Change、Builder、OT 代数和规范 codec 的基础能力边界；它不拥有
Document、Session、历史、同步协议、网络传输或其他应用层协作状态。
_Avoid_: Collaboration runtime, document framework

**Core Value**:
Colla 支持的不可变、封闭值树；它不等同于任意 JavaScript value。
_Avoid_: Document, JSON value

**Snapshot**:
某一时刻的完整 Core Value，作为 Change 的基准状态。
_Avoid_: Document state

**Change**:
相对于 Snapshot 的规范化前向操作；它不包含旧值、版本、作者或 operation identity。
_Avoid_: Patch, event, command

**Change View**:
结合 Change 与其 Snapshot 派生的只读投影；它不是 Change 的规范表示或构造输入。
_Avoid_: Change Data, serialized Change

**Path**:
相对于特定 Snapshot 的临时 Map key/List index 导航地址，不属于 Change。
_Avoid_: Change address

**Tie-break**:
上层为无法由内容决定顺序的并发 Change 提供的一致、确定性左右优先规则。
_Avoid_: Timestamp, operation identity

**Canonical form**:
同一 Colla 语义唯一合法的结构与编码表示。
_Avoid_: Normal-looking form

**Wire compatibility**:
不同 Colla 版本能否直接交换或读取规范二进制 body；它独立于单一版本内的
Canonical form，不因编码唯一就自动成立。
_Avoid_: Canonical encoding

**Conformance corpus**:
版本化的规范输入、输出、规范字节和错误 fixtures，用于验证不同 Colla 实现遵守
同一数据模型与 OT 语义。
_Avoid_: Unit test snapshots

**String**:
只能整体替换的原子字符串，不参与字符级 Operational Transformation。
_Avoid_: Text

**Text**:
按 Unicode scalar value 定位并支持字符级 Operational Transformation 的文本值。
_Avoid_: String

**Int**:
具有完整 `i64` 精度并支持 checked Add 的整数值。
_Avoid_: JavaScript number

**Float**:
有限 IEEE-754 `f64` 值；它不支持 Add。
_Avoid_: Int, NaN, Infinity

**Embed**:
RichText 中长度为 1 的原子 Core Value；它可以整体插入、删除或替换，但不能在
RichText 内递归修改。
_Avoid_: Nested collaborative document

**RichText**:
由规范化 Text span 与 Embed span 构成的序列，内容和格式属性共同参与 OT。
_Avoid_: Quill Delta, HTML

**Text position**:
Text 或 RichText 序列中的逻辑位置；Rust 核心按 Unicode scalar value 表示，
JavaScript facade 按 UTF-16 code unit 表示并在边界处转换。
_Avoid_: Grapheme index, byte offset
