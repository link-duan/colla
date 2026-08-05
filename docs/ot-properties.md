# Colla OT 性质

## 1. 记号

`apply(v, a)` 表示把 Change a 应用于 Value v。所有等式都以参与操作在对应
上下文中可成功应用为前提；类型错误、key/index 不存在或整数溢出不属于等式定义域。

## 2. Apply、Compose 与 Invert

Apply 原子性：失败不改变输入快照。

Compose：

    apply(apply(v, a), b) == apply(v, compose(a, b))

Invert：

    apply(apply(v, a), invert(a, v)) == v

Change 不携带旧值，因此 invert 必须接收 v。

## 3. TP1

对基于同一快照 v 的并发操作 a、b：

    (a_prime, b_prime) = transform_pair(a, b, tie_break)

必须满足：

    apply(apply(v, a), b_prime) == apply(apply(v, b), a_prime)

TP1 保证一对并发操作在两种执行顺序下收敛。TieBreak 必须由上层以一致、
确定的方式提供。

## 4. TP2

TP2 讨论三个并发操作经不同 transform 路径处理后，最终变换结果是否一致。
一种常见表述是：对共同基准上的 a、b、c，经 a 路径和 b 路径变换 c，得到
的 c 变体应等价。

Colla 不保证 TP2。位置型 Text/List OT 在缺少稳定 operation identity、上下文
向量或特定控制算法约束时，通常无法对所有三方交错保持路径无关。Colla 已
明确不在 Change 中携带 client id、op id、version 或 timestamp，因此不能把
某个分布式控制协议的假设伪装成数据模型性质。

使用方必须选择只依赖 TP1 且具有固定 transform 顺序的中心化控制算法，或在
上层引入足够的身份和上下文信息。Session 状态机不属于本次核心库范围。

## 5. 冲突摘要

- Replace 支配同节点及其后代修改；Replace/Replace 由 TieBreak 选赢家。
- Map Delete 支配 Modify；Insert/Insert 由 TieBreak 选赢家。
- List Delete 支配被删元素 Modify；并发 Insert 存活。
- Text/List/RichText 同位置 Insert 的顺序由 TieBreak 决定。
- RichText 不同属性 key 合并；同 key 冲突由 TieBreak 决定。
- Int Add 使用 checked arithmetic。共同结果越界时，后应用操作失败并由上层
  拒绝，该操作组不在 TP1 的共同可应用定义域内。

## 6. 验证要求

实现必须用 property-based tests 覆盖 Compose、Invert、TP1、codec roundtrip
和规范形式。随机字节 decoder 测试必须在 InputLimits 下保证不 panic、不爆栈且
不进行不受限分配。TP2 不作为通过条件，但文档必须保留本节限制说明。
