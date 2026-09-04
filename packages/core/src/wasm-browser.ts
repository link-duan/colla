import { initSync } from "./internal/colla_wasm.js"
import wasmBase64 from "./internal/wasm_base64.js"

const binary = atob(wasmBase64)
const len = binary.length
const bytes = new Uint8Array(len)
for (let i = 0; i < len; i++) {
  bytes[i] = binary.charCodeAt(i)
}
initSync({ module: bytes })

