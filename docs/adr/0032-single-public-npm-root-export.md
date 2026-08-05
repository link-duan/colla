# @colla/core v1 只公开根入口

`@colla/core` v1 只公开 package 根入口，不提供 `browser`、`node`、`wasm`、
`internal` 或其他公共 subpath exports。package 可在根入口的条件导出内部
选择 browser/base64 与 Node.js/Wasm 实现，但消费者只依赖统一的
TypeScript facade，从而允许后续替换 Wasm 工具链、包装方式和内部目录。
