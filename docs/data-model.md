# Colla 核心数据模型

状态：规范性设计。本文中的“必须/禁止/应当”具有约束力。

## 1. 范围

Colla 的核心只包含不可变 `Value`、不可变递归 `Change`、`Path` 导航、
`ChangeBuilder`、OT 代数操作、输入资源策略和规范二进制 body codec。

核心不提供 `DocOp`、`Action`、`Document`、Session、Cursor/Selection、Diff、
JSON、历史格式兼容、原子 Move 或业务 Schema。

## 2. Value

`Value` 是不可变、可结构共享的句柄。相等性只比较结构内容，不比较 Arc
指针身份。类型集合封闭：

    Null
    Bool(bool)
    Int(i64)
    Float(FiniteF64)
    String(String)
    Text(Text)
    RichText(RichText)
    List(List)
    Map(Map<String, Value>)

`String` 是原子字符串，只能整体替换；`Text` 支持 code point 粒度 OT。
`Float` 必须有限，且 `-0.0` 规范化为 `0.0`。Map key 只能是字符串。
Value 根节点可以是任意类型。

Value 通过受控构造器保证局部合法。深度、节点数、文本和容器大小不是 Value
类型的固定语义上限。`InputLimits` 只约束外部 Value/Change 输入，至少覆盖 Value
节点数、Change 节点数、递归深度、容器长度、字符串字节数、序列 op 数和序列
逻辑长度。

Apply、Compose、Transform、Invert 和 Builder 不接收或读取 InputLimits；运算结果
可以超过接收方的默认输入策略。序列算法必须保持紧凑，不能依赖可配置阈值避免
展开超大逻辑 retain/delete。

## 3. RichText

RichText 是核心类型，因为字符、embed、格式属性和 span 规范化具有独立
序列代数。文本按 Unicode scalar value 计长，每个 embed 长度为 1。
embed 是原子值，只能插入、删除或整体替换；需要独立协同的数据应放在
文档其他位置，embed 保存稳定引用。

快照规范：

- 禁止空文本 span。
- 相邻且 Attrs 相同的文本 span 必须合并。
- embed 独占一个 span。
- Attrs key 严格递增且唯一。

Attrs 只接受原子 `AttrValue`：Bool、Int、Float、String。属性变化使用
`Set(AttrValue)` 或 `Remove`，禁止用 Null 充当删除哨兵。

## 4. Change

一个根 `Change` 表示一次完整操作，不再使用 `DocOp(Vec<Change>)`：

    Noop
    Replace(Value)
    Map(MapChange)
    List(ListChange)
    Text(TextChange)
    RichText(RichTextChange)
    Int(IntChange::Add(i64))

Null、Bool、String、Float 只支持 Replace。Replace 可以改变节点类型，并
支配同节点或后代的并发修改。Float 不支持 Add，因为 IEEE-754 加法不满足
结合律，会破坏 TP1。

Change 是相对于某个基准快照的前向操作，不携带旧值、版本、作者或 op id。
Invert 显式接收变更前快照。冲突排序由上层传入 `TieBreak::LeftFirst` 或
`RightFirst`。

`Noop` 是代数单位元。空容器 Change、Add(0) 和被 transform 消除的操作均
规范化为 Noop。子结构中禁止残留 Noop。

## 5. MapChange

MapChange 是按 key 排序的唯一 entry change 集合：

    Insert(Value)   // 基准中 key 不存在
    Delete          // 基准中 key 存在
    Modify(Change)  // 基准中 key 存在，递归修改 value

整体替换 entry 表示为 `Modify(Replace(value))`。同一 MapChange 中一个 key
最多出现一次。

并发规则：同 key Insert/Insert 由 TieBreak 选赢家；Delete 支配 Modify；
Modify/Modify 递归 transform；Delete/Delete 合并。Insert 与 Delete/Modify
不可能同时基于同一合法快照。

## 6. 序列 Change

TextChange：Retain、Insert(String)、Delete。

ListChange：Retain、Insert(Vec<Value>)、Delete、Modify(Change)。Modify
消费并修改游标处一个基准元素。

RichTextChange：

    Retain { len, attrs: AttrPatch }
    Insert { content: RichInsert, attrs: Attrs }
    Delete(len)

`Retain { attrs }` 表示保留覆盖范围内的内容，同时把 AttrPatch 应用于每个
字符或 embed；空 AttrPatch 表示纯 Retain。Format 可以作用于 embed，但
不能递归进入 embed Value。

所有序列 Change 都省略未触及的尾部，尾部隐式 Retain。Change 不保存
input_len/output_len。Insert 与 Delete 同位置时，规范顺序为 Insert 在前。

并发 Delete 只删除共同基准已有内容，不删除并发 Insert。Delete 支配对被删
List 元素的 Modify。并发 RichText 属性修改中，不同 key 合并；相同 key
由 TieBreak 决定；内容 Delete 支配 Format。

## 7. 规范形式

公共 Value/Change 始终规范，字段私有，只能通过构造器、Builder、代数操作
或严格 decoder 产生。主要规则：

- Retain/Delete 长度必须大于零，Insert 内容不能为空。
- 相邻同类 Retain/Delete/Insert 必须合并。
- 尾部纯 Retain 必须移除。
- Text/RichText 相邻且属性兼容的文本 Insert 必须合并。
- Map/Attrs key 严格递增且唯一。
- 空类型化 Change、Modify(Noop)、Add(0) 必须折叠为 Noop。
- decoder 拒绝而不是修复非规范编码。

## 8. API 轮廓

    change.apply_to(&base, &limits)
    first.compose(&second, &limits)
    transform_pair(&left, &right, tie_break, &limits)
    change.invert(&base, &limits)

Apply 返回新的不可变 Value；失败时基准值不变。公共代数 trait 不开放，
因为 Value/Change 是封闭类型。

`Path` 仅是相对于某个 Value 快照的临时导航地址，用于查询、错误定位和
`ChangeBuilder`，不进入 Change 或 wire format。

`ChangeBuilder` 持有基准和临时工作快照。每次编辑按调用顺序构造小 Change、
应用到工作快照并 compose 到累计 Change。单次调用失败时 Builder 状态不变。
