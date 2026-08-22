# 核心库拥有 ErrorCode 分类；TS 类型独立维护

Status: accepted

Golden fixtures 需要 Rust reference implementation 与 JavaScript facade 断言同一套稳定错误 code（见
[ADR 0014](0014-golden-fixtures-format-and-error-codes.md)）。为让这套分类有单一
权威出处，本 ADR 决定：`colla` 核心新增公开 `ErrorCode`（由仓库内 `macro_rules!` 从一处
code 列表生成 `as_str`，标 `#[non_exhaustive]`），并给每个核心错误枚举加穷尽 match 的
`.code()`；`colla-wasm` facade 与 Rust 侧都取 `.code()`，workspace 内不
再有第二份手写映射。TS 侧的 `ErrorCode` union 类型**独立手写维护**（不做代码生成），覆盖
核心 code 与三个 facade 专属 code（`invalid_state`、`invalid_argument`、
`invalid_utf16_boundary`），手工与核心分类保持一致。golden fixtures 只断言核心操作能产生的 code 子集。

## Considered Options

- **映射位置**：选择放进 `colla` 核心 public API，而非只留在 wasm。既然 Rust 是
  reference implementation（ADR 0010），错误分类就应由核心库拥有；这会把 `ErrorCode`
  纳入 1.0 契约，但换来 workspace 内单一来源、消除 wasm facade 与 Rust 侧两份手写映射的漂移。
- **`as_str` 实现**：选择仓库内 `macro_rules!` 生成，而非引入 `strum` derive，守住核心
  crate 仅依赖 `thiserror` 的极简度。
- **TS 类型**：选择独立手写维护，而非从 Rust 生成 + drift 校验。生成方案更严谨但要自制
  管线；权衡后取简单，接受 TS union 与 Rust 手工同步的成本——它只是 DX 类型，golden fixtures
  才是行为一致性的强约束。

## Consequences

`ErrorCode` 与各错误的 `.code()` 是新增 1.0 公开 API，落地时记入 `CHANGELOG`。TS
`ErrorCode` union 可能与 Rust 漂移（手工维护），属可接受的低风险：golden fixtures 断言的是行为层
的 code，而非 union 类型的完整性。golden-tests 设计 §5 的 code 表是核心 `.code()` 的规格
镜像。
