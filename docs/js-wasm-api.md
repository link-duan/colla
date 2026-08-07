# @colla/core JavaScript/Wasm v1 API

状态：已冻结的架构规范。具体 error details 的可选字段、生成脚本参数和
benchmark 阈值在实施阶段补充，不得改变本文的公共边界。

## 1. 范围与运行时

`@colla/core` 是 Rust `colla` OT core 的 TypeScript facade 和 Wasm wrapper。v1 只提供
Value、Change、ChangeBuilder、Apply、Compose、TransformPair、Invert、规范 codec
和只读检查，不提供 Document、Session、client ID、版本向量、网络队列或
编辑器格式。

支持下限：

- Node.js 22+ ESM。
- Vite 5+。
- Rollup 4+ 和常规 node_modules resolver，不要求 Wasm 插件。
- Browser main thread、Dedicated Worker 和 Shared Worker。

v1 不承诺 CommonJS、Deno、Bun、Service Worker 或 edge worker runtime。

package 只公开 `@colla/core` 根入口，使用 Value/Change 类型入口、从 Snapshot
创建的 Builder 和包级代数函数；所有 API 在 package 导入后同步可用。

## 2. 使用轮廓

```ts
import {
  Value,
  Change,
  text,
  apply,
  compose,
  invert,
  transformPair,
  inspectChange,
} from "@colla/core"

using base = Value.fromJS({
  title: text("Draft"),
  metadata: { status: "new" },
})

using update = base
  .change()
  .text(["title"], text =>
    text.insert(5, " v2")
  )
  .map(["metadata"], map =>
    map.set("status", "draft")
  )
  .build()

using next = apply(base, update)

const data = next.toJS()
const view = inspectChange(update, base)
const bytes = update.encode()
using decoded = Change.decode(bytes)
```

## 3. 基础类型

```ts
export type Path = readonly (string | number)[]

export interface Range {
  readonly from: number
  readonly to: number
}

export type ValueKind =
  | "null"
  | "bool"
  | "int"
  | "float"
  | "string"
  | "text"
  | "richText"
  | "list"
  | "map"
```

Path 中的 number 必须是非负 safe integer。所有范围使用 `[from, to)` 半开语义。

## 4. ValueInput 与 ValueData

映射规则：

- `null` → Null，`boolean` → Bool。
- JavaScript `string` → 原子 String；`text(value)` → 可协同 Text。
- `number` → Float，且必须有限。
- `bigint` → Int，且必须处于 i64 范围。`int(number)` 是便利 helper。
- Array → List。
- 只有纯 record → Map。拒绝 accessor、symbol key、class instance、Date、Set、
  JavaScript Map 和循环引用。
- RichText 使用规范 Text/Embed spans；Embed 是长度为 1 的原子 Value。

```ts
export type AttrValueData =
  | boolean
  | bigint
  | number
  | string

export type AttrsData = Readonly<
  Record<string, AttrValueData>
>

export interface TextData {
  readonly type: "text"
  readonly value: string
}

export type TextInput = TextData

export type RichTextSpanData =
  | {
      readonly type: "text"
      readonly text: string
      readonly attrs?: AttrsData
    }
  | {
      readonly type: "embed"
      readonly value: ValueData
      readonly attrs?: AttrsData
    }

export interface RichTextData {
  readonly type: "richText"
  readonly spans: readonly RichTextSpanData[]
}

export type RichTextSpanInput =
  | {
      readonly type: "text"
      readonly text: string
      readonly attrs?: Readonly<
        Record<string, AttrValueData>
      >
    }
  | {
      readonly type: "embed"
      readonly value: ValueInput
      readonly attrs?: Readonly<
        Record<string, AttrValueData>
      >
    }

export interface RichTextInput {
  readonly type: "richText"
  readonly spans: readonly RichTextSpanInput[]
}

export type ValueInput =
  | null
  | boolean
  | bigint
  | number
  | string
  | TextInput
  | RichTextInput
  | readonly ValueInput[]
  | Readonly<Record<string, ValueInput>>

export type ValueData =
  | null
  | boolean
  | bigint
  | number
  | string
  | TextData
  | RichTextData
  | readonly ValueData[]
  | Readonly<Record<string, ValueData>>

export function text(value: string): TextInput

export function richText(
  spans: readonly RichTextSpanInput[],
): RichTextInput

export function int(value: number | bigint): bigint
```

