import { readFileSync } from "node:fs"
import { initSync } from "./internal/colla_wasm.js"

const bytes = readFileSync(new URL("./internal/colla_wasm_bg.wasm", import.meta.url))
initSync({ module: bytes })
