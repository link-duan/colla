import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import { resolve } from "node:path"

const workspaceDir = resolve(import.meta.dirname, "..")
const cargoManifest = await readFile(resolve(workspaceDir, "Cargo.toml"), "utf8")
const cargoLock = await readFile(resolve(workspaceDir, "Cargo.lock"), "utf8")
const npmPackage = JSON.parse(
  await readFile(resolve(workspaceDir, "packages/core/package.json"), "utf8"),
)

const workspaceVersion = cargoManifest.match(
  /\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m,
)?.[1]
assert.ok(workspaceVersion, "Cargo workspace version is missing")
assert.match(
  workspaceVersion,
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/,
  "Cargo workspace version is not valid SemVer",
)
assert.equal(
  npmPackage.version,
  workspaceVersion,
  `Version mismatch: colla=${workspaceVersion}, @colla/core=${npmPackage.version}`,
)

for (const crate of ["colla", "colla-wasm"]) {
  const entry = cargoLock.match(
    new RegExp(`\\[\\[package\\]\\]\\nname = "${crate}"\\nversion = "([^"]+)"`),
  )
  assert.equal(entry?.[1], workspaceVersion, `${crate} Cargo.lock version is not synchronized`)
}

console.log(`release version ${workspaceVersion} is synchronized`)
