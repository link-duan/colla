# Conformance corpus 方案

状态：设计方案（未落地）。本文规划版本化 Conformance corpus 的格式、错误分类、
runner 契约、版本化规则与落地里程碑；它是实现计划，不是规范性定义。

相关背景：[ADR 0010](../adr/0010-normative-specification-and-conformance.md) 决定 corpus
的地位（Rust 为 reference implementation，JavaScript 必须通过相同 corpus）；
[ADR 0014](../adr/0014-conformance-corpus-format-and-runner-contract.md) 决定本文规划的
中性表示、错误分类与 runner 契约；
[Roadmap](roadmap.md) 将 corpus 与 runner 契约列为 1.0 稳定的前置条件。
本文引用的结构与编码以 [核心数据模型](../data-model.md) 与
[二进制 Body 格式](../binary-format.md) 为准；两者分歧时以规范为准。

## 1. 目标与非目标

Conformance corpus 是**语言中立的机器可执行规范**：一组版本化 fixture，用统一的中性
格式描述"给定输入 → 期望输出 / 规范字节 / 错误分类"，让 Rust 与 JavaScript 两个
实现运行同一份用例，证明它们共享同一数据模型与 OT 语义。

corpus 一次性充当四种角色：机器可执行规范、跨语言一致性证据、回归测试、未来接入
第三种实现的验收基线。

本文只规划**脚手架阶段**：确定 fixture 格式与错误分类、建立目录骨架、为每类操作
提供少量种子用例、跑通 Rust 与 JavaScript 两个 runner 的闭环。以下不属于脚手架：

- 完整覆盖（后续以纯加数据文件的方式增长）；
- 由 fuzz 派生的用例、性能基线；
- 无法用中性 JSON 表达的输入（NaN、Infinity、未配对 UTF-16 surrogate 等），这些
  留给各语言自身的测试；
- Document、Session、同步、编辑器适配等核心之外的行为。

## 2. 目录布局

```
conformance/
  README.md                     # fixture 格式与版本化规则的规范说明
  corpus/
    v1/
      value-codec/*.json        # Value 与规范字节的双向往返
      decode-error/*.json       # 严格解码器拒绝的输入
      apply/*.json              # Apply 结果或错误
      compose/*.json            # Compose 结果或错误
      invert/*.json             # Invert 结果与往返
      transform/*.json          # 成对 Transform 结果与收敛
  runners/
    rust/                       # 遍历 corpus 的 Rust 测试（reference）
    js/                         # 遍历 corpus 的 JavaScript 测试
```

fixture 的 `id` 等于相对 `corpus/vN/` 的路径去掉扩展名（如
`value-codec/map-nested`），全 corpus 内唯一且稳定。

## 3. 中性编码

fixture 用带类型标签的 JSON 表达 Value 与 Change，避免依赖 JSON 原生类型（JSON 无法
区分 Int/Float、String/Text）。

### 3.1 Value

每个 Value 是恰好含一个键的对象，键即类型标签：

| 标签 | 形式 | 说明 |
| --- | --- | --- |
| `null` | `{"null": null}` | |
| `bool` | `{"bool": true}` | |
| `int` | `{"int": "-42"}` | 十进制字符串，保留完整 `i64` 精度 |
| `float` | `{"float": 1.5}` | 有限 JSON 数值；`-0.0` 规范化为 `0.0` |
| `string` | `{"string": "abc"}` | 原子字符串 |
| `text` | `{"text": "abc"}` | 支持字符级 OT 的文本 |
| `richtext` | `{"richtext": [<span>]}` | span 序列 |
| `list` | `{"list": [<value>]}` | |
| `map` | `{"map": {"k": <value>}}` | JSON 对象；键顺序由构造归一化，不承载语义 |

Map、Attrs、MapChange、AttrPatch 用 JSON 对象（可读优先）。「key 严格递增」是编码层的
规范形式，由 §4.1 的 `canonicalBytes` 断言负责，中性 JSON 不承载键顺序语义；构造器
会对键排序。非规范输入（乱序、重复 key）一律走 `decode-error` 的字节输入，不需要用
中性 JSON 表达。fixture 校验必须拒绝重复键（JSON 对象重复键跨语言行为不一致）。

RichText span：

- 文本 span：`{"text": "abc", "attrs": <attrs>}`（`attrs` 可省略）
- embed span：`{"embed": <value>, "attrs": <attrs>}`（`attrs` 可省略）

`attrs` 是 JSON 对象 `{"key": <attrvalue>}`；`attrvalue` 是 `bool`/`int`/`float`/`string`
四种标签的子集，不含 `null`。

### 3.2 Change

每个 Change 是恰好含一个键的对象：

| 标签 | 形式 |
| --- | --- |
| `noop` | `{"noop": null}` |
| `replace` | `{"replace": <value>}` |
| `int` | `{"int": {"add": "5"}}` |
| `map` | `{"map": {"key": <mapEntryChange>}}` |
| `list` | `{"list": [<listOp>]}` |
| `text` | `{"text": [<textOp>]}` |
| `richtext` | `{"richtext": [<richOp>]}` |

