# JavaScript 文本坐标使用 UTF-16

Rust core 与 wire 使用 Unicode scalar position，JavaScript Text/RichText API 使用
UTF-16 code unit，并在边界显式转换；落在 surrogate pair 内的位置被拒绝，范围统一为
`[from, to)`。普通 String 保持原子，Text 必须显式创建。

RichText 由规范 Text/Embed spans 与属性组成；Embed 是长度为 1 的原子 Core Value，
只能整体插入、删除，不能在 RichText 内递归修改。RichText 不提供通用 insert 或
replace；替换通过 delete 与 typed insert 显式组合。
