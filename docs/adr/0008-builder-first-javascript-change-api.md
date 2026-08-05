# JavaScript Change API 采用 Builder-first

JavaScript v1 使用基于 Snapshot 的 ChangeBuilder 作为主要 Change 构造入口；
Change 也可以由 decode、Compose、Transform 和 Invert 产生。v1 不公开
MapChange、ListOp、RichTextOp 等底层构造器，以免公共 API 允许制造非法或非规范
Change。Change 可以提供只读诊断视图，但该视图不是构造或传输格式；网络和持久化
继续使用规范二进制编码。
