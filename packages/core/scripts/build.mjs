import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises"
import { execFileSync } from "node:child_process"
import { fileURLToPath } from "node:url"
import { dirname, resolve } from "node:path"

const packageDir = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const workspaceDir = resolve(packageDir, "../..")
const generatedDir = resolve(packageDir, "src/internal")
const distInternal = resolve(packageDir, "dist/internal")

await rm(resolve(packageDir, "dist"), { recursive: true, force: true })
await rm(generatedDir, { recursive: true, force: true })

execFileSync(
  "wasm-pack",
  [
    "build",
    resolve(workspaceDir, "crates/colla-wasm"),
    "--target",
    "web",
    "--release",
    "--out-dir",
    generatedDir,
    "--out-name",
    "colla_wasm",
  ],
  { cwd: workspaceDir, stdio: "inherit" },
)

const wasmBase64 = (await readFile(resolve(generatedDir, "colla_wasm_bg.wasm"))).toString("base64")
await writeFile(
  resolve(generatedDir, "wasm_base64.ts"),
  `const wasmBase64 = ${JSON.stringify(wasmBase64)}\nexport default wasmBase64\n`,
)

execFileSync("pnpm", ["exec", "tsc", "-p", resolve(packageDir, "tsconfig.json")], {
  cwd: workspaceDir,
  stdio: "inherit",
})

await mkdir(distInternal, { recursive: true })
for (const file of [
  "colla_wasm.js",
  "colla_wasm.d.ts",
  "colla_wasm_bg.wasm",
  "colla_wasm_bg.wasm.d.ts",
]) {
  await cp(resolve(generatedDir, file), resolve(distInternal, file))
}
