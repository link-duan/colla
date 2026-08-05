# Workspace 以真实发布产物作为验收边界

仓库根目录同时是 Cargo 与 pnpm workspace：可发布 Rust core、私有 Wasm crate 和
可发布 npm facade 保持单向依赖，不引入 Turbo/Nx。Rust crate 与 npm package 使用
完全相同的 SemVer，registry preflight 必须在任一发布开始前全部成功。

发布门禁从真实 Rust package 与 workspace 外安装的 npm tarball 验证跨语言规范字节、
代数结果、生命周期及 Node/Vite/Rollup/Worker 兼容性。CI 先记录 Wasm、base64 entry、
tarball 体积与同步初始化/代表性操作 benchmark，稳定基线形成后才设置预算和回归阈值。
