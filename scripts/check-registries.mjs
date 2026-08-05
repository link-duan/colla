import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import { resolve } from "node:path"

const workspaceDir = resolve(import.meta.dirname, "..")
const npmPackage = JSON.parse(
  await readFile(resolve(workspaceDir, "packages/core/package.json"), "utf8"),
)

const headers = { "User-Agent": "colla-release-preflight/0.1" }
const crateResponse = await fetch(
  `https://crates.io/api/v1/crates/colla/${encodeURIComponent(npmPackage.version)}`,
  { headers },
)
assert.ok(
  crateResponse.status === 200 || crateResponse.status === 404,
  `crates.io preflight failed with HTTP ${crateResponse.status}`,
)
assert.equal(
  crateResponse.status,
  404,
  `colla ${npmPackage.version} already exists on crates.io`,
)

const npmResponse = await fetch(
  `https://registry.npmjs.org/${npmPackage.name.replace("/", "%2F")}`,
  { headers },
)
assert.ok(
  npmResponse.status === 200 || npmResponse.status === 404,
  `npm registry preflight failed with HTTP ${npmResponse.status}`,
)
if (npmResponse.status === 200) {
  const metadata = await npmResponse.json()
  assert.equal(
    Object.hasOwn(metadata.versions ?? {}, npmPackage.version),
    false,
    `${npmPackage.name} ${npmPackage.version} already exists on npm`,
  )
}

console.log(`registry version ${npmPackage.version} is available for both artifacts`)
