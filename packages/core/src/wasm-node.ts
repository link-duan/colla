import { Buffer } from "node:buffer"
import { initSync } from "./internal/colla_wasm.js"
import wasmBase64 from "./internal/wasm_base64.js"

// Node and browser entries share one encoded payload in the published package.
initSync({ module: Buffer.from(wasmBase64, "base64") })
