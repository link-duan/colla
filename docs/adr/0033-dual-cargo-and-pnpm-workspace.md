# 仓库根目录同时是 Cargo 与 pnpm workspace

现有 Rust core 从根目录下沉到可发布的 `crates/colla`，私有 Wasm ABI
wrapper 位于 `crates/colla-wasm`，npm package `@colla/core` 位于
`packages/core`。根目录以纯 workspace Cargo manifest、`private` 根
`package.json` 和 `pnpm-workspace.yaml` 统一管理 Rust 与 JavaScript 发布单元；
pnpm 版本通过 `packageManager` 精确锁定。npm tarball packaging fixtures 不加入
pnpm workspace，避免 workspace link 掩盖发布包问题。v1 使用 Cargo 与
`pnpm -r` 编排，不引入 Turborepo 或 Nx。
