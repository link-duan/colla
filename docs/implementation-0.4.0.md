# Colla 0.4.0 实现契约

本次升级按已确认的破坏性设计实施。发布是实现完成后的独立动作。

## 产物和依赖

保持产物极小是实现约束。JavaScript 包继续不引入运行时 npm 依赖。Core 的规范编码
沿用 `cocodec`，不增加 serde/JSON 编解码路径。新增直接或传递依赖必须说明必要性，
检查目标平台和默认 features，并比较最终产物；构建和测试工具不计入运行时依赖。

分别记录原始、gzip、Brotli 的 Wasm 和浏览器入口体积，以及实际 npm tarball。
性能比较覆盖编辑、重基、编码、初始化和长期内存；不能仅比较 Rust 源码行数或 crate 数。
优先复用既有底层算法、平台能力和结构共享，避免同时交付新旧公开实现。

0.3.0 基线使用 Git 提交 `d0ffba93386d44c1b7062647d589a97a946f0709` 的独立源码构建，
结果保存在 `docs/quality/0.3.0-baseline.{json,md}`。基线中的旧名称描述旧版本，
0.4.0 的公开 API 仅使用 `transform(base, left, right, priority)`。

## 模块边界

Rust 实现身份、不可变内容、操作序列、校验、OT、编辑状态、History、Sync 和所有 codec。
JavaScript 实现同步输入转换、事务作用域和订阅分发，不复制状态转换或协议编码。
网络、数据库、鉴权、presence、编辑器适配和跨文档同步由应用提供。

Core 公开 `Value`、`Change`、`ElementId`、`Ref`；Editing 公开 `Document`、
`Transaction`；History 是 Document 的可选原子参与者；Sync 公开 `SyncSession`、
不可变 `Authority` 和受控协议对象。旧 API 与版本 1 二进制格式不保留兼容层。

## 身份与代数

- 根和 Map/List 可独立寻址的所有元素拥有稳定 ID。Text/RichText 字符仍使用 scalar 坐标。
- 编辑和 set 保留目标身份；set 导入的后代、插入和 Copy 分配新 ID。Move 保留整个子树。
- ID 使用随机命名空间与单调序号；事务回滚不回退分配器；恢复、Undo、Redo 保留原身份。
- Copy 重映射复制范围内的 Ref；外部目标不变。set 的源根映射到保留的目标根。
- Ref 同文档、弱引用、允许悬空，单次 resolve 只前进一步；快照只解析自身内容。
- 所有权保持树，Ref 图允许环。按 ID 和反向引用的索引是派生数据，不进入规范编码。
- 完整相等性包含拥有型身份；contentEquals 忽略拥有型身份，但仍按目标 ID 比较 Ref。
- `apply(base, change)` 原子执行；`invert(base, change)` 恢复内容和身份；
  `compose(base, first, second)` 与顺序执行等价。
- `transform(base, left, right, { priority })` 返回 `[leftAfterRight, rightAfterLeft]`。
  可合并场景满足 TP1，不承诺 TP2 或任意点对点合并。

Move 的源和目标父节点先按 Path/ID 解析；List 目标 index 按移除源后的列表解释。
根、自身及后代不能作为非法移动目标，Map 目标键必须空闲，同位置移动为 Noop。
并发内容修改跟随身份；同元素竞争由优先级裁决；基准祖先删除优先；不冲突的 Move 均保留。
合并产生拥有关系环、失效目标父节点或不安全的 Map 占用返回 structural_conflict。
不能删除无关内容或拒绝规定可合并的操作来规避变换。

## 编辑与恢复

Document 的本地 version 和 Authority 的 revision 分开命名。每次 edit 同步执行，
在内容、inverse、editSteps、History 和同步输出全部计算成功后一次提交。
Noop 不推进 version 或触发事件。嵌套事务、thenable、远程重入、事务中 close 和事件
分发中的修改均拒绝；所有逃逸编辑对象在作用域结束时失效。

JS 日常 Text/RichText 编辑使用 UTF-16，每一步以当前事务内容转换为 scalar，拒绝代理对
中间位置和越界。Value、Change、Ref、事件及快照不可变，不暴露 Wasm 句柄或要求 dispose。
运行态 close 幂等，不能使已返回快照失效。监听器异常隔离，错误使用稳定 CollaError。

Value 仅表示内容。SyncSnapshot 包含 documentId、服务端 revision 和已确认 Value。
SessionCheckpoint 包含完整确认基准、会话身份、序号、原始在途请求、重基后的在途 Change、
缓冲、本地 version 和已启用 History。原始重试身份和 bytes 永不被重基覆盖。

Authority 按提交顺序重基并去重，accept 返回新状态和 Commit/Rejection。自己的 Commit
同时确认请求；重基成 Noop 的请求仍正式提交。缺口返回缺失区间且内容不变；重复幂等。
过期日志、结构拒绝和无法重基进入 recovery-required，停发且保留本地编辑/导出能力。
恢复需要应用取得新 SyncSnapshot 并显式合入本地工作，库不自动丢弃 pending。

History 默认每次本地事务一单元、容量 100，支持显式连续分组。远程、undo、redo 结束分组。
逆变更随远程优先重基，失效 Noop 跳过；undo/redo 作为正常本地变更进入同步。
撤销插入父容器会删除其中后来的远程编辑。独立 History 和 Session 都支持严格检查点恢复。

所有协议和持久化对象仅由受控构造或严格解码产生，使用类型标识和版本 2。
解码验证 u64、身份唯一性、结构、资源限制、消息类型与尾随字节，不保留可变输入缓冲。

## 交付门槛

依次完成身份模型及规范、Rust Core、编辑引擎、Sync/History、JS facade 和全部产物迁移。
验收包括原四类状态损坏回归、Move/Ref 身份、完整值类型和 Unicode 边界、代数性质、
多操作并发 Move、至少三个客户端随机模拟、重复/延迟/丢失/缺口/双端重启、历史裁剪、
恢复与分组撤销、共享 fixtures、畸形输入 fuzz、长期内存、包外安装及 Node/浏览器/Worker/
打包器验证。Rust tests/clippy/fmt/doc、JS 类型与产物测试、浏览器 E2E 和文档检查必须通过。
版本统一为 0.4.0，更新 CHANGELOG 与旧 API 替换说明，记录性能和体积差异。
