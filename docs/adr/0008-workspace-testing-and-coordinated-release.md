# Workspace 以真实发布产物作为验收边界

仓库根目录同时是 Cargo 与 pnpm workspace：可发布 Rust core、私有 Wasm crate 和
可发布 npm facade 保持单向依赖，不引入 Turbo/Nx。Rust crate 与 npm package 使用
完全相同的 SemVer，registry preflight 必须在任一发布开始前全部成功。

两个 registry 不提供跨平台原子事务。两个产物必须从同一不可变 Git tag 构建；若一个
registry 已成功而另一个失败，不撤回或改版已发布产物，而是在修复外部故障后从同一 tag
只重试缺失产物。两边公开产物均回读验证成功前，GitHub Release 保持 draft，发布
milestone 不得完成。

发布门禁从真实 Rust package 与 workspace 外安装的 npm tarball 验证跨语言规范字节、
代数结果、生命周期及 Node/Vite/Rollup/Worker 兼容性。CI 先记录 Wasm、base64 entry、
tarball 体积与同步初始化/代表性操作 benchmark，稳定基线形成后才设置预算和回归阈值。
