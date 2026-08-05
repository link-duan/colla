# JavaScript RichText 使用规范 spans

JavaScript ValueData 以带 `type` 判别字段的规范 Text/Embed spans 表示 RichText，
其 `AttrsData` 是递归冻结的 null-prototype record，值仅允许
`boolean | bigint | number | string`，并按 key 字典序输出。空 attrs 可以省略
并视为空集合；相邻且 attrs 相同的 Text spans 由核心合并。
RichText Change 继续使用独立的 retain/insert/delete 模型。Quill Delta、
ProseMirror Node、HTML 等生态格式不进入核心语义，由独立 adapter 转换，避免形成
第二套 Value 或 Change 规范。
