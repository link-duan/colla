import { CoreValue, CoreChange, CoreDocument, CoreHistory, CoreWire, CoreSession, CoreAuthority, core_validate_id, core_apply, core_invert, core_compose, core_transform, } from "./internal/colla_wasm.js";
export type ElementId = string & {
    readonly __elementId: unique symbol;
};
export type Path = readonly (string | number)[];
export type Location = Path | ElementId;
export type ValueKind = "null" | "bool" | "int" | "float" | "string" | "text" | "richtext" | "ref" | "list" | "map";
export type ErrorCode = "invalid_argument" | "invalid_value" | "invalid_encoding" | "invalid_state" | "limit_exceeded" | "type_mismatch" | "missing_key" | "out_of_bounds" | "integer_overflow" | "incompatible_change" | "invalid_utf16_boundary" | "structural_conflict" | "missing_revision" | "history_expired";
export class CollaError extends Error {
    readonly code: ErrorCode;
    readonly operation: string;
    readonly details: Readonly<Record<string, string>>;
    readonly elementId?: ElementId;
    constructor(code: ErrorCode, operation: string, details: Readonly<Record<string, string>> = {}) {
        super(details.reason ?? code);
        this.name = "CollaError";
        this.code = code;
        this.operation = operation;
        this.details = Object.freeze({ ...details });
        this.elementId = details.elementId as ElementId | undefined;
        Object.freeze(this);
    }
}
function fail(code: ErrorCode, operation: string, reason: string): never { throw new CollaError(code, operation, { reason }); }
function invoke<T>(operation: string, fn: () => T): T {
    try {
        return fn();
    }
    catch (error) {
        if (error instanceof CollaError)
            throw error;
        if (error && typeof error === "object" && "code" in error) {
            const native = error as {
                code: ErrorCode;
                details?: Record<string, string>;
            };
            throw new CollaError(native.code, operation, native.details);
        }
        throw new CollaError("invalid_state", operation, { reason: error instanceof Error ? error.message : String(error) });
    }
}
function checkedString(value: unknown, operation = "input"): string {
    if (typeof value !== "string")
        fail("invalid_argument", operation, "expected a string");
    for (let i = 0; i < value.length; i++) {
        const c = value.charCodeAt(i);
        if (c >= 0xd800 && c <= 0xdbff) {
            const next = value.charCodeAt(++i);
            if (!(next >= 0xdc00 && next <= 0xdfff))
                fail("invalid_value", operation, "unpaired UTF-16 surrogate");
        }
        else if (c >= 0xdc00 && c <= 0xdfff)
            fail("invalid_value", operation, "unpaired UTF-16 surrogate");
    }
    return value;
}
function checkedIndex(value: unknown): number {
    if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0)
        fail("invalid_argument", "position", "expected a nonnegative safe integer");
    return value;
}
function bytes(value: unknown): Uint8Array {
    if (!(value instanceof Uint8Array))
        fail("invalid_argument", "decode", "expected Uint8Array");
    return value;
}
export const ElementId = Object.freeze({ parse(value: string): ElementId { return invoke("ElementId.parse", () => core_validate_id(checkedString(value)) as ElementId); } });
function locate(value: Location): string | (string | number)[] {
    if (typeof value === "string")
        return ElementId.parse(value);
    if (!Array.isArray(value))
        fail("invalid_argument", "location", "expected a Path or ElementId");
    return value.map(part => typeof part === "string" ? checkedString(part) : checkedIndex(part));
}
export type AttrValue = boolean | bigint | number | string;
export type Attrs = Readonly<Record<string, AttrValue>>;
export type AttrPatch = Readonly<Record<string, AttrValue | null>>;
export type Input = null | boolean | bigint | number | string | Text | RichText | Ref | Value | readonly Input[] | InputMap;
export interface InputMap {
    readonly [key: string]: Input;
}
export type RichTextSpan = {
    readonly type: "text";
    readonly text: string;
    readonly attrs?: Attrs;
} | {
    readonly type: "embed";
    readonly value: Input;
    readonly attrs?: Attrs;
};
function entries(value: object): [
    string,
    unknown
][] {
    if (Object.getPrototypeOf(value) !== Object.prototype && Object.getPrototypeOf(value) !== null)
        fail("invalid_argument", "input", "expected a plain object");
    if (Object.getOwnPropertySymbols(value).length)
        fail("invalid_argument", "input", "symbol keys are unsupported");
    return Object.entries(Object.getOwnPropertyDescriptors(value)).map(([key, descriptor]) => {
        if (!("value" in descriptor))
            fail("invalid_argument", "input", "accessor properties are unsupported");
        checkedString(key);
        return [key, descriptor.value];
    });
}
function attributes(value: Attrs | AttrPatch = {}, patch = false): [
    string,
    AttrValue | null
][] {
    return entries(value).map(([key, value]) => {
        if (value === null && patch)
            return [key, null];
        if (!["boolean", "bigint", "number", "string"].includes(typeof value))
            fail("invalid_argument", "attributes", "attribute values must be atomic");
        if (typeof value === "string")
            checkedString(value);
        return [key, value as AttrValue];
    });
}
function dataObject<T>(items: readonly (readonly [
    string,
    T
])[]): Readonly<Record<string, T>> {
    const out: Record<string, T> = {};
    for (const [key, value] of items)
        Object.defineProperty(out, key, { value, enumerable: true });
    return Object.freeze(out);
}
export class Text {
    readonly type = "text";
    readonly value: string;
    constructor(value: string) { this.value = checkedString(value); Object.freeze(this); }
}
export class Ref {
    readonly target: ElementId;
    constructor(target: ElementId) { this.target = ElementId.parse(target); Object.freeze(this); }
}
export class RichText {
    readonly type = "richtext";
    readonly spans: readonly RichTextSpan[];
    constructor(spans: readonly RichTextSpan[]) {
        if (!Array.isArray(spans))
            fail("invalid_argument", "richText", "expected spans array");
        this.spans = Object.freeze(spans.map(span => span.type === "text"
            ? Object.freeze({ type: "text" as const, text: checkedString(span.text), attrs: dataObject(attributes(span.attrs)) as Attrs })
            : span.type === "embed"
                ? Object.freeze({ type: "embed" as const, value: immutableInput(span.value), attrs: dataObject(attributes(span.attrs)) as Attrs })
                : fail("invalid_argument", "richText", "invalid span type")));
        Object.freeze(this);
    }
}
export function text(value: string): Text { return new Text(value); }
export function richText(spans: readonly RichTextSpan[]): RichText { return new RichText(spans); }
export function ref(target: ElementId): Ref { return new Ref(target); }
function immutableInput(input: Input, active = new Set<object>(), depth = 0): Input {
    if (depth > 100)
        fail("limit_exceeded", "input", "value depth exceeded");
    if (input instanceof Value || input instanceof Text || input instanceof RichText || input instanceof Ref)
        return input;
    if (typeof input === "string")
        return checkedString(input);
    if (input === null || ["boolean", "bigint", "number"].includes(typeof input))
        return input;
    if (!input || typeof input !== "object")
        fail("invalid_argument", "input", "unsupported value");
    if (active.has(input))
        fail("invalid_value", "input", "owning content cannot contain cycles");
    active.add(input);
    const out = Array.isArray(input) ? Object.freeze(input.map(v => immutableInput(v, active, depth + 1)))
        : dataObject(entries(input).map(([k, v]) => [k, immutableInput(v as Input, active, depth + 1)]));
    active.delete(input);
    return out;
}
type Tagged = unknown[];
function inputNode(input: Input, active = new Set<object>(), depth = 0): Tagged {
    if (depth > 100)
        fail("limit_exceeded", "input", "value depth exceeded");
    if (input instanceof Value)
        return [10, input.encode()];
    if (input instanceof Text)
        return [5, input.value];
    if (input instanceof Ref)
        return [7, input.target];
    if (input instanceof RichText)
        return [6, input.spans.map(span => spanInput(span, active, depth + 1))];
    if (input === null)
        return [0];
    if (typeof input === "boolean")
        return [1, input];
    if (typeof input === "bigint")
        return [2, input];
    if (typeof input === "number")
        return [3, input];
    if (typeof input === "string")
        return [4, checkedString(input)];
    if (!input || typeof input !== "object")
        fail("invalid_argument", "input", "unsupported value");
    if (active.has(input))
        fail("invalid_value", "input", "owning content cannot contain cycles");
    active.add(input);
    const out = Array.isArray(input) ? [8, input.map(v => inputNode(v, active, depth + 1))]
        : [9, entries(input).map(([key, value]) => [key, inputNode(value as Input, active, depth + 1)])];
    active.delete(input);
    return out;
}
function spanInput(span: RichTextSpan, active = new Set<object>(), depth = 0): Tagged {
    if (span.type === "text")
        return [0, checkedString(span.text), attributes(span.attrs)];
    if (span.type === "embed")
        return [1, inputNode(span.value, active, depth + 1), attributes(span.attrs)];
    return fail("invalid_argument", "richText", "invalid span type");
}
function projectedSpan(span: any[]): RichTextSpan {
    return span[0] === 0 ? Object.freeze({ type: "text", text: span[1], attrs: dataObject(span[2]) as Attrs })
        : Object.freeze({ type: "embed", value: span[1] instanceof CoreValue ? wrapValue(span[1]) : project(span[1]), attrs: dataObject(span[2]) as Attrs });
}
function project(node: any[]): Input {
    switch (node[0]) {
        case 0: return null;
        case 1:
        case 2:
        case 3:
        case 4: return node[1];
        case 5: return text(node[1]);
        case 6: return richText(node[1].map(projectedSpan));
        case 7: return ref(node[1]);
        case 8: return Object.freeze(node[1].map(project));
        case 9: return dataObject(node[1].map(([key, value]: [
            string,
            any[]
        ]) => [key, project(value)]));
        default: return fail("invalid_state", "projection", "invalid Rust projection");
    }
}
const token = Symbol("Colla internal construction");
const values = new WeakMap<Value, CoreValue>();
const changes = new WeakMap<Change, CoreChange>();
function rawValue(value: Value): CoreValue { const raw = values.get(value); if (!raw)
    fail("invalid_argument", "Value", "expected Value"); return raw; }
