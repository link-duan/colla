# Golden fixtures 使用语言中立的带标签表示与统一错误 code

Status: accepted

golden fixtures 需要一份既能被 Rust reference implementation 又能被 JavaScript facade
消费的数据源。本 ADR 决定：fixtures 用带类型标签的中性 JSON 表示 Value 与 Change
（`int` 用十进制字符串保留完整 `i64` 精度、`map` 与 attrs 用 JSON 对象、规范字节用
十六进制），并断言一套**两侧统一的稳定错误 code**（只断言到 `code` 层）而非任一实现的
内部错误枚举；fixtures 同一集内只增量新增（不做目录版本化）。完整格式、
fixture 类型、断言与演进见 [Golden fixtures 设计](../internal/golden-tests.md)。

## Considered Options

- **Map / attrs 表示**：选择 JSON 对象而非有序 `[key, value]` 数组。「key 严格递增」属于
  编码层的规范形式，由 `canonicalBytes` 断言负责，构造器会自行对键排序；非规范输入
  （乱序、重复 key）一律用 `decode-error` 的字节输入表达，不需要中性 JSON 承载键顺序，
  因此 JSON 对象更可读且无损。fixture 校验拒绝重复键。
- **错误 code**：选择一套两侧统一的稳定 code 作为断言依据，而非任一实现的内部错误枚举。
  设计文档 §5 的映射表是该统一 code 集的规格镜像，JavaScript facade 与 Rust 侧均按相同
  规则折叠到同一 code。只断言 `code`，`reason` 等子字段不作断言。备选的"另建一套中性
  error taxonomy"会引入第三套需维护的名字并与公开契约脱节，被否决。统一 code 的单一
  事实来源由 [ADR 0015](0015-error-code-classification.md) 定为核心 `ErrorCode` 并纳入
  Rust 1.0 契约。

## Consequences

中性表示与错误 code 一旦被写成 fixture 并由两侧共享，即难以逆转；变更需两侧同一提交
同步修改，影响规范字节或语义时记入 `CHANGELOG`。fixtures 当前不做目录版本化；若未来
wire 破坏性修订需新旧向量并存，再引入版本层。
