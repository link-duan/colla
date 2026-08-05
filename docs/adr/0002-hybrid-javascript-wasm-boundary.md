# JavaScript 使用混合 Wasm 边界

Colla 的稳定 JavaScript API 采用 TypeScript facade，隐藏 wasm-bindgen 生成 ABI。
Value 和 Change 以 Wasm 句柄参与重复代数运算，ChangeBuilder 以可变句柄构造
Change；普通 JS 数据仅通过显式转换进入或离开 Value，规范二进制通过
`Uint8Array` 传输。这样既保留 Rust 的结构共享并避免每次运算复制整棵值树，又
保持 JS 输入输出与网络、存储和 worker 边界的可用性；代数操作可以使用顶层函数，
不要求所有能力都表现为类方法。
