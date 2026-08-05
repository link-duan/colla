# TextChangeBuilder 使用 insert、delete 与 replace

TextChangeBuilder 提供 `insert(position, text)`、半开范围 `delete(range)` 和
`replace(range, text)`；Rust 坐标为 Unicode scalar，JavaScript facade 坐标为严格
UTF-16 边界，空内容或空范围折叠为 Noop。replace 在 scoped callback 中事务性地
组合 Delete 与 Insert，不引入新的原子 OT 语义。v1 不增加 append、prepend 或
clear 等可由这三个操作直接表达的别名。
