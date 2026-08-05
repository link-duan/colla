# JavaScript 数字映射保持类型与精度

JavaScript `number` 映射为 Colla Float，`bigint` 映射为 Colla Int 且必须处于
`i64` 范围；`toJS()` 也分别返回 `number` 和 `bigint`。facade 另提供 `int(42)`
便利构造器，但不根据 `number` 是否恰为整数来猜测类型。该规则避免 `1` 在
Int 与 Float 之间产生不稳定往返，也不会受 JavaScript 安全整数范围限制；NaN
和 Infinity 必须拒绝。
