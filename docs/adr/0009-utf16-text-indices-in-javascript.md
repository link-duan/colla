# JavaScript 文本索引使用 UTF-16

JavaScript facade 的 Text 和 RichText API 使用 UTF-16 code unit 索引，以匹配
JavaScript string、DOM 和主流编辑器坐标；embed 始终计为 1。facade 根据当前
Snapshot 转换为 Rust 核心使用的 Unicode scalar index，并严格拒绝落在 surrogate
pair 中间的位置，不做隐式取整。显式坐标转换通过包级
`resolveCodePointPosition(value, path, utf16Position)` 与
`resolveUtf16Position(value, path, codePointPosition)` 提供，而不挂在 Context 或
通用 Value 方法上。对 Text，CodePoint 坐标对应核心 Unicode scalar 坐标；
对 RichText，文本按 code point 计数，Embed 在两套坐标中均计为 1。Rust API、
Change 和二进制格式保持现有 Unicode scalar 语义。