function rawChange(change: Change): CoreChange { const raw = changes.get(change); if (!raw)
    fail("invalid_argument", "Change", "expected Change"); return raw; }
function wrapValue(raw: CoreValue): Value { return new (Value as any)(token, raw); }
function wrapChange(raw: CoreChange): Change { return new (Change as any)(token, raw); }
export class Value {
    private constructor(key: symbol, raw: CoreValue) {
        if (key !== token)
            fail("invalid_argument", "Value", "use Value.fromJS or Value.decode");
        values.set(this, raw);
        Object.freeze(this);
    }
    static fromJS(input: Input): Value { if (input instanceof Value)
        return input; return invoke("Value.fromJS", () => wrapValue(CoreValue.from_input(inputNode(input)))); }
    static decode(input: Uint8Array): Value { return invoke("Value.decode", () => wrapValue(CoreValue.decode(bytes(input)))); }
    get id(): ElementId { return rawValue(this).id() as ElementId; }
    encode(): Uint8Array { return invoke("Value.encode", () => rawValue(this).encode()); }
    toJS(): Input { return invoke("Value.toJS", () => project(rawValue(this).projection())); }
    get(location: Location = []): Value | undefined {
        try {
            return invoke("Value.get", () => wrapValue(rawValue(this).get(locate(location))));
        }
        catch (error) {
            if (error instanceof CollaError && (error.code === "missing_key" || error.code === "out_of_bounds"))
                return undefined;
            throw error;
        }
    }
    has(location: Location): boolean { return this.get(location) !== undefined; }
    kind(location: Location = []): ValueKind | undefined { const value = Array.isArray(location) && location.length === 0 ? this : this.get(location); return value && invoke("Value.kind", () => rawValue(value).kind() as ValueKind); }
    idAt(path: Path): ElementId { const value = this.get(path); if (!value)
        fail("missing_key", "Value.idAt", "element does not exist"); return value.id; }
    pathOf(id: ElementId): Path | undefined { const path = invoke("Value.pathOf", () => rawValue(this).path_of(ElementId.parse(id))); return path && Object.freeze(path); }
    resolve(reference: Ref): Value | undefined {
        if (!(reference instanceof Ref))
            fail("invalid_argument", "Value.resolve", "expected Ref");
        const value = invoke("Value.resolve", () => rawValue(this).resolve(reference.target));
        return value && wrapValue(value);
    }
    referencesTo(id: ElementId): readonly ElementId[] { return Object.freeze(invoke("Value.referencesTo", () => rawValue(this).references_to(ElementId.parse(id))) as ElementId[]); }
    equals(other: Value): boolean { return invoke("Value.equals", () => rawValue(this).equals(rawValue(other))); }
    contentEquals(other: Value): boolean { return invoke("Value.contentEquals", () => rawValue(this).content_equals(rawValue(other))); }
    copy(): Value { return invoke("Value.copy", () => wrapValue(rawValue(this).copied())); }
}
export type Destination = {
    readonly parent: ElementId;
    readonly key: string;
    readonly index?: never;
} | {
    readonly parent: ElementId;
    readonly index: number;
    readonly key?: never;
};
export type MoveTarget = {
    readonly parent: Location;
    readonly key: string;
    readonly index?: never;
} | {
    readonly parent: Location;
    readonly index: number;
    readonly key?: never;
};
export type TextOp = {
    readonly type: "retain" | "delete";
    readonly length: number;
} | {
    readonly type: "insert";
    readonly text: string;
};
export type RichTextOp = {
    readonly type: "retain";
    readonly length: number;
    readonly attrs?: AttrPatch;
} | {
    readonly type: "delete";
    readonly length: number;
} | {
    readonly type: "insert";
    readonly span: RichTextSpan;
};
export type Operation = {
    readonly type: "insert";
    readonly destination: Destination;
    readonly value: Value;
} | {
    readonly type: "delete";
    readonly target: ElementId;
} | {
    readonly type: "set";
    readonly target: ElementId;
    readonly value: Value;
} | {
    readonly type: "move";
    readonly target: ElementId;
    readonly destination: Destination;
} | {
    readonly type: "text";
    readonly target: ElementId;
    readonly operations: readonly TextOp[];
} | {
    readonly type: "add";
    readonly target: ElementId;
    readonly delta: bigint;
} | {
    readonly type: "richtext";
    readonly target: ElementId;
    readonly operations: readonly RichTextOp[];
};
export type EditStep = Operation;
function destinationInput(value: Destination | MoveTarget, identity = true): Tagged {
    if ((value.key === undefined) === (value.index === undefined))
        fail("invalid_argument", "destination", "provide exactly one key or index");
    return [identity ? ElementId.parse(value.parent as ElementId) : locate(value.parent), value.key === undefined ? checkedIndex(value.index) : checkedString(value.key)];
}
function textOperationInput(op: TextOp): Tagged {
    if (op.type === "retain")
        return [0, checkedIndex(op.length)];
    if (op.type === "insert")
        return [1, checkedString(op.text)];
    if (op.type === "delete")
        return [2, checkedIndex(op.length)];
    return fail("invalid_argument", "Change.create", "unknown Text operation");
}
function richOperationInput(op: RichTextOp): Tagged {
    if (op.type === "retain")
        return [0, checkedIndex(op.length), attributes(op.attrs, true)];
    if (op.type === "insert")
        return [1, spanInput(op.span)];
    if (op.type === "delete")
        return [2, checkedIndex(op.length)];
    return fail("invalid_argument", "Change.create", "unknown RichText operation");
}
function operationInput(op: Operation): Tagged {
    switch (op.type) {
        case "insert": return [0, destinationInput(op.destination), inputNode(op.value)];
        case "delete": return [1, ElementId.parse(op.target)];
        case "set": return [2, ElementId.parse(op.target), inputNode(op.value)];
        case "move": return [3, ElementId.parse(op.target), destinationInput(op.destination)];
        case "text": return [4, ElementId.parse(op.target), op.operations.map(textOperationInput)];
        case "add": return [5, ElementId.parse(op.target), op.delta];
        case "richtext": return [6, ElementId.parse(op.target), op.operations.map(richOperationInput)];
        default: return fail("invalid_argument", "Change.create", "unknown operation");
    }
}
function operationProjection(op: any[]): Operation {
    const destination = (d: any[]): Destination => Object.freeze(typeof d[1] === "string" ? { parent: d[0], key: d[1] } : { parent: d[0], index: d[1] });
    switch (op[0]) {
        case 0: return Object.freeze({ type: "insert", destination: destination(op[1]), value: wrapValue(op[2]) });
        case 1: return Object.freeze({ type: "delete", target: op[1] });
        case 2: return Object.freeze({ type: "set", target: op[1], value: wrapValue(op[2]) });
        case 3: return Object.freeze({ type: "move", target: op[1], destination: destination(op[2]) });
        case 4: return Object.freeze({ type: "text", target: op[1], operations: Object.freeze(op[2].map((op: any[]) => Object.freeze(op[0] === 1 ? { type: "insert", text: op[1] } : { type: op[0] === 0 ? "retain" : "delete", length: op[1] }))) });
        case 5: return Object.freeze({ type: "add", target: op[1], delta: op[2] });
        case 6: return Object.freeze({ type: "richtext", target: op[1], operations: Object.freeze(op[2].map((op: any[]) => Object.freeze(op[0] === 1 ? { type: "insert", span: projectedSpan(op[1]) } : op[0] === 0 ? { type: "retain", length: op[1], attrs: dataObject(op[2]) } : { type: "delete", length: op[1] }))) });
        default: return fail("invalid_state", "Change.operations", "invalid Rust operation projection");
    }
}
export class Change {
    private constructor(key: symbol, raw: CoreChange) { if (key !== token)
        fail("invalid_argument", "Change", "use Change.create or Change.decode"); changes.set(this, raw); Object.freeze(this); }
    static create(operations: readonly Operation[]): Change { if (!Array.isArray(operations))
        fail("invalid_argument", "Change.create", "expected operations array"); return invoke("Change.create", () => wrapChange(CoreChange.from_input(operations.map(operationInput)))); }
    static decode(input: Uint8Array): Change { return invoke("Change.decode", () => wrapChange(CoreChange.decode(bytes(input)))); }
    static noop(): Change { return Change.create([]); }
    get isNoop(): boolean { return rawChange(this).is_noop(); }
    get operations(): readonly Operation[] { return Object.freeze(invoke("Change.operations", () => rawChange(this).operations().map(operationProjection))); }
    encode(): Uint8Array { return invoke("Change.encode", () => rawChange(this).encode()); }
}
export function apply(base: Value, change: Change): Value { return invoke("apply", () => wrapValue(core_apply(rawValue(base), rawChange(change)))); }
export function invert(base: Value, change: Change): Change { return invoke("invert", () => wrapChange(core_invert(rawValue(base), rawChange(change)))); }
export function compose(base: Value, first: Change, second: Change): Change { return invoke("compose", () => wrapChange(core_compose(rawValue(base), rawChange(first), rawChange(second)))); }
export function transform(base: Value, left: Change, right: Change, options: {
    readonly priority: "left" | "right";
}): readonly [
    Change,
    Change
] {
    if (!options || !["left", "right"].includes(options.priority))
        fail("invalid_argument", "transform", "priority must be left or right");
    return invoke("transform", () => { const result = core_transform(rawValue(base), rawChange(left), rawChange(right), options.priority === "left"); return Object.freeze([wrapChange(result[0]), wrapChange(result[1])]); });
}
export type Origin = "local" | "remote" | "undo" | "redo";
export interface EditResult {
    readonly before: Value;
    readonly after: Value;
    readonly change: Change;
    readonly inverse: Change;
    readonly editSteps: readonly EditStep[];
    readonly version: bigint;
    readonly origin: Origin;
}
export type EditEvent = EditResult;
export interface SubscribeOptions {
    readonly onError?: (error: unknown) => void;
}
type Listener<T> = {
    listener: (event: T) => void;
    onError?: (error: unknown) => void;
};
function subscribe<T>(listeners: Set<Listener<T>>, listener: (event: T) => void, options: SubscribeOptions = {}): () => void {
    if (typeof listener !== "function" || (options.onError !== undefined && typeof options.onError !== "function"))
        fail("invalid_argument", "subscribe", "expected listener functions");
    const item = { listener, onError: options.onError };
    listeners.add(item);
    return () => { listeners.delete(item); };
}
function dispatch<T>(listeners: Set<Listener<T>>, event: T): void {
    for (const item of [...listeners]) {
        try {
            item.listener(event);
        }
        catch (error) {
            try {
                item.onError?.(error);
            }
            catch { /* Listener diagnostics are isolated too. */ }
        }
    }
}
function editResult(raw: any): EditResult | null {
    return raw === null ? null : Object.freeze({ before: wrapValue(raw.before), after: wrapValue(raw.after), change: wrapChange(raw.change), inverse: wrapChange(raw.inverse), editSteps: Object.freeze(raw.editSteps.map(operationProjection)), version: raw.version, origin: raw.origin });
}
abstract class Reader {
    abstract snapshot(): Value;
    get(location: Location = []): Value | undefined { return this.snapshot().get(location); }
    has(location: Location): boolean { return this.snapshot().has(location); }
    kind(location: Location = []): ValueKind | undefined { return this.snapshot().kind(location); }
    idAt(path: Path): ElementId { return this.snapshot().idAt(path); }
    pathOf(id: ElementId): Path | undefined { return this.snapshot().pathOf(id); }
    resolve(reference: Ref): Value | undefined { return this.snapshot().resolve(reference); }
    referencesTo(id: ElementId): readonly ElementId[] { return this.snapshot().referencesTo(id); }
}
type DocumentState = {
    raw: CoreDocument;
    editing: boolean;
    dispatching: boolean;
    closed: boolean;
    listeners: Set<Listener<EditEvent>>;
    session?: SyncSession;
};
const documents = new WeakMap<Document, DocumentState>();
function documentState(doc: Document): DocumentState { const state = documents.get(doc); if (!state)
    fail("invalid_argument", "Document", "expected Document"); return state; }