`ValueData` 的 Array 与 record 递归冻结，Map 输出使用 null-prototype record。
helper 返回冻结的显式输入 marker。`int(number)` 要求参数是 safe integer，
并与 bigint 输入一样校验 i64 范围。facade 在运行时执行上述严格边界验证。

## 5. InputLimits

```ts
export interface InputLimits {
  readonly maxDepth: number
  readonly maxValueNodes: number
  readonly maxChangeNodes: number
  readonly maxContainerLength: number
  readonly maxStringBytes: number
  readonly maxSequenceOps: number
  readonly maxSequenceLength: number
}

export const DEFAULT_INPUT_LIMITS: Readonly<InputLimits>

export interface InputOptions {
  readonly limits?: Partial<InputLimits>
}
```

默认值固定为：`maxDepth = 128`、`maxValueNodes = 1_000_000`、
`maxChangeNodes = 1_000_000`、`maxContainerLength = 1_000_000`、
`maxStringBytes = 16 * 1024 * 1024`、`maxSequenceOps = 1_000_000`、
`maxSequenceLength = 1_000_000`。

`InputLimits` 只约束 `Value.fromJS()`、`Value.decode()` 和 `Change.decode()` 接收的
外部数据，不定义 Core Value/Change 的合法大小，也不限制 Builder 或代数结果。
partial overrides 仅在本次同步调用期间读取，不保留引用。所有字段必须是非负
safe integer；没有可变全局默认值。

## 6. Value

```ts
export class Value {
  private constructor()

  static fromJS(
    input: ValueInput,
    options?: InputOptions,
  ): Value

  static decode(
    bytes: Uint8Array,
    options?: InputOptions,
  ): Value

  change(): ChangeBuilder

  kind(path?: Path): ValueKind
  has(path: Path): boolean
  get(path: Path): ValueData
  toJS(): ValueData

  encode(): Uint8Array
  clone(): Value
  dispose(): void
  [Symbol.dispose](): void
}
```

`get()` 复制指定子树，不返回 Wasm 子句柄。`change()` 在 Rust 中廉价 clone
Snapshot `Arc`，因此 Builder 创建后可以立即释放原 Value。

## 7. Change 与 codec

```ts
export class Change {
  private constructor()

  static decode(
    bytes: Uint8Array,
    options?: InputOptions,
  ): Change

  encode(): Uint8Array
  clone(): Change
  dispose(): void
  [Symbol.dispose](): void
}
```

`encode()` 每次返回新的 JS-owned `Uint8Array`，不暴露 Wasm memory view。
`decode()` 不保留输入 buffer 引用。

## 8. ChangeBuilder

```ts
export class ChangeBuilder {
  private constructor()

  replace(path: Path, value: ValueInput): this

  map(
    path: Path,
    edit: (map: MapChangeBuilder) => unknown,
  ): this

  list(
    path: Path,
    edit: (list: ListChangeBuilder) => unknown,
  ): this

  text(
    path: Path,
    edit: (text: TextChangeBuilder) => unknown,
  ): this

  richText(
    path: Path,
    edit: (richText: RichTextChangeBuilder) => unknown,
  ): this

  int(
    path: Path,
    edit: (int: IntChangeBuilder) => unknown,
  ): this

  build(): Change
  dispose(): void
  [Symbol.dispose](): void
}

export interface MapChangeBuilder {
  set(key: string, value: ValueInput): this
  delete(key: string): this
}

export interface ListChangeBuilder {
  insert(index: number, values: readonly ValueInput[]): this
  set(index: number, value: ValueInput): this
  delete(range: Range): this
}

export interface TextChangeBuilder {
  insert(position: number, text: string): this
  delete(range: Range): this
  replace(range: Range, text: string): this
}

export interface RichTextChangeBuilder {
  insertText(
    position: number,
    text: string,
    attrs?: AttrsData,
  ): this

  insertEmbed(
    position: number,
    embed: ValueInput,
    attrs?: AttrsData,
  ): this

  delete(range: Range): this

  format(
    range: Range,
    edit: (patch: AttrPatchBuilder) => unknown,
  ): this
}

export interface IntChangeBuilder {
  add(delta: bigint): this
}

export interface AttrPatchBuilder {
  set(key: string, value: AttrValueData): this
  remove(key: string): this
}
```

