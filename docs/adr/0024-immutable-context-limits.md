# Rust Context 固定 Limits，JavaScript 按调用传入 overrides

Rust `Context::default()` 使用 `Limits::default()`，`Context::new(limits)` 接收完整
配置，创建后不可修改。JavaScript 包级运算、静态 decode/fromJS 和
`Value.change()` 使用默认 Limits，并可接收 `{ limits: partialOverrides }` 作为
本次调用的不可变 options；不存在可变全局配置或公开 JavaScript Context。
package 导出递归冻结的 `DEFAULT_LIMITS: Readonly<Limits>` 和共享基础类型
`OperationOptions { readonly limits?: Partial<Limits> }`；有额外参数的操作扩展
该类型。options 仅在本次同步调用期间读取，不保留引用。overrides 必须是
非负 safe integer，默认值由跨语言 golden tests 保证与 Rust 一致。
