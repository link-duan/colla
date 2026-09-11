# Colla

Colla 定义不可变结构化内容及其中心化协作编辑领域语言。

## Language

**Core Value**:
具有稳定身份的不可变拥有型内容树；包含标量、容器、Text、RichText 与 Ref。
_Avoid_: JSON value, Document

**Snapshot**:
某一时刻的不可变完整 Core Value；其查询与引用解析仅观察该时刻的内容。
_Avoid_: 已确认服务端状态, SyncSnapshot

**Element ID**:
元素实例的稳定身份，独立于内容、位置和同步请求身份。
_Avoid_: List index, content hash, request identity

**Path**:
特定 Snapshot 中由 Map key 和 List index 组成的临时导航地址。
_Avoid_: Stable identity

**Move**:
改变同一元素的拥有位置，同时保留它及其后代身份的原生结构编辑。
_Avoid_: Copy, unrelated Delete and Insert

**Copy**:
创建一棵具有新拥有型身份的内容副本，并将内部 Ref 重映射到对应新元素。
_Avoid_: Move, identity-preserving restore

**Ref**:
指向同一文档内某个 Element ID 的原子弱引用；允许悬空，解析只前进一步。
_Avoid_: Owning edge, automatic traversal, cross-document reference

**Change**:
按执行顺序排列的规范操作序列；结构目标使用元素身份，序列坐标相对各步内容。
_Avoid_: Event, protocol message, recursive patch

**Edit Steps**:
一次提交的可顺序重放操作投影，保留 Move 的来源身份和移动语义。
_Avoid_: UI diff, UTF-16 cursor events

**Document**:
拥有当前可见内容及本地编辑状态的运行态对象。
_Avoid_: Core Value, server revision

**Transaction**:
在同步作用域内积累并原子提交一组内容编辑的工作状态。
_Avoid_: Long-lived editor, database transaction

**Local version**:
Document 可见内容成功提交的本地顺序号。
_Avoid_: Server revision

**History**:
保存、分组并随远程编辑重基本地撤销与重做意图的状态。
_Avoid_: Authority log, time-based grouping

**SyncSnapshot**:
某个文档在指定服务端 revision 的已确认内容。
_Avoid_: Unconfirmed content snapshot

**SyncSession**:
协调已确认基准、在途请求、缓冲编辑和本地可见内容的客户端状态机。
_Avoid_: Network connection, presence session

**Submission**:
由文档身份、客户端身份、单调序号、基准 revision 和 Change 确定的一次提交请求。
_Avoid_: Element identity, mutable rebased payload

**Commit**:
Authority 正式排序并赋予服务端 revision 的提交；自身 Commit 同时完成请求确认。
_Avoid_: Local edit result, independent ack

**Authority**:
维护单一提交顺序、历史基准和请求去重信息的服务端协作状态。
_Avoid_: Database, transport, authentication service

**SessionCheckpoint**:
足以恢复客户端可见内容、同步状态和已启用 History 的完整会话恢复点。
_Avoid_: Value, SyncSnapshot

**Recovery-required**:
无法安全继续重基时保留本地工作、停止同步发送并等待显式恢复的状态。
_Avoid_: Automatic reset, dropped pending work

**Priority**:
对并发意图需要裁决时采用的一致左右优先规则。
_Avoid_: Timestamp, Element ID order

**Structural conflict**:
并发合并导致拥有环、目标父节点失效或无法安全解决的 Map 占用。
_Avoid_: Silent data removal, any concurrent Move

**Canonical form**:
单个受控对象经规范化后的确定表示；不意味着所有效果相同的多步 Change 都具有相同序列。
_Avoid_: Arbitrary field object, semantic equivalence class

**Wire compatibility**:
版本之间直接交换规范二进制对象的能力，与单版本内编码的确定性分别定义。
_Avoid_: Canonical encoding

**Golden fixtures**:
Rust 与 JavaScript 共享的固定输入、输出和规范字节回归证据。
_Avoid_: Independent implementation proof, normative specification

**CollaError**:
通过稳定 code、operation 和 details 描述失败的公开错误。
_Avoid_: Error message parsing

**String**:
只能整体替换的原子字符串。
_Avoid_: Text

**Text**:
支持按 Unicode scalar 编辑的协作文本；日常 JavaScript 编辑接受 UTF-16 坐标。
_Avoid_: String, grapheme sequence

**Int**:
具有完整 i64 精度并支持 checked Add 的整数。
_Avoid_: JavaScript number

**Float**:
有限 IEEE-754 f64 值，不支持 Add。
_Avoid_: Int, NaN, Infinity

**RichText**:
由文本片段和原子 Embed 构成、内容与属性共同参与 OT 的序列。
_Avoid_: HTML, editor-specific delta

**Embed**:
RichText 中长度为 1 的原子 Core Value。
_Avoid_: Independently editable nested document

**Change position**:
低层 Text/RichText 操作中的 Unicode scalar 坐标，Embed 占一个位置。
_Avoid_: UTF-16 offset, byte offset

**Editing position**:
JavaScript 日常 Text/RichText 编辑中的 UTF-16 坐标，按每步当前内容解释。
_Avoid_: Grapheme index, low-level Change position
