# Value 使用显式路径读取

JavaScript Value 是只读句柄，通过 `get(path)` 复制指定子树，通过 `toJS()` 显式
复制整棵值树，并提供 `kind(path)`、`has(path)` 等不复制子树的轻量查询。v1 不
使用 Proxy 属性访问，避免普通属性读取触发隐式 Wasm 往返；`get` 也不返回新的
Wasm 子句柄，避免为每个字段增加主动释放负担。所有修改必须通过 ChangeBuilder。
