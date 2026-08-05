# Core Map 使用严格安全的 JavaScript record

JavaScript Array 映射为 List；只有 prototype 为 `Object.prototype` 或 `null`、且仅
包含 own enumerable string-keyed data properties 的普通对象可映射为 Map。facade
拒绝 accessor、symbol key、class instance、Date、Set、JavaScript Map 和循环引用。
反向转换使用递归冻结的 null-prototype record 与冻结数组，使 `__proto__` 等任意
合法字符串 key 不触发原型语义，并保持 Value 的只读边界。
