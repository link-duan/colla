# RichText 规范 spans 与索引存储分离

RichText 的规范模型和 wire 继续表示为 Text/Embed spans，但 Rust 公共 API 不再暴露
底层 span slice；内存表示缓存 Unicode scalar/UTF-16 长度及两套累计 span 结束位置，
Apply/Invert 通过 span cursor 工作而不展开逐字符 atom。scalar 与 UTF-16 索引都是可从
UTF-8 span 内容重建的派生缓存，不参与相等性、哈希、Canonical form 或 wire encoding。

Rust OT 操作和 JavaScript Change 构造都使用 Unicode scalar 坐标。UTF-16 索引仅服务于
结合 Snapshot 的 JavaScript 坐标投影：先用累计位置二分定位 span，再只扫描目标 Text
span；Embed 在两套坐标中都占 1。这样保持既有规范字节和 OT 语义，同时避免把当前
`Vec` 或未来可能采用的 rope/tree 固化成跨语言数据契约。

Rust 统一通过 fallible `RichText::from_spans` 构造值：入口删除空 Text、合并属性
兼容的相邻 Text spans，再以 checked arithmetic 构建累计索引。内部不提供跳过
规范化的构造入口，避免调用者把内容与派生长度或索引组合成不一致状态。

Snapshot decoder 保留 `max_sequence_len` 的流式资源限制，但不复制 RichText 的
规范化规则。为兼容历史数据和外部实现，它把解码出的 spans 交给 `from_spans`，因此
接受空 Text 和未合并的相邻 Text；内存值及 encoder 输出仍保持规范。由此允许
`encode(decode(bytes))` 对这类宽容输入产生不同但规范的 bytes，既有规范 fixtures
本身不变。RichText Change decoder 继续拒绝空 Insert 和非规范操作序列。
