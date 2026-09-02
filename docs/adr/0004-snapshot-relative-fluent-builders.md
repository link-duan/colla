---
status: superseded by ADR-0012
---

# ChangeBuilder 相对 Snapshot 链式构造

Builder 从 `base.change()` 创建并持有 Snapshot 的廉价 clone，使用可链式的 scoped
Map、List、Text、RichText 和 Int builder。callback 同步且事务化，失败回滚当前
callback；成功 build 消费根 Builder，dispose 放弃构造。Map set 是 snapshot-aware
upsert，序列范围采用半开区间，空操作规范化为 Noop。

Builder 输入仍执行 Core 合法性验证，但不接收或隐式应用 `InputLimits`；直接传入的
Value 规模由消费方负责。scoped builder 不能逃逸 callback，Builder 不提供 clone。
