# Change 结构化检查返回 ChangeView

JavaScript 通过包级 `inspectChange(change, base, options?)` 返回递归冻结的
`ChangeView`。该视图结合 Change 与对应 Snapshot，派生可读 Path，并将
Text/RichText 核心坐标转换为经过 surrogate 边界验证的 JavaScript UTF-16
坐标。`ChangeView` 是扁平、有序、递归冻结的
`readonly ChangeViewEntry[]`，以 Snapshot 上的确定性遍历顺序列出叶子操作，
不暴露 Rust 的嵌套 Change 树。视图以面向用户的语义操作表达可观察
效果：省略 `retain`，展开 `modify`，并将结合 Snapshot 可识别的操作表达为
`map.set`、`list.set`、`text.insert` 等公共词汇，而非复制核心存储结构。
它是稳定的只读检查投影，不是规范
二进制、不能用于构造 Change，也不使 Path 成为 Change 本身的一部分。

`ChangeViewEntry` 是以 `type` 判别的稳定联合，操作集合为
`value.replace`、`int.add`、`map.set`、`map.delete`、`list.insert`、
`list.set`、`list.delete`、`text.insert`、`text.delete`、
`richText.insertText`、`richText.insertEmbed`、`richText.delete` 和
`richText.format`。Noop 返回空数组；Text replace 展开为 delete 与 insert，
RichText 替换也保持显式组合。Map 的 `key` 和 List 的 `index` 与指向
容器的 `path` 分开，所有范围是 `{ from, to }` 半开区间；`value`、
`values` 和 `embed` 使用递归冻结的 `ValueData`。

RichText insert entry 的可选 `attrs` 使用 `AttrsData`。format entry 的
`patch` 使用冻结的 null-prototype `AttrPatchView` record，每个值是
`{ type: "set", value: AttrValueData }` 或 `{ type: "remove" }`，不使用
`null` 或 `undefined` 充当删除哨兵。`AttrValueData` 仅允许
`boolean | bigint | number | string`，record key 按规范字典序输出；空
attrs 可省略，空 patch 为 Noop 且不产生 entry。
