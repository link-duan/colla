# 路径寻址核心：普通类型不携带身份

状态：Accepted · 取代 [0007](0007-stable-element-identity-and-native-move.md) 的身份、Move 与 Ref 部分

0007 为每个元素分配稳定 Element ID，并以 ID 支撑跨父节点 Move 与 Ref。实践表明代价
落在所有用户身上：每个 ID 编码 18～26 字节，常超过节点内容；apply 每步全树校验唯一性、
每次编辑重建索引，单步编辑代价随文档大小增长；Set 保留根 ID、Copy 重映射、相等性二分、
线程级随机分配器等语义需要所有使用者理解。transform 依赖"按 ID 合并状态再反推差分"，
无法在去掉身份后保留。

## 决策

Map、List、Text、RichText 与标量**不携带身份**，以 **Path** 作为唯一寻址方式；Change
由按路径寻址的操作组成，transform 为逐操作的路径 OT。中心化 Authority 只要求 TP1。

- 移动仅支持同一 List 内的 **ListMove**。需要跨父节点移动且让并发编辑跟随元素的场景，
  由后续按需引入的 MovableTree 类型提供；其节点身份只在所属树内有效，普通类型不为此
  付费。在此之前，跨父移动表达为 Delete 加 Insert，不保留并发编辑关联。
- 移除 Ref。需要稳定业务键的应用在内容中自行保存。
- 值相等即结构相等；没有独立的内容相等。
- 普通类型的并发总能得到确定结果，不产生 Structural conflict：
  - Delete 或 Set 某路径时，另一侧在其内部的操作被丢弃；同路径 Delete 胜过 Set；
    Set 对 Set 按 Priority。
  - 同一 Map 键的并发 Insert 由高优先级者生效（变换为 Set），另一侧为 Noop。
  - 同一 List 间隙的并发 Insert 按 Priority 排序；ListMove 与 List 内其他操作按下标
    映射变换，同一元素的并发移动按 Priority。
  - Text/RichText 使用序列 OT；Add 可交换，与 Set 并发时 Set 胜。

## 后果

编码不再包含身份，apply 单步代价只与路径深度和被修改容器大小有关，运行时不再需要
安全随机源。代价是失去跨父节点 Move 与 Ref，以及元素在远程编辑后的长期地址：编辑器
需在每次远程编辑后通过变换维护自己持有的路径。

具体规则由[内容模型](../data-model.md)、[运行态规范](../document-model.md)、
[变更代数](../ot-properties.md)与[编码规范](../binary-format.md)定义。
