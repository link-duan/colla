# 0017 — 采用 cocodec，抛弃解码期语义 canonical

状态：已接受。取代 0016 中「colla 自持完整 codec」的部分。

## 背景

colla 早期自持一套手写的 canonical 二进制 codec（`put_varint`/`Decoder`/逐类型
encode/decode + InputLimits 解码期计量）。同时抽出了独立通用库
[`cocodec`](https://crates.io/crates/cocodec)（严格 canonical、no_std、内建抗 DoS）。
本 ADR 决定 colla 迁移到 cocodec。

## 决定

- **byte 机制交给 cocodec**：领域类型 `#[derive(cocodec::Encode, cocodec::Decode)]`
  （`transparent` 用于 Arc-newtype，显式 `tag` 用于 enum）；`FiniteF64`/`RichTextChunk`/
  `RichSpan`/`RichText` 因缓存/bits 数据保留少量手写 impl。`codec/mod.rs` 退化为
  cocodec 的薄封装 + `From<cocodec::Error> for CodecError`。
- **破坏性 wire 升级**：tag 重排为干净序列，Bool 不再是「两个 tag」的 hack。由于 colla
  尚处早期、无外部 consumer，接受一次性破坏。golden 向量（Rust + JS）随之重生成。
- **抛弃解码期语义 canonical**：解码变为**结构性**——不再在解码时拒绝零长 op、空 insert、
  相邻可合并 op、Modify(Noop)、负零等。这些语义规范化归构造 API 与 `normalize`
  （契合 cocodec 的字节/语义 canonical 分界）。`-0.0` 解码时由 `FiniteF64::new` 归一为
  `+0.0`（不再拒绝）。RichText 仍在 `from_spans` 时合并相邻同属性 span。
- **字节解码不再接受 InputLimits**：`decode_*` 仅由 cocodec 内建防御（固定 MAX_DEPTH +
  不预分配）兜底，签名与 cocodec 一致。删除 `decode_with_limits` 与领域类型上的
  `check_input_limits`。`InputLimits` 类型保留，但**只约束结构化输入**（JS 绑定的
  `fromJS` 路径），不再施加于字节解码。wasm `decode(bytes, limits)` → `decode(bytes)`，
  TS/JS 同步去掉 limits 参数。

## 结果

- colla 删去约 750 行手写 codec；字节规则、fuzz、跨语言一致性由 cocodec 统一保证。
- 破坏 wire 是一次性成本，无 consumer 承担。
- 解码更宽松（结构性）；调用方若需语义 canonical，用构造 API 或 `normalize`。
