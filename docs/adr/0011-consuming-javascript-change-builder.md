# ChangeBuilder build 转移所有权

JavaScript `ChangeBuilder.build()` 成功时消费 Builder、主动释放其 Wasm 资源并返回
新的 Change；被消费后的 Builder 拒绝任何后续运算。编辑失败时 Builder 保持调用
前状态，仍可继续使用或释放。Apply、Compose、Transform 和 Invert 不消费输入
句柄，只返回由调用方独立释放的新句柄；所有显式 `dispose()` 均为幂等操作。