- `mapEntryChange`：`{"insert": <value>}` | `{"delete": null}` | `{"modify": <change>}`。
  规范形式按 key 排序且不含 `{"modify": {"noop": null}}`。
- `listOp`：`{"retain": n}` | `{"insert": [<value>]}` | `{"delete": n}` | `{"modify": <change>}`
- `textOp`：`{"retain": n}` | `{"insert": "abc"}` | `{"delete": n}`
- `richOp`：`{"retain": n, "attrs": <attrPatch>}` | `{"insert": <richContent>, "attrs": <attrs>}` | `{"delete": n}`
  - `richContent`：`{"text": "abc"}` | `{"embed": <value>}`
  - `attrPatch`：JSON 对象 `{"key": <attrChange>}`；`attrChange`：`{"set": <attrvalue>}` | `{"remove": null}`

长度 `n` 是 JSON 安全整数范围内的数值；长度溢出等超范围场景通过 `decode-error` 的
字节输入表达，不用巨大的字面量数值。

### 3.3 字节

规范二进制 body 一律编码为小写十六进制字符串，无分隔符（如 `"07a1..."`）。

## 4. Fixture 类型

所有 fixture 共享信封字段：

```json
{ "id": "...", "kind": "...", "corpusVersion": 1, "note": "可选说明" }
```

`corpusVersion` 必须与所在 `corpus/vN` 目录一致。各类型附加字段与 runner 断言：

### 4.1 `value-codec`

```json
{ "kind": "value-codec", "value": <value>, "canonicalBytes": "<hex>" }
```

断言：`encode(value) == canonicalBytes`；`decode(canonicalBytes)` 结构等于 `value`；
`encode(decode(canonicalBytes)) == canonicalBytes`（规范唯一性）。

### 4.2 `decode-error`

```json
{ "kind": "decode-error", "target": "value", "inputBytes": "<hex>",
  "expectError": { "code": "invalid_encoding" } }
```

`target` 取 `value` 或 `change`。断言：解码以映射后的错误分类（见 §5）被拒绝。

### 4.3 `apply`

```json
{ "kind": "apply", "snapshot": <value>, "change": <change>,
  "expect": { "value": <value> } }
```

或期望失败：`"expectError": { "code": "type_mismatch" }`。可选 `changeBytes`
同时锁定 Change 的规范编码。断言：`apply(snapshot, change)` 等于 `expect.value`，
或以 `expectError.code` 失败。

### 4.4 `compose`

```json
{ "kind": "compose", "changes": [<changeA>, <changeB>],
  "expect": { "change": <change> } }
```

或 `expectError`。可选 `snapshot`：当提供时，额外断言
`apply(snapshot, composed) == apply(apply(snapshot, changeA), changeB)`。
断言：`compose(changeA, changeB)` 的规范字节等于 `expect.change` 的规范字节。

### 4.5 `invert`

```json
{ "kind": "invert", "snapshot": <value>, "change": <change>,
  "expect": { "change": <inverse> } }
```

断言：`invert(snapshot, change)` 规范字节等于 `expect.change`；并且
`apply(apply(snapshot, change), inverse) == snapshot`。

### 4.6 `transform`

```json
{ "kind": "transform", "base": <value>, "changeA": <change>, "changeB": <change>,
  "side": "left", "expect": { "aPrime": <change>, "bPrime": <change> } }
```

`side` 是显式 tie-break（`left` 或 `right`），保证确定性。断言：
`transform_pair(changeA, changeB, side)` 等于 `(aPrime, bPrime)`；当提供 `base` 时，
收敛检查 `apply(apply(base, changeA), bPrime) == apply(apply(base, changeB), aPrime)`。

## 5. 统一错误 code

fixture 断言一套**两侧统一的稳定错误 code**。单一事实来源是 `colla` 核心新增的
公开 `ErrorCode`（宏生成 `as_str`，`#[non_exhaustive]`）与各错误的 `.code()`；wasm
facade 与 Rust runner 都取 `.code()`，workspace 内不再有第二份手写映射。下表是该映射的
规格镜像（与核心 `.code()` 一致）。TS 侧 `ErrorCode` union 类型独立手写维护（见 §5.1）。
详见 [ADR 0015](../adr/0015-error-code-classification.md)。

| 统一 code | 覆盖的核心错误 |
| --- | --- |
| `invalid_encoding` | `CodecError` 的 EOF、未知 tag、非最小 varint、非规范形式、无效 UTF-8、尾随字节 |
| `limit_exceeded` | codec `LimitExceeded` 与长度溢出（`InputLimits` / `LengthOverflow`） |
| `type_mismatch` | `ApplyError::TypeMismatch` |
| `missing_key` | `ApplyError::MissingKey` |
| `key_already_exists` | `ApplyError::ExistingKey` |
| `out_of_bounds` | `ApplyError::IndexOutOfBounds` 与 `SequenceOutOfBounds` |
| `integer_overflow` | `ApplyError::IntegerOverflow` |
| `incompatible_change` | `ComposeError` / `TransformError` 的 kind 不兼容、map entry 冲突、长度溢出 |
| `invalid_value` | Value 构造违规（`NonFiniteFloat`、`DuplicateKey`） |

