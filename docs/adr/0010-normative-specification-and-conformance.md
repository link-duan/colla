# 书面规范与 Conformance corpus 共同定义 1.0 契约

到 1.0，`docs/data-model.md`、`docs/ot-properties.md` 和 `docs/binary-format.md` 是人类
可读的规范性定义，版本化 Conformance corpus 与 tests 是机器可执行规范。Rust 是
reference implementation，JavaScript 必须通过相同 corpus，但任一实现的偶然行为都
不能自行覆盖书面规范；三者发生分歧时必须作为规范缺陷显式裁决并同步修正。