scoped builder 只在同步 callback 期间有效，不能逃逸。每个 callback 事务提交，
失败时回滚该 callback。`build()` 成功时消费根 Builder。Map `set` 是 snapshot-aware
upsert；缺失 key 的 delete 为 Noop。Text replace 是 Delete+Insert sugar。RichText 不提供
通用 insert 或 replace。Builder 收到的 ValueInput 仍执行 Core 合法性验证，但不接收
或隐式应用 InputLimits；输入规模由消费方负责。

## 9. 代数运算

```ts
export function apply(
  base: Value,
  change: Change,
): Value

export function compose(
  first: Change,
  second: Change,
): Change

export function invert(
  change: Change,
  base: Value,
): Change

export interface TransformPairOptions {
  readonly order: "left-first" | "right-first"
}

export function transformPair(
  left: Change,
  right: Change,
  options: TransformPairOptions,
): readonly [Change, Change]
```

代数运算不消费输入。输出是独立、需主动释放的句柄。`transformPair()`
结果顺序与 left/right 输入一一对应；失败时不返回部分结果。

## 10. Text/RichText 坐标

Builder 的 Text/RichText 位置使用 UTF-16 code unit。核心和 wire 使用 Unicode
scalar value；落在 surrogate pair 中间的 UTF-16 位置被拒绝。RichText Embed 在
两套坐标中都计为 1。

```ts
export function resolveCodePointPosition(
  value: Value,
  path: Path,
  utf16Position: number,
): number

export function resolveUtf16Position(
  value: Value,
  path: Path,
  codePointPosition: number,
): number
```

path 必须指向 Text 或 RichText。这两个包级函数只读取 Value，不创建句柄。

## 11. ChangeView

```ts
export function inspectChange(
  change: Change,
  base: Value,
): ChangeView

export type ChangeView =
  readonly ChangeViewEntry[]

interface ChangeViewEntryBase {
  readonly path: Path
}

export type ChangeViewEntry =
  | (ChangeViewEntryBase & {
      readonly type: "value.replace"
      readonly value: ValueData
    })
  | (ChangeViewEntryBase & {
      readonly type: "int.add"
      readonly delta: bigint
    })
  | (ChangeViewEntryBase & {
      readonly type: "map.set"
      readonly key: string
      readonly value: ValueData
    })
  | (ChangeViewEntryBase & {
      readonly type: "map.delete"
      readonly key: string
    })
  | (ChangeViewEntryBase & {
      readonly type: "list.insert"
      readonly index: number
      readonly values: readonly ValueData[]
    })
  | (ChangeViewEntryBase & {
      readonly type: "list.set"
      readonly index: number
      readonly value: ValueData
    })
  | (ChangeViewEntryBase & {
      readonly type: "list.delete"
      readonly range: Range
    })
  | (ChangeViewEntryBase & {
      readonly type: "text.insert"
      readonly at: number
      readonly text: string
    })
  | (ChangeViewEntryBase & {
      readonly type: "text.delete"
      readonly range: Range
    })
  | (ChangeViewEntryBase & {
      readonly type: "richText.insertText"
      readonly at: number
      readonly text: string
      readonly attrs?: AttrsData
    })
  | (ChangeViewEntryBase & {
      readonly type: "richText.insertEmbed"
      readonly at: number
      readonly embed: ValueData
      readonly attrs?: AttrsData
    })
  | (ChangeViewEntryBase & {
      readonly type: "richText.delete"
      readonly range: Range
    })
  | (ChangeViewEntryBase & {
      readonly type: "richText.format"
      readonly range: Range
      readonly patch: AttrPatchView
    })
```

`ChangeView` 是结合 Snapshot 派生的递归冻结、扁平、有序只读投影，不是
Change 的构造或传输格式。entry discriminant 为：

```ts
type ChangeViewEntryType =
  | "value.replace"
  | "int.add"
  | "map.set"
  | "map.delete"
  | "list.insert"
  | "list.set"
  | "list.delete"
  | "text.insert"
  | "text.delete"
  | "richText.insertText"
  | "richText.insertEmbed"
  | "richText.delete"
  | "richText.format"
```

Noop 返回空数组；Text replace 展开为 delete 与 insert。

```ts
export type AttrPatchView = Readonly<
  Record<
    string,
    | {
        readonly type: "set"
        readonly value: AttrValueData
      }
    | { readonly type: "remove" }
  >
>
```

