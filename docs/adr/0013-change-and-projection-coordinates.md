---
status: accepted
---

# Change 构造坐标与 Snapshot 投影坐标分离

Rust Change 与 JavaScript `ChangeInput`/Builder 的 Text、RichText 长度统一使用 Unicode
scalar，RichText Embed 占 1。JavaScript Change View、Edit Steps 和显式 Snapshot 坐标
转换使用 UTF-16 code unit，以适配编辑器和 JavaScript 字符串；Edit Steps 的 retain/delete
长度按实际消费的 Snapshot 片段投影，insert 内容不转换。落在 surrogate pair 内的位置必须
拒绝而不能取整。普通 String 保持原子；RichText Embed 也是原子 Core Value，只能整体
插入或删除，不能在 RichText 内递归修改。
