# ListChangeBuilder 使用 insert、set 与 delete

ListChangeBuilder 仅提供 `insert(index, values)`、`set(index, value)` 和半开范围
`delete(range)`。set 生成单元素 Modify(Replace)，不会退化成删除后插入；所有索引
按根 Builder 当前临时 Snapshot 解释，越界报错，空插入或空范围为 Noop。v1 不
提供原子 Move，也不增加与 set 重叠的单元素 replace API。
