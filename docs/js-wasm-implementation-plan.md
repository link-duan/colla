# @colla/core JavaScript/Wasm v1 实施计划

本计划以 [JavaScript/Wasm v1 API](./js-wasm-api.md) 和 ADR-0001—0008 为边界。
实施应按阶段小步提交，每个阶段保持 Rust tests 可运行，不在一个变更中
同时完成目录迁移、Rust API 重构、Wasm ABI 和 npm 发布。

## 阶段 0：建立可重复基线

工作：

- 记录当前 `cargo test`、property tests、bench 和 codec fixtures 结果。
- 记录当前 crate version、Rust toolchain 和 binary golden bytes。
- 为后续目录迁移准备只检查路径、不依赖旧工作目录的命令。

退出条件：当前 Rust 行为有可对比基线，后续机械迁移不改变规范字节或
OT 结果。

## 阶段 1：迁移为 Cargo + pnpm 双 workspace

目标结构：

```text
colla/
├── Cargo.toml
├── package.json
├── pnpm-workspace.yaml
├── crates/
│   ├── colla/
│   └── colla-wasm/
└── packages/
    └── core/
```

工作：

- 将现有 `src`、`tests`、`benches`、`examples` 和 package manifest 下沉到
  `crates/colla`。
- 根 Cargo manifest 改为 workspace，`default-members = ["crates/colla"]`。
- 建立 private 根 `package.json`、精确锁定 pnpm version，workspace 只包含
  `packages/*`。
- 创建 `colla-wasm` 和 `@colla/core` 空骨架，暂不导出功能。
- 不引入 Turborepo/Nx。

退出条件：根目录执行 Rust tests 与 pnpm workspace 基础命令成功，规范二进制
与迁移前一致。

## 阶段 2：对齐 Rust 函数与输入 API

工作：

- 移除独立 operation facade object，提供与 JavaScript 参数顺序一致的包级 Apply、
  Compose、TransformPair 和 Invert 函数。
- 为 Value/Change 增加默认 `decode()`、显式 `decode_with_limits()` 与实例 `encode()`；
  codec 自由函数继续作为底层 API。
- 将 `Limits` 重命名为 `InputLimits`，只用于外部输入转换和 decode；从代数、Builder
  及其错误类型删除 limits 参数与不可达错误分支。
- 为 Value 增加 `change()`，与不接收 limits 的 `ChangeBuilder::new(&base)` 等价。
- 重构大逻辑序列路径，使 Apply、Compose、TransformPair 和 Invert 不依赖可配置阈值
  仍保持紧凑，并仅在真实语义或算术错误时失败。

测试：

- 包级函数与底层方法产生相同 Value、Change、错误和规范 bytes。
- 默认 decode 与 `InputLimits::default()` 等价，显式输入覆盖只影响 decode。
- Builder 和代数结果可以超过默认 InputLimits，重新 decode 时由接收方策略决定。
- 超大 retain/delete 等逻辑序列不会展开或依赖默认 operation budget。

退出条件：Rust 与 JavaScript 共享函数式公共概念，输入资源策略与 Core 运算语义
分离，且二进制格式未变。

## 阶段 3：实现私有 colla-wasm ABI

工作：

- 使用 wasm-bindgen 导出不稳定的内部 Value、Change 和根 Builder handles。
- 句柄导出 clone/free、codec、查询、Builder 和代数运算所需的最小方法。
- ABI 不保存 facade identity。只有 Value.fromJS 与 Value/Change decode 将 InputOptions
  转为 Rust InputLimits；Builder 和代数 ABI 不接收 limits。
- Builder 内部持有 Snapshot `Arc` clone，不借用 Value facade。
- 将 Rust errors 转为结构化私有 ABI data，不直接暴露 thiserror message。
- 保证 ABI 失败路径释放临时分配，transform pair 不泄漏部分结果。

测试：在 Wasm 层验证 clone/free 顺序、重复 free、Builder consume、错误路径和
InputLimits 只作用于显式输入入口。

退出条件：私有 ABI 能完整驱动 facade，且没有生成名称或 class 被视为公共 API。

## 阶段 4：实现 TypeScript facade

工作：

- 实现 `Value`、`Change`、`ChangeBuilder`、scoped builders 和 `CollaError`。
- 实现 ValueInput 严格归一化、ValueData 递归冻结、null-prototype Map 输出。
- 实现 `text`、`richText`、`int`、`Value.fromJS/decode`、`Change.decode`、
  `Value.change`、代数函数、ChangeView 和坐标函数。