function checkRead(doc: Document): DocumentState { const state = documentState(doc); if (state.closed)
    fail("invalid_state", "Document", "document is closed"); return state; }
function checkWrite(doc: Document): DocumentState { const state = checkRead(doc); if (state.editing || state.dispatching)
    fail("invalid_state", "Document", "modification during an active edit or event dispatch"); return state; }
function wrapDocument(raw: CoreDocument): Document { return new (Document as any)(token, raw); }
function deliver(doc: Document, raw: any): EditResult | null {
    const result = editResult(raw);
    const state = documentState(doc);
    if (result) {
        state.dispatching = true;
        try {
            dispatch(state.listeners, result);
        }
        finally {
            state.dispatching = false;
        }
    }
    if (state.session)
        notifySession(state.session);
    return result;
}
export class Document extends Reader {
    private constructor(key: symbol, raw: CoreDocument) {
        super();
        if (key !== token)
            fail("invalid_argument", "Document", "use Document.create");
        documents.set(this, { raw, editing: false, dispatching: false, closed: false, listeners: new Set() });
        Object.freeze(this);
    }
    static create(input: Input): Document { return invoke("Document.create", () => wrapDocument(CoreDocument.create(rawValue(Value.fromJS(input))))); }
    snapshot(): Value { return invoke("Document.snapshot", () => wrapValue(checkRead(this).raw.snapshot())); }
    get version(): bigint { return invoke("Document.version", () => checkRead(this).raw.version()); }
    edit(callback: (tx: Transaction) => unknown, options: {
        readonly group?: string;
    } = {}): EditResult | null {
        const state = checkWrite(this);
        if (typeof callback !== "function")
            fail("invalid_argument", "Document.edit", "expected a synchronous callback");
        const group = options.group === undefined ? undefined : checkedString(options.group);
        invoke("Document.edit", () => state.raw.begin(group));
        state.editing = true;
        const scope: Scope = { state, active: true };
        let result: any;
        try {
            const returned = callback(new (Transaction as any)(token, scope));
            if (returned !== null && (typeof returned === "object" || typeof returned === "function") && typeof (returned as any).then === "function") {
                // Observe real Promise rejection without executing an arbitrary thenable.
                try {
                    Promise.prototype.then.call(returned, undefined, () => { });
                }
                catch { /* not a Promise */ }
                fail("invalid_state", "Document.edit", "callback must be synchronous");
            }
            result = invoke("Document.edit", () => state.raw.commit());
        }
        catch (error) {
            if (error instanceof CollaError)
                throw error;
            throw new CollaError("invalid_state", "Document.edit", { reason: error instanceof Error ? error.message : String(error) });
        }
        finally {
            scope.active = false;
            state.raw.rollback();
            state.editing = false;
        }
        return deliver(this, result);
    }
    apply(change: Change): EditResult | null { const state = checkWrite(this); return deliver(this, invoke("Document.apply", () => state.raw.apply(rawChange(change)))); }
    subscribe(listener: (event: EditEvent) => void, options: SubscribeOptions = {}): () => void { return subscribe(checkRead(this).listeners, listener, options); }
    close(): void {
        const state = documentState(this);
        if (state.closed)
            return;
        checkWrite(this);
        invoke("Document.close", () => state.raw.close());
        state.closed = true;
        state.listeners.clear();
        if (state.session)
            notifySession(state.session);
    }
}
type Scope = {
    state: DocumentState;
    active: boolean;
};
const scopes = new WeakMap<Transaction, Scope>();
function scoped(tx: Transaction): Scope { const scope = scopes.get(tx); if (!scope || !scope.active)
    fail("invalid_state", "Transaction", "transaction scope has ended"); return scope; }
