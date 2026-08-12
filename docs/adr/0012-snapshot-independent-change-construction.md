---
status: accepted
---

# JavaScript Change 使用 Snapshot-independent typed 构造

JavaScript 使用 `Change.fromJS(ChangeInput)` 作为低层 typed 构造入口，并用纯
TypeScript `Change.build(callback)` 生成同一输入。Builder 不持有 Snapshot 或 Wasm
handle，不解析 Path、不执行 apply/compose，也不自动 upsert；Snapshot 类型、key 和
范围兼容性延迟到 `apply()` 检查。

两种入口通过私有 construction payload 调用 Rust typed constructors，由 Rust 统一完成
规范化和 checked length accumulation；`InputLimits` 针对规范化前的原始输入统计。
