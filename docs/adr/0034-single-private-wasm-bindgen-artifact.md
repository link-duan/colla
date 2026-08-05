# Browser 与 Node 共用单一私有 wasm-bindgen 产物

`colla-wasm` 只构建一份最终 Wasm binary，再由 wasm-bindgen 生成私有 JS
glue。Browser 入口将该 Wasm 字节转为 base64 并同步初始化，Node.js
入口读取同一份 `.wasm` 并同步初始化，避免按 wasm-pack target 分别
构建导致行为或字节漂移。wasm-bindgen 的初始化函数、生成 class、文件名与
目录都是可替换的内部 ABI，不作为 package exports 或 TypeScript 契约。
