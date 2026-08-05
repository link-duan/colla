# JavaScript 使用结构化 CollaError

JavaScript facade 将所有核心失败表示为单一 `CollaError`，并稳定承诺 `code`、
`operation`、`path` 和结构化 `details`；同步构造与运算抛出该错误。错误
`message` 仅用于人类阅读，Rust enum 名称和 thiserror 文案不构成
公共契约。API 不为每个 Rust 变体创建 class，也不把普通调用改成 Result 对象。

`code` 使用原因导向的 lower_snake_case 稳定值，不包含操作前缀；
`operation` 独立说明失败的公共 API，使同一原因可以跨 Apply、Compose、
Decode 等操作统一处理。JavaScript 不按 Rust 错误 enum 一对一导出 code；
例如 codec 的具体解析失败统一为 `invalid_encoding`，精确原因和字节
偏移放在 `details`。

v1 稳定的顶层 code 清单仅为 `invalid_argument`、`invalid_value`、
`invalid_utf16_boundary`、`type_mismatch`、`missing_key`、
`key_already_exists`、`out_of_bounds`、`integer_overflow`、
`limit_exceeded`、`incompatible_change`、`invalid_encoding`、
`invalid_state`。List/Text/sequence 越界统一为
`out_of_bounds`；已释放句柄、已消费 Builder 和已退出 scoped Builder 统一为
`invalid_state`；精确情形通过稳定的结构化 `details` 区分。

`CollaError<C extends CollaErrorCode = CollaErrorCode>` 以 code 为泛型参数，
`details` 的类型由 `CollaErrorDetails[C]` 映射，并提供
`error.is(code)` TypeScript type guard 收窄 code 与 details。details 递归冻结，
每个 code 已公开的必需字段保持兼容；后续只能增加可选字段，不能改变
已有字段语义。

`operation` 使用扁平 lower_snake_case 的领域操作名，例如
`transform_pair`、`map_set` 和 `rich_text_format`，不使用
`Context.transformPair` 或 `MapChangeBuilder.set` 等绑定 JavaScript 接收者的
名称。已发布的 operation 值保持稳定，新增公共 API 可以追加新值。
