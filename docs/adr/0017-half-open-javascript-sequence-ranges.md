# JavaScript 序列范围统一为半开区间

JavaScript Text、RichText 和 List API 统一使用 `{ from, to }` 表示 `[from, to)`；
Text/RichText 坐标为 UTF-16，List 坐标为元素索引。Insert 只接收单个位置，不公开
含义容易混淆的 `(index, length)` 调用形式。空范围产生 Noop，反向范围或越界范围
抛出结构化错误；facade 在进入 Rust 核心前转换为现有 index/length 表示。
