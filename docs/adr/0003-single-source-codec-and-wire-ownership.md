# Canonical codec 单一实现与 wire ownership

状态：Accepted

Canonical Value/Change codec 只有一个事实来源，由 Rust Core 拥有。Wasm 边界传递结构化
Value/Change 数据，JavaScript facade 不复制 varint、tag、排序或字节布局规则；编码和
解码错误由 Core 统一产生并映射到公共错误契约。

Snapshot/Update envelope 与 Core Value/Change body 使用不同的格式。当前仓库处于早期
开发阶段，不为历史 envelope 或 body bytes 提供兼容承诺；具体字节布局和严格解码要求
以 `docs/binary-format.md` 与 `docs/document-model.md` 为准。
