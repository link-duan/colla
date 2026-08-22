# Golden fixtures 设计

本文说明版本化 golden fixtures 的格式、错误分类与版本化规则。golden fixtures 是
**经过评审的固定回归证据**，不是规范性定义，也不是不同独立实现之间的差分证明；当前
只有一个核心 OT / codec 实现（`colla`），JavaScript facade 经 Wasm 复用它（见
[ADR 0016](../adr/0016-single-source-codec-via-structured-wasm-boundary.md)）。

相关背景：[ADR 0014](../adr/0014-golden-fixtures-format-and-error-codes.md) 决定
中性表示与统一错误 code；[ADR 0015](../adr/0015-error-code-classification.md) 决定错误
code 的单一事实来源；[Roadmap](roadmap.md) 把维护这批 fixtures 列为 1.0 前的证据来源。
本文引用的结构与编码以 [核心数据模型](../data-model.md) 与
[二进制 Body 格式](../binary-format.md) 为准；两者分歧时以规范为准。

## 1. 目标与非目标

golden fixtures 是一组版本化、语言中立的用例，用统一中性格式描述"给定输入 → 固定的
期望输出 / 规范字节 / 错误分类"。同一份 fixture 由两侧消费：`crates/colla/tests/golden.rs`
检查 Rust reference implementation，`packages/core/tests/golden.test.mjs` 检查构建后的
`colla-ot` Node 产物及其 JS↔Wasm 边界。单一数据源是关键——两侧被钉在同一批数据上，
规范字节或结果一旦分歧立即暴露。

fixtures 锁定的是**结果**（规范字节、核心操作输出、tie-break、稳定错误 code），而不是
实现结构。以下明确不属于它的职责：

- 证明 Rust 与 JavaScript 是两套独立算法并互证一致（已不成立）；
- 替代 property tests、fuzz 或完整 malformed-input 专项测试；
- 由 fuzz 派生的用例、性能基线、浏览器运行时覆盖；
- 无法用中性 JSON 表达的输入（NaN、Infinity、未配对 UTF-16 surrogate 等），这些
  留给各语言自身的测试；
- Document、Session、同步、编辑器适配等核心之外的行为；
- 因为当前实现产生了某个结果就自动定义规范。

## 2. 目录布局

```
golden/
  README.md                     # fixture 格式与变更规则的规范说明
  value-codec/*.json            # Value 与规范字节的双向往返
  decode-error/*.json           # 严格解码器拒绝的输入
  apply/*.json                  # Apply 结果或错误
  compose/*.json                # Compose 结果或错误
  invert/*.json                 # Invert 结果与往返
  transform/*.json              # 成对 Transform 结果与收敛
```

两侧的消费代码不在此目录，而是各自 package 的常规测试：
`crates/colla/tests/golden.rs` 与 `packages/core/tests/golden.test.mjs`。

fixture 的 `id` 等于相对 `golden/` 的路径去掉扩展名（如
`value-codec/map-nested`），全集内唯一且稳定。

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

fixture 的 `id` 与 `kind` 不存在文件里，而是由路径推导：`id` 是 fixture 在 `golden/`
下去掉扩展名的路径，`kind` 是其首段目录（如 `apply/map-modify` → id
`apply/map-modify`、kind `apply`）。文件内只写 kind 相关字段与可选 `note`：

```json
{ "note": "可选说明" }
```

各类型附加字段与断言（下面示例以 `目录/` 标出推导出的 kind）：

### 4.1 `value-codec/`

```json
{ "value": <value>, "canonicalBytes": "<hex>" }
```

断言：`encode(value) == canonicalBytes`；`decode(canonicalBytes)` 结构等于 `value`；
`encode(decode(canonicalBytes)) == canonicalBytes`（规范唯一性）。

### 4.2 `decode-error/`

```json
{ "target": "value", "inputBytes": "<hex>",
  "expectError": { "code": "invalid_encoding" } }
```

`target` 取 `value` 或 `change`。断言：解码以映射后的错误分类（见 §5）被拒绝。

### 4.3 `apply/`

```json
{ "snapshot": <value>, "change": <change>,
  "expect": { "value": <value> } }
```

或期望失败：`"expectError": { "code": "type_mismatch" }`。可选 `changeBytes`
同时锁定 Change 的规范编码。断言：`apply(snapshot, change)` 等于 `expect.value`，
或以 `expectError.code` 失败。

### 4.4 `compose/`

```json
{ "changes": [<changeA>, <changeB>],
  "expect": { "change": <change> } }
```

或 `expectError`。可选 `snapshot`：当提供时，额外断言
`apply(snapshot, composed) == apply(apply(snapshot, changeA), changeB)`。
断言：`compose(changeA, changeB)` 的规范字节等于 `expect.change` 的规范字节。

### 4.5 `invert/`

```json
{ "snapshot": <value>, "change": <change>,
  "expect": { "change": <inverse> } }
```

断言：`invert(snapshot, change)` 规范字节等于 `expect.change`；并且
`apply(apply(snapshot, change), inverse) == snapshot`。

### 4.6 `transform/`

```json
{ "base": <value>, "changeA": <change>, "changeB": <change>,
  "side": "left", "expect": { "aPrime": <change>, "bPrime": <change> } }
```

