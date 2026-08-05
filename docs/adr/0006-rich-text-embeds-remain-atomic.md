# RichText embed 保持原子

JavaScript v1 沿用 Rust 核心语义，不增加 `ModifyEmbed`：embed payload 可以是
任意 Core Value，但在 RichText 内只能整体插入、删除或替换。需要独立协同修改的
数据应存放在文档的 Map/List 中，embed 仅保存稳定引用；展示属性可以使用 embed
的 RichText attrs。该边界避免扩展 Compose、Transform、Invert 和二进制格式，
也避免用 delete+insert 模拟并发替换时产生多个存活 embed。