function command(tx: Transaction, args: Tagged): any { const scope = scoped(tx); return invoke("Transaction.edit", () => scope.state.raw.command(args)); }
export class Transaction extends Reader {
    private constructor(key: symbol, scope: Scope) { super(); if (key !== token)
        fail("invalid_argument", "Transaction", "transactions come from Document.edit"); scopes.set(this, scope); Object.freeze(this); }
    snapshot(): Value { return invoke("Transaction.snapshot", () => wrapValue(scoped(this).state.raw.transaction_snapshot())); }
    set(location: Location, value: Input): void { scoped(this); command(this, [0, locate(location), inputNode(value)]); }
    delete(location: Location): void { scoped(this); command(this, [1, locate(location)]); }
    move(source: Location, target: MoveTarget): void { scoped(this); const [parent, slot] = destinationInput(target, false); command(this, [2, locate(source), parent, slot]); }
    copy(source: Location, target: MoveTarget): ElementId { scoped(this); const [parent, slot] = destinationInput(target, false); return command(this, [3, locate(source), parent, slot]) as ElementId; }
    increment(location: Location, delta: bigint): void { scoped(this); command(this, [4, locate(location), delta]); }
    apply(change: Change): void { invoke("Transaction.apply", () => scoped(this).state.raw.transaction_apply(rawChange(change))); }
    list(location: Location): ListEditor { return new (ListEditor as any)(token, this, editorTarget(this, location, "list")); }
    text(location: Location): TextEditor { return new (TextEditor as any)(token, this, editorTarget(this, location, "text")); }
    richText(location: Location): RichTextEditor { return new (RichTextEditor as any)(token, this, editorTarget(this, location, "richtext")); }
}
function editorTarget(tx: Transaction, location: Location, kind: ValueKind): ElementId {
    const value = tx.get(location);
    if (!value)
        fail("missing_key", "Transaction", "editor target does not exist");
    if (value.kind() !== kind)
        fail("type_mismatch", "Transaction", `expected ${kind}`);
    return value.id;
}
const editors = new WeakMap<object, {
    tx: Transaction;
    target: ElementId;
}>();
function editor(editor: object): {
    tx: Transaction;
    target: ElementId;
} { const item = editors.get(editor); if (!item)
    fail("invalid_state", "editor", "invalid editor"); scoped(item.tx); return item; }
