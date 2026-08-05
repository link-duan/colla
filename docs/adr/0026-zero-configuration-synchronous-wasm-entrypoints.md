# JavaScript 默认入口同步且无需 Wasm 插件

Browser/default ESM 入口将 Wasm 以 base64 内嵌并在模块求值时同步
初始化，Node.js 20+ ESM 通过条件导出读取独立 `.wasm`；两者都使
Value/Change 创建与核心运算保持同步。v1 不使用 top-level await 或同步 XHR，也不
要求 Vite/Rollup 消费者安装 Wasm 插件。这以 browser 入口约 33% 的
未压缩体积增长和解码期额外内存，换取零配置、可预测的 bundler
兼容性。发布验证必须安装实际 npm tarball，并覆盖 Vite dev/build、
plain Rollup build 和 Node.js ESM 的真实核心运算。

v1 的明确支持下限为 Node.js 20+、Vite 5+ 和 Rollup 4+。CI 同时
验证每个最低支持版本与当前最新稳定版本。Rollup 测试可使用常规
node_modules resolution，但不使用 Wasm、资源复制或 top-level-await 插件。

Browser 入口不依赖 `window`、`document`、同步 XHR 或网络 API，正式支持
browser main thread、Dedicated Worker 和 Shared Worker。Node.js Worker Threads
使用 Node 条件入口。v1 不承诺 Service Worker 或 edge worker runtime。

根入口在模块求值时准备完整 Wasm，因此 package 显式声明
`sideEffects: true`。任何根入口 import，包括仅导入 `text` 等 helper，都可以
触发 Wasm 加载、解码与同步初始化；v1 不承诺按 named export tree-shake
掉 Wasm，也不允许 bundler 错误删除初始化副作用。
