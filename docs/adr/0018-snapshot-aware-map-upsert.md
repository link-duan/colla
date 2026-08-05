# MapChangeBuilder 使用 snapshot-aware upsert

MapChangeBuilder 的 `set(key, value)` 根据根 Builder 当前临时 Snapshot 生成
Insert 或 Modify(Replace)，相同值赋值折叠为 Noop；`delete(key)` 删除已存在 key，
缺失 key 时产生 Noop。主 API 不区分严格 insert/replace，调用方若需要前置条件可先
查询 Snapshot。每一步都观察当前临时 Snapshot，使同一 fluent scope 内的连续操作
具有直观顺序语义。