export class ListEditor {
    private constructor(key: symbol, tx: Transaction, target: ElementId) { if (key !== token)
        fail("invalid_argument", "ListEditor", "use tx.list"); editors.set(this, { tx, target }); Object.freeze(this); }
    insert(index: number, values: readonly Input[]): void { this.replace(index, 0, values); }
    delete(index: number, count: number): void { this.replace(index, count, []); }
    replace(index: number, count: number, values: readonly Input[]): void {
        const { tx, target } = editor(this);
        if (!Array.isArray(values))
            fail("invalid_argument", "ListEditor.replace", "expected values array");
        command(tx, [5, target, checkedIndex(index), checkedIndex(count), values.map(v => inputNode(v))]);
    }
}
export class TextEditor {
    private constructor(key: symbol, tx: Transaction, target: ElementId) { if (key !== token)
        fail("invalid_argument", "TextEditor", "use tx.text"); editors.set(this, { tx, target }); Object.freeze(this); }
    insert(index: number, value: string): void { this.replace(index, 0, value); }
    delete(index: number, count: number): void { this.replace(index, count, ""); }
    replace(index: number, count: number, value: string): void { const { tx, target } = editor(this); command(tx, [6, target, checkedIndex(index), checkedIndex(count), checkedString(value)]); }
}
export class RichTextEditor {
    private constructor(key: symbol, tx: Transaction, target: ElementId) { if (key !== token)
        fail("invalid_argument", "RichTextEditor", "use tx.richText"); editors.set(this, { tx, target }); Object.freeze(this); }
    insertText(index: number, value: string, attrs: Attrs = {}): void { this.replace(index, 0, [{ type: "text", text: value, attrs }]); }
    insertEmbed(index: number, value: Input, attrs: Attrs = {}): void { this.replace(index, 0, [{ type: "embed", value, attrs }]); }
    delete(index: number, count: number): void { this.replace(index, count, []); }
    replace(index: number, count: number, spans: readonly RichTextSpan[]): void {
        const { tx, target } = editor(this);
        if (!Array.isArray(spans))
            fail("invalid_argument", "RichTextEditor.replace", "expected spans array");
        command(tx, [7, target, checkedIndex(index), checkedIndex(count), spans.map(span => spanInput(span))]);
    }
    format(index: number, count: number, patch: AttrPatch): void { const { tx, target } = editor(this); command(tx, [8, target, checkedIndex(index), checkedIndex(count), attributes(patch, true)]); }
}
const wires = new WeakMap<WireObject, CoreWire>();
function rawWire(value: WireObject): CoreWire { const wire = wires.get(value); if (!wire)
    fail("invalid_argument", "protocol", "expected a controlled protocol object"); return wire; }
