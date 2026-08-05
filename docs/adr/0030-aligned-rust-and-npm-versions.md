# Rust crate 与 @colla/core 使用相同版本

Rust crate 与 npm package `@colla/core` 始终使用相同 SemVer，并作为一个
发布单元原子发布。npm 包是该 Rust core 的 Wasm facade，统一版本可以让
核心语义、规范二进制、跨语言 golden tests 和问题定位对应到同一个
可识别的实现版本。
