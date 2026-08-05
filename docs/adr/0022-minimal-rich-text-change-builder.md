# RichTextChangeBuilder 使用显式最小操作集

RichTextChangeBuilder 仅提供 insertText、insertEmbed、半开范围 delete 和通过
AttrPatchBuilder 表达的 format。它不提供通用 insert 或 replace；范围替换必须在
同一事务 callback 中显式组合 delete 与对应 insert，使 API 不暗示 Rust 核心不存在
的原子 Replace 语义。Rust 坐标为 Unicode scalar，JavaScript facade 在严格 UTF-16
边界与核心坐标之间转换。