class WireObject {
    /** @internal */
    protected constructor(key: symbol, raw: CoreWire) { if (key !== token)
        fail("invalid_argument", "protocol", "use a codec or runtime constructor"); wires.set(this, raw); }
    encode(): Uint8Array { return invoke("encode", () => rawWire(this).encode()); }
}
export class SyncSnapshot extends WireObject {
    readonly documentId: string;
    readonly revision: bigint;
    readonly value: Value;
    private constructor(key: symbol, raw: CoreWire) { super(key, raw); const info = raw.info(); this.documentId = info.documentId; this.revision = info.revision; this.value = wrapValue(info.value); Object.freeze(this); }
    static decode(input: Uint8Array): SyncSnapshot { return invoke("SyncSnapshot.decode", () => new SyncSnapshot(token, CoreWire.decode(3, bytes(input)))); }
}
export class Submission extends WireObject {
    readonly documentId: string;
    readonly clientId: string;
    readonly sequence: bigint;
    readonly baseRevision: bigint;
    readonly change: Change;
    private constructor(key: symbol, raw: CoreWire) { super(key, raw); const info = raw.info(); this.documentId = info.documentId; this.clientId = info.clientId; this.sequence = info.sequence; this.baseRevision = info.baseRevision; this.change = wrapChange(info.change); Object.freeze(this); }
    static decode(input: Uint8Array): Submission { return invoke("Submission.decode", () => new Submission(token, CoreWire.decode(4, bytes(input)))); }
}
export class ServerMessage extends WireObject {
    readonly type: "commit" | "rejection";
    readonly documentId: string;
    readonly clientId: string;
    readonly sequence: bigint;
    readonly revision?: bigint;
    readonly change?: Change;
    readonly reason?: CollaError;
    private constructor(key: symbol, raw: CoreWire) {
        super(key, raw);
        const info = raw.info();
        this.type = info.type;
        this.documentId = info.documentId;
        this.clientId = info.clientId;
        this.sequence = info.sequence;
        this.revision = info.revision;
        this.change = info.change && wrapChange(info.change);
        this.reason = info.reason && new CollaError(info.reason.code, info.reason.operation, info.reason.details);
        Object.freeze(this);
    }
    static decode(input: Uint8Array): ServerMessage { return invoke("ServerMessage.decode", () => new ServerMessage(token, CoreWire.decode(5, bytes(input)))); }
}
export class SessionCheckpoint extends WireObject {
    private constructor(key: symbol, raw: CoreWire) { super(key, raw); Object.freeze(this); }
    static decode(input: Uint8Array): SessionCheckpoint { return invoke("SessionCheckpoint.decode", () => new SessionCheckpoint(token, CoreWire.decode(6, bytes(input)))); }
}
export class HistoryCheckpoint extends WireObject {
    private constructor(key: symbol, raw: CoreWire) { super(key, raw); Object.freeze(this); }
    static decode(input: Uint8Array): HistoryCheckpoint { return invoke("HistoryCheckpoint.decode", () => new HistoryCheckpoint(token, CoreWire.decode(7, bytes(input)))); }
}
export class AuthorityCheckpoint extends WireObject {
    private constructor(key: symbol, raw: CoreWire) { super(key, raw); Object.freeze(this); }
    static decode(input: Uint8Array): AuthorityCheckpoint { return invoke("AuthorityCheckpoint.decode", () => new AuthorityCheckpoint(token, CoreWire.decode(8, bytes(input)))); }
}
function wrapWire<T extends WireObject>(kind: {
    prototype: T;
}, wire: CoreWire): T { return new (kind as any)(token, wire); }
type HistoryState = {
    raw: CoreHistory;
    document: Document;
    closed: boolean;
};
const histories = new WeakMap<History, HistoryState>();
const attached = new WeakMap<Document, History>();
function historyState(history: History): HistoryState { const state = histories.get(history); if (!state || state.closed)
    fail("invalid_state", "History", "history is closed"); return state; }
