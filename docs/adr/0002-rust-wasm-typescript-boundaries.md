# Rust、Wasm 与 TypeScript 保持单向边界

可发布的 `colla` Rust crate 是唯一 OT 与规范 codec 实现；私有 `colla-wasm` crate
单向依赖 core，手写 TypeScript facade 再包装私有 ABI。Rust core 不依赖 wasm-bindgen，
公共 npm API 不暴露生成 class、初始化函数、指针或生成目录，因此 ABI 可以独立演进。