脚手架阶段**只断言 `code`**。`expectError` 可携带 `reason` 等子字段供人阅读，但不作为
断言依据，等 code 稳定后再考虑纳入。runner 遇到未知 `kind` 或未知 `code` 必须硬失败，
防止静默跳过。

### 5.1 完整可观测 code 集与 TS 类型

上表是 **核心操作能产生的 code**（corpus 只断言这个子集）。JS 使用者还能收到三个
**facade 专属** code，核心永不产生：`invalid_state`（对已释放句柄操作）、`invalid_argument`
（JS 入参形状错误）与 `invalid_utf16_boundary`（UTF-16 位置转换越界）。公开的 TS
`ErrorCode` union = 核心 code ∪ 这三个 facade code，在 JS 侧**独立手写维护**一处（不做代码
生成），手工与本节保持一致。

## 6. Runner 契约

- corpus 是唯一数据源，runner 保持薄。二者遍历同一 `corpus/vN/`，按 `kind` 分派，
  只调用各自实现的公开 API。
- **Rust runner** 是 reference implementation（`cargo test`），断言 `err.code().as_str()`
  等于 fixture 的 `expectError.code`。**JavaScript runner** 使用 `colla-ot` 公开 API
  运行同一批 fixture。两者都必须进入 CI。
- 依 ADR 0010：当 JavaScript 与 Rust 对同一 fixture 结果分歧时，作为规范缺陷显式裁决，
  两侧与书面规范同一提交内同步修正。
- 决定性：fixture 完整给出 tie-break `side`，runner 不得依赖时钟或随机源。
- runner 失败信息必须包含 fixture `id` 与期望/实际差异，便于定位。

## 7. 版本化规则

- corpus 按 `corpus/vN/` 目录版本化，`corpusVersion` 字段必须与目录一致。
- 同一版本内**只允许增量新增** fixture。修改既有 fixture 的期望输出，必须是真实的
  规范变更：两个 runner 同一提交更新，若影响规范字节或语义则在 `CHANGELOG` 记录。
- 引入 wire 破坏性版本时，新增 `corpus/v2/` 与 `v1/` 并存，可同时运行以记录兼容边界。
- fixture `id` 稳定唯一，等于相对路径去扩展名。

## 8. 种子用例清单

脚手架以最小但有代表性的集合闭合回路：

- `value-codec`：null、bool、int（含负数与 `i64` 极值）、float、string、text、
  richtext（文本 + embed + attrs）、嵌套 list、嵌套 map。
- `decode-error`：截断（EOF）、未知 tag、非最小 varint、map key 乱序（非规范）、
  尾随字节、超出 limit。
- `apply`：text retain+insert、map modify、list insert/delete、richtext attr retain；
  外加 `type_mismatch` 与 `missing_key` 两个错误用例。
- `compose`：两个 text change、两个 map insert、kind 不兼容错误。
- `invert`：text change 往返、map delete 的逆。
- `transform`：两个并发 text insert，分别以 `left` / `right` tie-break 展示确定性收敛。

## 9. 落地里程碑

- **阶段 0（本文）**：格式、错误分类、目录布局达成一致。
- **阶段 1（已落地）**：`conformance/corpus/v1/` 与 `conformance/README.md`（格式规范）
  已建立，提供 3 个 `value-codec` fixture，Rust runner（`colla-conformance` crate，
  `cargo test -p colla-conformance`）跑通闭环。
- **阶段 2（已落地）**：JavaScript runner（`conformance/runners/js/corpus.test.mjs`，
  `node --test`）与 Rust runner 在同一批 fixture 上同时通过。两个 runner 都已并入
  常规测试：Rust runner 是 workspace 成员（`cargo test --workspace`），JS runner 由
  `colla-ot` 的 `pnpm test` 调用，无需单独的 CI job。
- **阶段 3（已落地）**：种子用例扩展到全部 6 类操作（`value-codec`、`decode-error`、
  `apply`、`compose`、`invert`、`transform`，共 17 个 fixture，规模仍小）。RichText 内容
  的中性转换暂缓，不在当前种子集。
- **阶段 2**：加入 JavaScript runner，两个 runner 在同一批 fixture 上于 CI 同时通过。
- **阶段 3**：把种子用例扩展到覆盖全部 6 类操作（规模仍小）。
- **阶段 4（脚手架之后，持续）**：将现有 Rust / JavaScript 的临时用例迁入 corpus，
  持续扩充覆盖，作为 1.0 稳定门槛的证据来源。
