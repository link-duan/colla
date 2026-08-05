# JavaScript v1 保持 OT 核心范围

npm v1 仅包装 Value、ChangeBuilder、Apply、Compose、Transform、Invert 和规范
codec，不提供 Document、Session、client/operation identity、版本控制、网络同步或离线
队列。`transformPair` 要求调用方使用明确的 `left-first`/`right-first` 顺序，并由
上层基于稳定 operation identity 在所有参与方作出一致选择。控制算法与协作
session 若需要，将作为独立上层 package，而不扩张 Wasm core wrapper。

JavaScript `transformPair(left, right, options)` 返回与输入顺序一一对应的
`readonly [Change, Change]`。两个输出是独立、需主动释放的句柄，不消费
输入；失败时不返回部分结果。