export class History {
    private constructor(key: symbol, raw: CoreHistory, document: Document) { if (key !== token)
        fail("invalid_argument", "History", "use History.attach"); histories.set(this, { raw, document, closed: false }); Object.freeze(this); }
    static attach(document: Document, options: {
        readonly capacity?: number;
    } = {}): History {
        const existing = attached.get(document);
        if (existing)
            return existing;
        const state = checkWrite(document);
        const history = new History(token, invoke("History.attach", () => state.raw.history(checkedIndex(options.capacity ?? 100))), document);
        attached.set(document, history);
        return history;
    }
    static restore(document: Document, checkpoint: HistoryCheckpoint): History {
        if (!(checkpoint instanceof HistoryCheckpoint))
            fail("invalid_argument", "History.restore", "expected HistoryCheckpoint");
        const raw = invoke("History.restore", () => checkWrite(document).raw.restore_history(rawWire(checkpoint)));
        const existing = attached.get(document);
        if (existing) {
            historyState(existing).raw = raw;
            return existing;
        }
        const history = new History(token, raw, document);
        attached.set(document, history);
        return history;
    }
    get canUndo(): boolean { return invoke("History.canUndo", () => historyState(this).raw.can_undo()); }
    get canRedo(): boolean { return invoke("History.canRedo", () => historyState(this).raw.can_redo()); }
    undo(): EditResult | null { const state = historyState(this); checkWrite(state.document); return deliver(state.document, invoke("History.undo", () => state.raw.undo())); }
    redo(): EditResult | null { const state = historyState(this); checkWrite(state.document); return deliver(state.document, invoke("History.redo", () => state.raw.redo())); }
    clear(): void { const state = historyState(this); checkWrite(state.document); invoke("History.clear", () => state.raw.clear()); }
    checkpoint(): HistoryCheckpoint { return invoke("History.checkpoint", () => wrapWire(HistoryCheckpoint, historyState(this).raw.checkpoint())); }
    close(): void {
        const state = histories.get(this);
        if (!state || state.closed)
            return;
        if (!documentState(state.document).closed)
            checkWrite(state.document);
        invoke("History.close", () => state.raw.close());
        state.closed = true;
        if (attached.get(state.document) === this)
            attached.delete(state.document);
    }
}
const authorities = new WeakMap<Authority, CoreAuthority>();
function rawAuthority(authority: Authority): CoreAuthority { const raw = authorities.get(authority); if (!raw)
    fail("invalid_argument", "Authority", "expected Authority"); return raw; }
