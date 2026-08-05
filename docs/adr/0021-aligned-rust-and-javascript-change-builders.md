# Rust 与 JavaScript 对齐类型化 ChangeBuilder

Rust 在现有 ChangeBuilder 上增加 MapChangeBuilder、ListChangeBuilder、
TextChangeBuilder、RichTextChangeBuilder、IntChangeBuilder 和 AttrPatchBuilder；
这些类型在同步 callback 中暂时借用根 Builder，并以事务方式提交。JavaScript
facade 使用相同概念和类型命名，只负责 camelCase、UTF-16 坐标与 JS 值转换。
该扩展复用现有构造、Apply 和 Compose 逻辑，不改变 Value、Change、OT 规则或
二进制格式；现有扁平 Builder 方法可以先作为底层 API 保留。
