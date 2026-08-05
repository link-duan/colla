# JavaScript Value 边界严格且规范

`Value.fromJS()` 将有限 number 映射为 Float、范围内 bigint 映射为 Int、普通 string
映射为原子 String，协同 Text 和 RichText 使用显式 marker。Map 只接受普通或
null-prototype record，并拒绝 accessor、symbol key、class instance、循环引用和其他
不确定输入；输出 ValueData 递归冻结，Map 使用 null-prototype record。

Value/Change codec 使用唯一规范字节。encode 总是返回独立所有权的 bytes；decode
只在调用期间读取输入，并以 `InputLimits` 约束不可信输入的深度、节点、容器、字符串、
序列操作和逻辑长度。
