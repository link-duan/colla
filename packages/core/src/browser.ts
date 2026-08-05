import { initSync } from "./internal/colla_wasm.js"
import wasmBase64 from "./internal/wasm_base64.js"

const binary = atob(wasmBase64)
const bytes = Uint8Array.from(binary, character => character.charCodeAt(0))
initSync({ module: bytes })

export * from "./index.js"
