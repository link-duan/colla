# JavaScript 与 Rust 统一使用 Value

JavaScript 拥有 Wasm 资源的只读句柄命名为 Value，与 Rust 核心类型一致；严格 JS
输入和冻结输出分别命名为 ValueInput 与 ValueData。`Value.fromJS(input, options?)` 创建
Value，`Value.toJS()` 和 `Value.get(path)` 返回 ValueData。Snapshot 继续表示某个
Value 在一次 Change 运算中扮演的基准状态角色，不作为独立公共类型。
