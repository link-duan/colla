# JavaScript 显式构造可协同 Text

普通 JavaScript `string` 映射为只能整体替换的 Colla String；需要字符级 OT 的值
必须用 `text(value)` 显式构造。反向转换必须保留 Text 的类型信息，不能静默降级
为普通 `string`。这保持了 primitive string 的直觉和无损往返，也避免普通赋值在
不知情时改变字段的协同语义。