## 12. 错误

```ts
export type CollaErrorCode =
  | "invalid_argument"
  | "invalid_value"
  | "invalid_utf16_boundary"
  | "type_mismatch"
  | "missing_key"
  | "key_already_exists"
  | "out_of_bounds"
  | "integer_overflow"
  | "limit_exceeded"
  | "incompatible_change"
  | "invalid_encoding"
  | "invalid_state"

export type CollaErrorDetails = {
  readonly invalid_argument: {
    readonly argument: string
    readonly reason: string
  }
  readonly invalid_value: {
    readonly reason: string
  }
  readonly invalid_utf16_boundary: {
    readonly position: number
  }
  readonly type_mismatch: {
    readonly expected:
      | ValueKind
      | readonly ValueKind[]
    readonly actual: ValueKind
  }
  readonly missing_key: {
    readonly key: string
  }
  readonly key_already_exists: {
    readonly key: string
  }
  readonly out_of_bounds: {
    readonly target: string
    readonly length: number
    readonly index?: number
    readonly range?: Range
  }
  readonly integer_overflow: Readonly<
    Record<string, never>
  >
  readonly limit_exceeded: {
    readonly limit: string
    readonly actual: number
    readonly maximum: number
  }
  readonly incompatible_change: {
    readonly reason: string
    readonly left?: string
    readonly right?: string
    readonly key?: string
  }
  readonly invalid_encoding: {
    readonly reason: string
    readonly offset?: number
  }
  readonly invalid_state: {
    readonly resource: string
    readonly reason:
      | "disposed"
      | "consumed"
      | "scope_closed"
  }
}

export class CollaError<
  C extends CollaErrorCode = CollaErrorCode,
> extends Error {
  readonly code: C
  readonly operation: string
  readonly path?: Path
  readonly details: CollaErrorDetails[C]

  is<K extends CollaErrorCode>(
    code: K,
  ): this is CollaError<K>
}
```

code 和 operation 使用 lower_snake_case。code 表示可恢复原因，operation 独立表示
失败操作。details 按 code 映射为稳定、递归冻结的类型，已公开必需字段
不得改变，可增加新的可选字段。错误 message、Rust enum 名称与 wasm-bindgen
异常形状不是公共契约。

scoped callback 主动抛出的普通 JavaScript 异常原样传播，不包装为 CollaError。
`limit_exceeded` 只由 `Value.fromJS()`、`Value.decode()` 和 `Change.decode()` 的
InputLimits 检查产生，不属于 Builder、代数、坐标转换或 ChangeView 的错误集合。

## 13. 资源生命周期

- Value、Change 和根 ChangeBuilder 提供幂等 `dispose()` 与 `Symbol.dispose`。
- `FinalizationRegistry` 仅作为遗漏释放的不确定性安全网。
- dispose 后使用句柄抛出 `invalid_state`。
- `Value.clone()` 与 `Change.clone()` 通过 Rust `Arc` 创建独立释放权。
- Builder 不可 clone；`build()` 成功时消费 Builder。
- algebra 输入不被消费。

## 14. package 与 Wasm 包装

- Browser/default ESM 内嵌 base64 Wasm，在模块求值时同步初始化。
- Node.js ESM 条件入口同步读取独立 `.wasm`。
- Browser 与 Node 使用同一份最终 Wasm binary。
- 不使用 top-level await、同步 XHR 或公开 Wasm 初始化函数。
- package 显式声明 `sideEffects: true`；任何根入口 import 都可以初始化 Wasm。
- wasm-bindgen 生成 class、glue 和文件布局是私有 ABI。

Rust crate 与 `@colla/core` 始终使用相同 SemVer 并原子发布。

## 15. Rust API 对齐

Rust 推荐的包级代数入口与 JavaScript 保持相同参数顺序：

```rust
apply(&base, &change)
compose(&first, &second)
invert(&change, &base)
transform_pair(&left, &right, tie_break)
```

Rust Value/Change 提供 `decode()`、`decode_with_limits()` 和实例 `encode()`；
`Value::change()` 创建 Builder。现有 inherent 与 codec 入口可以作为等价底层 API
保留，但代数和 Builder 都不接受 `InputLimits`。Rust 使用 snake_case 字段，
JavaScript 使用 camelCase 完整名称。
