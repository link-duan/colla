# 二进制 codec 属于 Value 与 Change

JavaScript 通过 `value.encode()` 和 `change.encode()` 生成规范二进制，
通过 `Value.decode(bytes, options?)` 和 `Change.decode(bytes, options?)` 在默认或本次调用
Limits 约束下解码。`encode()` 每次返回新的、由 JavaScript 拥有且不
引用 Wasm memory 的 `Uint8Array`，因此句柄释放后仍然有效；解码仅在
调用期间读取输入字节，不保留其引用，后续修改或转移输入 buffer 不
影响已解码句柄。