- 实现主动 dispose、`Symbol.dispose`、廉价 clone 与 FinalizationRegistry fallback。
- 所有 Wasm 异常在 facade 边界映射为稳定 CollaError；callback 主动抛出的 JS
  异常原样传播。
- 只从公共 root module 导出规范 API，内部 wrap/unwrap 和 wasm-bindgen types 不导出。
- `InputOptions` 只由 Value.fromJS 与 Value/Change decode 接受；Builder ValueInput
  继续验证 Core 合法性，但规模由消费方保证。

测试：

- TypeScript declaration tests 与 runtime JavaScript misuse tests。
- 所有 ValueInput 允许/拒绝边界，包括 cycle、accessor、symbol、prototype、
  NaN/Infinity 和 i64 越界。
- UTF-16 surrogate 边界，Text/RichText/Embed 坐标，半开 range。
- dispose/clone/build consume/scoped escape 和 FinalizationRegistry 不参与正确性。
- codec byte ownership，句柄释放后 encode 输出仍有效。

退出条件：公共 API 与 `docs/js-wasm-api.md` 对齐，consumer 不需要知道
wasm-bindgen ABI。

## 阶段 5：生成单 Wasm 产物与双 ESM 入口

工作：

- 构建 `colla-wasm` 的单一 release Wasm binary，生成私有 wasm-bindgen web glue。
- Browser build 将最终 Wasm 转为 base64 JS module 并同步初始化。
- Node build 发布同一 Wasm bytes 的独立 `.wasm`，通过 `import.meta.url` 和
  `readFileSync` 同步初始化。
- 生成统一 declarations 和条件 exports：`node` 选 Node 入口，`browser` 选
  browser 入口，`import/default` 使用 browser-safe fallback。
- 设置 `sideEffects: true`，校验 npm tarball 只包含必要文件。
- v1 不提供 slim/web/node/wasm/internal subpath。

退出条件：Browser 与 Node 实际运行的 Wasm bytes hash 一致，包根入口同步
可用。

## 阶段 6：发布包兼容 fixtures

packaging tests 必须先 `pnpm pack`/`npm pack`，然后在 workspace 之外的临时目录
安装 tarball，不得链接源码。

矩阵：

- Node.js 20+ ESM。
- Vite 5 最低版本与当前最新稳定版：dev + build + SSR。
- Rollup 4 最低版本与当前最新稳定版。
- Browser main thread、Dedicated Worker、Shared Worker。

每个 fixture 至少执行 `Value.fromJS()`、Builder、`apply()`、codec round-trip 和
dispose。Rollup 除常规 node resolver 外不使用 Wasm、资源复制或 top-level-await 插件。

退出条件：所有声明支持环境使用真实 tarball 零 Wasm 配置通过。

## 阶段 7：跨语言 golden tests

发布门禁：

- Rust encode → JavaScript decode、JavaScript encode → Rust decode。
- Value/Change bytes 逐字节一致。
- Apply、Compose、TransformPair、Invert 结果一致。
- UTF-16/code point 转换与 surrogate 拒绝一致。
- 非规范 bytes、未知 tag、非最短 varint、非法 UTF-8、尾随 bytes 和 InputLimits
  超限在两端输入边界一致拒绝。

退出条件：Rust crate 与 npm tarball 之间的所有规范边界都由双向 fixture 覆盖。

## 阶段 8：度量、文档与发布

工作：

- CI 记录原始 Wasm、browser base64 entry、tarball 的 raw/gzip/brotli 体积。
- 记录同步初始化、Value conversion、Builder、Apply、Compose 和 TransformPair
  benchmark。
- 首个可重复基线建立后再设置回归门禁与绝对预算。
- 编写 npm README、lifecycle 指南、Vite/Rollup/Node examples 和迁移示例。
- 校验 Rust crate、`@colla/core`、Cargo.lock 与 package lock 的版本一致。
- 以相同 SemVer 原子发布 crates.io 与 npm；任一 registry 预检失败则不开始发布。

退出条件：所有发布门禁通过，版本一致，实际 tarball 文档与行为匹配。

## 建议的提交边界

1. `chore: create cargo and pnpm workspaces`
2. `feat(core)!: align function and input APIs`
3. `feat(wasm): add private wasm bindings`
4. `feat(js): add value and change facade`
5. `feat(js): add fluent change builders`
6. `feat(js): add algebra inspection and errors`
7. `build(js): add browser and node wasm entries`
8. `test(js): add packaging and cross-language fixtures`
9. `docs(js): document @colla/core`

实际 commit subject 应继续遵守仓库 Conventional Commits 约定。