export class Authority {
    private constructor(key: symbol, raw: CoreAuthority) { if (key !== token)
        fail("invalid_argument", "Authority", "use Authority.create or Authority.restore"); authorities.set(this, raw); Object.freeze(this); }
    static create(options: {
        readonly documentId: string;
        readonly value: Input;
    }): Authority { return invoke("Authority.create", () => new Authority(token, CoreAuthority.create(checkedString(options.documentId), rawValue(Value.fromJS(options.value))))); }
    static restore(checkpoint: AuthorityCheckpoint): Authority { if (!(checkpoint instanceof AuthorityCheckpoint))
        fail("invalid_argument", "Authority.restore", "expected AuthorityCheckpoint"); return invoke("Authority.restore", () => new Authority(token, CoreAuthority.restore(rawWire(checkpoint)))); }
    get revision(): bigint { return invoke("Authority.revision", () => rawAuthority(this).revision()); }
    snapshot(): SyncSnapshot { return wrapWire(SyncSnapshot, rawAuthority(this).snapshot()); }
    checkpoint(): AuthorityCheckpoint { return wrapWire(AuthorityCheckpoint, rawAuthority(this).checkpoint()); }
    compact(throughRevision: bigint): Authority { return invoke("Authority.compact", () => new Authority(token, rawAuthority(this).compact(throughRevision))); }
    commitsSince(revision: bigint): readonly ServerMessage[] { return invoke("Authority.commitsSince", () => Object.freeze(rawAuthority(this).commits_since(revision).map(raw => wrapWire(ServerMessage, raw)))); }
    accept(submission: Submission): {
        readonly authority: Authority;
        readonly message: ServerMessage;
    } {
        if (!(submission instanceof Submission))
            fail("invalid_argument", "Authority.accept", "expected Submission");
        return invoke("Authority.accept", () => { const [authority, message] = rawAuthority(this).accept(rawWire(submission)); return Object.freeze({ authority: new Authority(token, authority), message: wrapWire(ServerMessage, message) }); });
    }
}
export interface SyncState {
    readonly status: "active" | "recovery-required" | "closed";
    readonly revision: bigint;
    readonly hasOutbound: boolean;
    readonly recoveryReason?: CollaError;
}
type SessionState = {
    raw: CoreSession;
    document: Document;
    listeners: Set<Listener<SyncState>>;
    last: SyncState;
    fingerprint: string;
};
const sessions = new WeakMap<SyncSession, SessionState>();
function sessionState(session: SyncSession): SessionState { const state = sessions.get(session); if (!state)
    fail("invalid_argument", "SyncSession", "expected SyncSession"); return state; }
function currentSyncState(state: SessionState): SyncState {
    if (documentState(state.document).closed)
        return Object.freeze({ ...state.last, status: "closed", hasOutbound: false });
    const raw = invoke("SyncSession.state", () => ({ revision: state.raw.revision(), outbound: state.raw.outbound(), reason: state.raw.recovery_reason() }));
    return Object.freeze({ revision: raw.revision, status: raw.reason ? "recovery-required" : "active", hasOutbound: raw.outbound !== undefined, recoveryReason: raw.reason && new CollaError(raw.reason.code, raw.reason.operation, raw.reason.details) });
}
function syncFingerprint(state: SyncState): string { return `${state.status}:${state.revision}:${state.hasOutbound}:${state.recoveryReason?.code}:${state.recoveryReason?.message}`; }
function notifySession(session: SyncSession): void {
    const state = sessionState(session);
    const next = currentSyncState(state);
    const fingerprint = syncFingerprint(next);
    if (fingerprint === state.fingerprint)
        return;
    state.fingerprint = fingerprint;
    state.last = next;
    const document = documentState(state.document);
    const previous = document.dispatching;
    document.dispatching = true;
    try {
        dispatch(state.listeners, next);
    }
    finally {
        document.dispatching = previous;
    }
}
export class SyncSession {
    readonly document: Document;
    private constructor(key: symbol, raw: CoreSession) {
        if (key !== token)
            fail("invalid_argument", "SyncSession", "use SyncSession.create or SyncSession.restore");
        this.document = wrapDocument(raw.document());
        const state: SessionState = { raw, document: this.document, listeners: new Set(), last: Object.freeze({ status: "active", revision: 0n, hasOutbound: false }), fingerprint: "" };
        sessions.set(this, state);
        documentState(this.document).session = this;
        state.last = currentSyncState(state);
        state.fingerprint = syncFingerprint(state.last);
        Object.freeze(this);
    }
    static create(options: {
        readonly clientId: string;
        readonly snapshot: SyncSnapshot;
    }): SyncSession {
        if (!(options.snapshot instanceof SyncSnapshot))
            fail("invalid_argument", "SyncSession.create", "expected SyncSnapshot");
        return invoke("SyncSession.create", () => new SyncSession(token, CoreSession.create(checkedString(options.clientId), rawWire(options.snapshot))));
    }
    static restore(checkpoint: SessionCheckpoint): SyncSession { if (!(checkpoint instanceof SessionCheckpoint))
        fail("invalid_argument", "SyncSession.restore", "expected SessionCheckpoint"); return invoke("SyncSession.restore", () => new SyncSession(token, CoreSession.restore(rawWire(checkpoint)))); }
    get state(): SyncState { return currentSyncState(sessionState(this)); }
    get revision(): bigint { return this.state.revision; }
    outbound(): Submission | null { return invoke("SyncSession.outbound", () => { const raw = sessionState(this).raw.outbound(); return raw ? wrapWire(Submission, raw) : null; }); }
    receive(message: ServerMessage): EditResult | null {
        checkWrite(this.document);
        if (!(message instanceof ServerMessage))
            fail("invalid_argument", "SyncSession.receive", "expected ServerMessage");
        try {
            return deliver(this.document, invoke("SyncSession.receive", () => sessionState(this).raw.receive(rawWire(message))));
        }
        finally {
            notifySession(this);
        }
    }
    checkpoint(): SessionCheckpoint { return invoke("SyncSession.checkpoint", () => wrapWire(SessionCheckpoint, sessionState(this).raw.checkpoint())); }
    subscribe(listener: (event: SyncState) => void, options: SubscribeOptions = {}): () => void { checkRead(this.document); return subscribe(sessionState(this).listeners, listener, options); }
    close(): void { this.document.close(); sessionState(this).listeners.clear(); }
}
