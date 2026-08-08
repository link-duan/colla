# 单一 Wasm 产物提供同步跨运行时入口

`colla-ot` v1 只公开 ESM package 根入口且导入后同步可用，不提供公开 init、TLA、
slim 或内部 subpath。Browser/default entry 将最终 Wasm 内嵌为 base64，Node entry 通过
`import.meta.url` 同步读取独立 `.wasm`；两者必须执行相同 hash 的单一 binary。

Browser 初始化不依赖 DOM、同步 XHR 或网络，支持 main thread、Dedicated Worker 和
Shared Worker。Vite 5+ 与 Rollup 4+ 不需要 Wasm、资源复制或 top-level-await 插件；
package 明确声明初始化 side effects。
