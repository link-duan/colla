# 跨语言 golden tests 是发布门禁

Rust native 与从实际 npm tarball 安装的 `@colla/core` 必须在每次发布前
通过双向 golden tests。门禁覆盖 Value/Change 规范二进制的逐字节一致性、
双向 encode/decode、Apply/Compose/Transform/Invert 结果、UTF-16 与 Unicode
scalar 边界转换，以及对非规范编码和 Limits 超限输入的一致拒绝。
