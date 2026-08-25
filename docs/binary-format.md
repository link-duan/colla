# Colla 规范二进制 Body 格式

状态：规范性。本文只定义 Value 与 Change 的 canonical body encoding 和严格解码要求。

相关规范：[核心数据模型](data-model.md)定义合法结构，
[OT 性质](ot-properties.md)定义代数语义。

本文只定义 Value 与 Change 的规范 body。magic、版本、CRC、压缩、消息类型和
业务元数据属于调用方信封，不在核心 codec 中。

字节机制（varint、zigzag、字符串、浮点、tag 派发、canonical 排序、结构性 DoS 界限）
由 [`cocodec`](https://crates.io/crates/cocodec) 提供；本文定义 colla 领域类型到其上的
tag 映射与解码约束。

## 1. 目标

- 每个规范内存值只有一个合法字节序列。
- 无状态、单趟编码；严格、受 InputLimits 约束的解码。
- 完整消费输入；拒绝未知 tag、尾随字节和非规范形式。
- 不编码 Arc 共享、引用 ID、字符串字典或压缩状态。

## 2. 原语

无符号整数使用最短 u64 LEB128。i64 使用 zigzag 后再编码 varint。长度和索引
在 wire 中为 u64，decoder 必须检查可转换为当前平台 usize 并满足 InputLimits。

字符串编码为 UTF-8 字节长度 varint，随后是原始 UTF-8。Float 固定使用 8
字节 IEEE-754 little-endian，**原样双射**：NaN 与 Infinity 在构造为 `FiniteF64` 时被拒绝，
负零在解码时被规范化为正零（不再拒绝）。bool 编码为单字节 `00`/`01`。

Map、MapChange、Attrs、AttrPatch 的 key 必须按 Rust 字符串字典序严格递增，
重复或乱序均为非规范编码。

## 3. Value tag

    00 Null
    01 Bool(bool 单字节 00/01)
    02 Int(zigzag varint)
    03 Float(f64 little-endian)
    04 String(utf8 string)
    05 Text(utf8 string)
    06 RichText(span count + spans)
    07 List(count + Value*)
    08 Map(count + (string + Value)*)

RichText span tag：00 Text(string + Attrs)，01 Embed(Value + Attrs)。Attrs 是
无 tag 的 `(count + key + AttrValue)*`。AttrValue tag：00 false、01 true、
02 Int、03 Float、04 String。

## 4. Change tag

    00 Noop
    01 Replace(Value)
    02 Map(MapChange)
    03 List(ListChange)
    04 Text(TextChange)
    05 RichText(RichTextChange)
    06 IntAdd(zigzag varint)

Map entry tag：00 Insert(Value)、01 Delete、02 Modify(Change)。List op tag：
00 Retain、01 Insert、02 Delete、03 Modify。Text op tag：00 Retain、01 Insert、
02 Delete。RichText op tag：00 Retain(len + AttrPatch)、01 Insert(content +
Attrs)、02 Delete。

AttrPatch 是 `(count + key + AttrChange)*`。AttrChange tag：00 Set(AttrValue)、
01 Remove。

## 5. 严格规范检查

解码是**结构性**的：decoder 强制的是**字节 canonical**，不再强制**语义 canonical**。

decoder 必须拒绝（字节 canonical，由 cocodec 保证）：非最短 varint、非法 UTF-8、
未知 tag、尾随字节、乱序或重复 key、非 `00`/`01` 的 bool、非有限 Float、深度超限，
以及长度前缀超过剩余输入。

decoder **不再**在解码期拒绝**语义**非规范形式（零长度 Retain/Delete、空 Insert、
相邻可合并 op、尾部纯 Retain、空类型化 Change、Modify(Noop)、IntAdd(0)、负零）。
这些属语义规范化，由构造 API（`from_ops`/`from_entries`/`from_spans` 等）与
上层 `normalize` 负责；encoder 的输入始终已规范，故正常路径产出的字节仍唯一。

RichText 仍在解码时经 `RichText::from_spans` 合并属性相同的相邻 span、接受空 Text span，
重新编码产生规范 bytes。

InputLimits 作为**解码后**资源预算施加于解出的值（`check_input_limits`），与字节机制解耦。

## 6. API

    value.encode() -> Vec<u8>
    Value::decode(&[u8]) -> Result<Value, CodecError>
    change.encode() -> Vec<u8>
    Change::decode(&[u8]) -> Result<Change, CodecError>

`codec::encode_value`、`codec::decode_value`、`codec::encode_change` 和
`codec::decode_change` 作为等价底层入口保留。

**字节解码不接收可配置 limits**：它只由 cocodec 的内建防御兜底——固定递归深度
（`depth` 超限报 `limit_exceeded`）与「不按不可信长度预分配」。放大攻击被「不能声称
你交付不出的量」免费拆解。

`InputLimits` 只约束**结构化输入**构造路径（JS 绑定的 `Value::fromJS`/`Change::fromJS`，
计量调用方传入的原始未规范化 JS 结构，例如 `sequence ops` 按原始 op 数计），
不定义 Value/Change 的合法大小，也不施加于字节解码或代数结果。`limit_exceeded` 的 `name`
字段稳定，调用方可据此反查触发的上限：

| `InputLimits` 字段 | `name` | 施加路径 |
|---|---|---|
| `max_depth` | `depth` | fromJS + 字节解码（cocodec 固定深度）|
| `max_value_nodes` | `value nodes` | fromJS |
| `max_change_nodes` | `change nodes` | fromJS |
| `max_string_bytes` | `string bytes` | fromJS |
| `max_container_len` | `container length` | fromJS |
| `max_sequence_ops` | `sequence ops` | fromJS |
| `max_sequence_len` | `sequence length` | fromJS |

操作元数据不在 body 内。调用方必须在外部维护文档 ID、版本、作者、时间和
operation ID 等字段。
