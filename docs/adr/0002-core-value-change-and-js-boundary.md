# Core Value、Change 与 JavaScript 边界

状态：Accepted

Core Value 使用封闭、严格的递归模型；JavaScript 输入必须通过明确的 Value marker、
typed Change Input 或纯 TypeScript Builder 进入 Core。普通 JavaScript string 是原子
String，协同文本和 RichText 必须显式表达。输入形状、限制和错误分类属于公共边界，
规范化、语义校验和 OT 行为由 Rust Core 负责。

Change 构造不依赖 Snapshot；Snapshot 只在 apply、投影或坐标转换等需要内容上下文时
参与。Core 与 Change 构造使用 Unicode scalar 坐标，JavaScript 面向编辑器的投影使用
UTF-16 坐标。Value、Change、RichText 和坐标的完整结构定义分别以项目规范文档为准。