`side` 是显式 tie-break（`left` 或 `right`），保证确定性。断言：
`transform_pair(changeA, changeB, side)` 等于 `(aPrime, bPrime)`；当提供 `base` 时，
收敛检查 `apply(apply(base, changeA), bPrime) == apply(apply(base, changeB), aPrime)`。

## 5. 统一错误 code

fixture 断言一套**两侧统一的稳定错误 code**。单一事实来源是 `colla` 核心新增的
公开 `ErrorCode`（宏生成 `as_str`，`#[non_exhaustive]`）与各错误的 `.code()`；wasm
facade 与 Rust 侧都取 `.code()`，workspace 内不再有第二份手写映射。下表是该映射的
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

fixtures **只断言 `code`**。`expectError` 可携带 `reason` 等子字段供人阅读，但不作为
断言依据。两侧遇到未知 `kind` 或未知 `code` 必须硬失败，防止静默跳过。

### 5.1 完整可观测 code 集与 TS 类型

上表是 **核心操作能产生的 code**（fixtures 只断言这个子集）。JS 使用者还能收到三个
**facade 专属** code，核心永不产生：`invalid_state`（对已释放句柄操作）、`invalid_argument`
（JS 入参形状错误）与 `invalid_utf16_boundary`（UTF-16 位置转换越界）。公开的 TS
`ErrorCode` union = 核心 code ∪ 这三个 facade code，在 JS 侧**独立手写维护**一处（不做代码
生成），手工与本节保持一致。

## 6. 消费约定

- fixtures 是唯一数据源，两侧测试保持薄。二者遍历同一 `golden/` 树，按 `kind` 分派，
  只调用各自的公开 API。
- **Rust 侧**（`crates/colla/tests/golden.rs`）检查 reference implementation，断言
  `err.code().as_str()` 等于 fixture 的 `expectError.code`。**JavaScript 侧**
  （`packages/core/tests/golden.test.mjs`）用 `colla-ot` 公开 API 与构建产物运行同一批
  fixture。两者都进入常规测试与 CI（`cargo test --workspace` 与 `pnpm test:js`）。
- 当 JavaScript 与 Rust 对同一 fixture 结果分歧时，作为缺陷调查；若涉及规范，与书面规范
  同一提交内同步修正，且书面规范拥有权威（fixture 不因当前实现结果自动定义规范）。
- 决定性：fixture 完整给出 tie-break `side`，测试不得依赖时钟或随机源。
- 失败信息必须包含 fixture `id` 与期望/实际差异，便于定位。

## 7. 变更规则

- 当前 pre-1.0 阶段**只允许增量新增** fixture。修改既有 fixture 的期望输出，必须是真实的
  行为变更：两侧同一提交更新，若影响规范字节或语义则在 `CHANGELOG` 记录。
- fixtures **不做目录版本化**（无 `vN/` 层）。若未来 wire 破坏性修订真需新旧向量并存，
  再在那时引入版本层；当前 YAGNI。
- fixture `id` 稳定唯一，等于相对 `golden/` 的路径去扩展名。

## 8. 当前用例与待补盖面

当前共 17 个 fixture，以最小但有代表性的集合闭合回路：

- `value-codec`：int（含负数）、string、嵌套 map。
- `decode-error`：截断（EOF）、尾随字节。
- `apply`：text retain+insert、map modify、list insert/delete；外加
  `type_mismatch` 与 `missing_key` 两个错误用例。
- `compose`：两个 text change、两个 map insert、kind 不兼容错误。
- `invert`：text change 往返、map delete 的逆。
- `transform`：两个并发 text insert，分别以 `left` / `right` tie-break 展示确定性收敛。

RichText 内容（text span + embed + attrs、attrPatch）的中性转换**尚未实现**，两侧对
`richtext` tag 目前都硬失败；它是下一步扩充的首选项。bool、float、text value、嵌套 list
等 canonical bytes，以及更多核心 `ErrorCode`（`key_already_exists`、`out_of_bounds`、
`integer_overflow`、非最小 varint、未知 tag、map key 乱序、Change 截断等）也尚未进入用例集。

## 9. 演进

golden fixtures 已从脚手架落地为两侧共享、并入常规测试的回归集：

- 单一数据源 `golden/`（共 17 个 fixture），覆盖全部 6 类操作
  （`value-codec`、`decode-error`、`apply`、`compose`、`invert`、`transform`）。
- Rust 侧 `crates/colla/tests/golden.rs`（`cargo test --workspace`）与 JS 侧
  `packages/core/tests/golden.test.mjs`（`pnpm test:js`）在同一批 fixture 上同时通过，
  无需单独的 CI job。
- 后续（持续）：按 §8 的待补清单扩充覆盖（首先 RichText），将现有 Rust / JavaScript
  的高价值临时用例适当迁入 fixtures，作为 1.0 稳定门槛的证据来源。新 fixture 至少
  满足一项：锁定 canonical bytes / canonical Change、表达重要 OT 或 tie-break 语义、
  锁定公开 `ErrorCode`、覆盖语言绑定边界、或记录已发生的兼容性 bug。
