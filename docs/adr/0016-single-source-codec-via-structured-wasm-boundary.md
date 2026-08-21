# wire codec 单一实现：wasm 边界改为结构化，Rust 独占编解码

Status: accepted

在 1.0 冻结 wire 契约前审计发现:同一套 canonical body 格式被实现了三处——
`crates/colla/src/codec/mod.rs`(规范 codec)、`crates/colla-wasm/src/change_input.rs`
(第二个 Rust 字节解码器)、以及 `packages/core/src/index.ts` 里手写的 `ByteWriter` 编码器
与 `ByteReader` 解码器。当前 JS 输入流转为「TS 编成 canonical/input bytes → wasm 再解析」,
即 TS **懂 wire 格式**。三份实现要长期保持逐字节一致,是 1.0 之后代价高昂的兼容隐患。

本 ADR 决定:**wire 格式只保留 `codec/mod.rs` 一处实现**。wasm 边界从「TS 编好的字节」
改为「结构化 JS 值」,由 Rust 负责 `JS 输入 ⇄ Value/Change` 的构造、canonical 化与编解码。
TS 退化为**薄 marshaling 层**,不再包含任何 varint / tag / 排序 / 字节布局知识。

## Considered Options

- **边界表示**:选择 wasm 直接接收既有的 `ValueInput`/`ChangeInput` 结构化 JS 值,用
  手写 `js_sys` 遍历在 Rust 内构造 Value/Change;而非继续让 TS 预编码成字节。理由:只要 TS
  还产出 wire bytes 就必然懂格式。放弃 `serde-wasm-bindgen` 是为了对 `i64→BigInt`、错误
  code(`invalid_argument`/`invalid_value` + path)、tag 校验保留完全控制,并且不给 facade
  新增依赖;风格与既有手写的 `change_input.rs` 一致。核心 `colla` 仍只依赖 `thiserror`。

- **canonical 化归属**:选择完全交给 Rust。`Value` 的 `Map` 是 `Arc<BTreeMap<String,Value>>`,
  `Map::from_entries` 插入即按 UTF-8 字节序排序,与 wire 要求一致;因此 TS 侧的 `compareUtf8`
  key 排序是冗余复制,删除。文本 op 合并、空 change 拒绝、scalar 计数等一并由 Rust 拥有。

- **保留在 TS 的输入校验**:实现确认这类检查是 JS 语义、Rust 侧无法无损复刻的,故保留在 TS 做
  一次**纯校验遍历**(不产字节、不排序、不算 limit):`isRecord`/proto 判定、`ownDataEntries`
  拒绝 getter 与 symbol key、cyclic 检测、`assertWellFormedString`(未配对 UTF-16 代理 →
  `invalid_value`,因 wasm-bindgen 会把孤立代理无声转成 U+FFFD)、以及 `ChangeInput` 的
  字段/额外字段/长度类型校验(`invalid_argument` + context path)。**limit 全部下沉 Rust**
  单一来源。其余(kind 判别、canonical 化、编解码)全在 Rust。

## 边界契约(实现须逐点满足)

- **方法**:`ValueHandle.fromJs(jsValue, limitsJson)` / `toJs()`,
  `ChangeHandle.fromJs(jsValue, limitsJson)` / `toJs()`。`decode`/`encode`(bytes)保持不变。
- **`ValueInput` 判别**:`null`→Null;`boolean`→Bool;**`bigint`→Int**(超 i64 → `invalid_value`);
  **`number`→恒为 Float**(须有限,整数值 number 不视作 int);`string`→String;数组→List;
  `{type:"text",value}`→Text;`{type:"richText",spans}`→RichText;其余 plain object→Map;
  其它 → `invalid_value` "unsupported ValueInput"。
- **i64 跨界**:Int/IntChange.delta/AttrValue.Int 一律用 JS `BigInt` 承载,双向零精度损失。
- **`toJs` 输出**:Int→`bigint`,Float→`number`,与既有 `ValueData`/`ChangeData` 形状一致。
- **错误契约**:限额 → `limit_exceeded`(name/actual/maximum,exact 名称如 `value depth`/
  `value nodes`/`text bytes`/`string bytes`/`container length`/`sequence ops`/`sequence length`);
  值域/形状 → `invalid_value` 或 `invalid_argument`(+ 现有 path/context),与现 TS 行为一致。
- **limits**:仍由接收方经 `limitsJson` 传入;不改变 `InputLimits` 语义。

## Consequences

- wire 格式的单一事实来源 = `codec/mod.rs`;`change_input.rs` 与 TS 的 `ByteWriter`/`ByteReader`
  及全部 `encode*`/`decode*`/`compareUtf8`/`writeAttrs` 删除(约 500+ 行 TS 消失)。
- conformance corpus 的 `canonicalBytes` 从此只需证明这**一个**实现;跨语言字节分歧面被消除,
  而非靠两套实现的差分维持。
- JS↔wasm 每次构造多一次边界穿越(此前是纯 TS 编码),以正确性与可维护性换取少量性能;对 1.0
  地基是正确取舍。
- marshaling 桥不定义 wire 字节,不违反「核心只做 OT primitives、版本协商归信封」的章程。
- **限额语义保持现状**:`fromJs` 沿用旧 `Change.fromJS`/`Value.fromJS` 的行为——在**未规范化的原始
  输入**上逐层校 `InputLimits`(例如 `sequence ops` 按原始 op 数、Text 值报 `text bytes`),而非委派给
  `decode_with_limits`。这与 `decode` 的 post-canonical 校验存在**早于本次重构的历史差异**(紧限额下
  `fromJS` 比 `decode` 更严);统一两者的限额语义是单独的行为变更,不属本次行为保持型重构。
- 既有 Rust 与 JS 测试、corpus runner 作为回归 oracle:行为保持不变则全绿。
