# Wasm 句柄以主动释放为正常生命周期

所有拥有 Wasm 资源的公开句柄，包括 Value、Change 和根 ChangeBuilder，
提供幂等 `dispose()`，并在可用时实现
`Symbol.dispose`；调用方应在正常控制流中主动释放。句柄释放后的任何运算都必须
失败，而不能访问已回收内存。`FinalizationRegistry` 的执行时机不确定，因此只作
为遗漏释放时的安全网，不承担正确性或常规内存管理职责。

Value 与 Change 提供廉价 `clone()`，通过 Rust `Arc` clone 返回独立、需单独
释放的新句柄，不复制完整数据树或操作树。clone 与原句柄可以按任意顺序
释放，已释放句柄不能 clone。ChangeBuilder 保持线性所有权，不提供 clone。
