# 书面规范定义 1.0 契约，golden tests 提供回归证据

Status: accepted

到 1.0，`docs/data-model.md`、`docs/ot-properties.md` 和 `docs/binary-format.md` 是人类
可读的、唯一的规范性定义。版本化 golden fixtures 是经过评审的固定回归证据：它们锁定
规范字节、核心操作结果、tie-break 与稳定错误分类，供 `colla` reference implementation
（`crates/colla/tests/golden.rs`）与 `colla-ot` JavaScript facade
（`packages/core/tests/golden.test.mjs`）共同验证。

自 [ADR 0016](0016-single-source-codec-via-structured-wasm-boundary.md) 起，仓库只有一个
核心 OT / codec 实现，JavaScript 经 Wasm 复用它；因此 golden tests 不再是"两套独立实现
互证"，而是把同一实现及其 facade 钉在一批评审过的固定结果上。fixtures 不因某个实现恰好
产生某结果就自动定义规范：当 fixture 与书面规范分歧时，以书面规范为准，并把 fixture 作为
缺陷在同一提交内修正。
