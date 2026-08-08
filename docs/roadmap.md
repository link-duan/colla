# Colla Core Roadmap

Colla 只提供基础 OT 能力：Core Value、Change、Builder、OT 代数和规范 codec。
Document、Session、历史、同步协议、网络传输、presence 和编辑器适配器不属于本仓库
milestone。路线只承诺 Rust 与 JavaScript 两个官方公共实现。

## Milestone 1：Colla Core 0.1 Release

目标：公开发布并证明 `colla` 与 `colla-ot` 0.1.0 可以从真实 registry 重复消费。

范围：

- 将 Node.js 最低版本从已 EOL 的 20 调整为 22，并验证 Node 22、24。
- 为可发布的 `colla` crate 声明 Rust 1.81 MSRV，并增加 library MSRV gate。
- 两个产物从同一不可变 `v0.1.0` tag 构建，完成 registry preflight。
- 发布 crates.io 与 npm 产物，从公开 registry 回读并执行 Rust、Node、Vite、Rollup、
  Browser/Worker smoke tests。
- 创建 draft GitHub Release；两个公开产物均验证成功后再正式发布 release。
- 补充 changelog、兼容性声明、已知限制和发布/故障恢复说明。

退出条件：两个 registry 上的 0.1.0、`v0.1.0` tag 与 GitHub Release 均存在，公开产物
验证通过。若只成功发布一个 registry，则从同一 tag 重试缺失产物，milestone 保持未完成。

## Milestone 2：Colla Core 0.2 Hardening

目标：利用 1.0 前的调整窗口，根据真实消费验证稳定基础 OT 实现。

范围：

- 修正真实消费者暴露的 Rust/JavaScript API 问题，并提供迁移说明。
- 增强 property、fuzz、malformed input 和跨语言 Conformance corpus。
- 使用 Playwright Chromium、Firefox、WebKit 执行真实浏览器测试；三引擎覆盖主线程与
  Dedicated Worker，Shared Worker 在宿主支持时执行。Service Worker 与 edge worker
  继续排除。
- 验证长时间运行、内存增长、主动 dispose、clone 独立性和错误路径资源释放。
- 为 artifact size 设置基于 Linux CI 基线并保留约 15% 余量的硬预算；性能使用多轮
  median 与宽幅相对门禁，初始只阻止约 2 倍的灾难性回退。
- baseline 只能通过显式评审更新，测试不得自动接受新基线。

退出条件：硬化门禁可重复通过，已知的基础 OT 正确性、资源生命周期、真实浏览器和
性能回退风险均有直接测试证据；不以下载量或等待任意时长作为完成条件。

## Milestone 3：Colla Core 1.0 Stability

目标：在真实使用和 hardening 证据充分后，冻结基础 OT 的长期公共契约。

范围：

- 将数据模型、OT 性质与二进制格式整理为完整的人类可读规范。
- 发布版本化 Conformance corpus 与 runner contract，Rust 与 JavaScript 使用同一套
  fixtures。
- 冻结 Rust/JavaScript 公共语义、API、错误契约和 wire compatibility 承诺。
- 确认 1.0 发布时仍受支持的 Rust、Node、Vite、Rollup 与浏览器基线。
- 完成 1.0 migration guide、稳定性声明和长期维护政策。

退出条件：规范、Conformance corpus、两个官方实现与公开文档一致，所有 1.0 release
gates 通过。1.0 是结果型 milestone；如果 0.2 后仍有真实问题，可以增加必要的 0.x
迭代，但不预设不存在的功能。

## 当前顺序

1. 立即执行 Colla Core 0.1 Release。
2. 发布后进入 Colla Core 0.2 Hardening。
3. 达到稳定证据后推进 Colla Core 1.0 Stability。

任何新 Value/Change 类型或 OT 操作都必须由明确消费者需求单独提案，不自动进入以上
milestone；应用层协作能力始终不进入本仓库路线。
