# JavaScript v1 运行时边界

Colla 的 JavaScript v1 以单一 npm package `@colla/core` 提供 TypeScript facade 和内部 Wasm
实现，支持现代浏览器 bundler 与 Node.js 20+ ESM。v1 不承诺 CommonJS、Deno、
Bun 或 edge worker 支持，以避免运行时特定加载方式扩大公共兼容性边界。
