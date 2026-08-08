import assert from "node:assert/strict"
import { appendFile, mkdir, readFile, writeFile } from "node:fs/promises"
import { dirname, resolve } from "node:path"

function argumentsFrom(argv) {
  const result = { attempts: 1, require: "none" }
  for (let index = 0; index < argv.length; index += 2) {
    const name = argv[index]
    const value = argv[index + 1]
    assert.ok(name?.startsWith("--") && value, `invalid argument near ${name ?? "end"}`)
    result[name.slice(2)] = value
  }
  assert.ok(result.manifest, "--manifest is required")
  assert.ok(["none", "crate", "npm", "all"].includes(result.require))
  result.attempts = Number.parseInt(result.attempts, 10)
  assert.ok(Number.isSafeInteger(result.attempts) && result.attempts > 0)
  return result
}

async function registryState(candidate) {
  const headers = { "User-Agent": "colla-release-state/0.1" }
  const [crateResponse, npmResponse] = await Promise.all([
    fetch(`https://crates.io/api/v1/crates/colla/${encodeURIComponent(candidate.version)}`, {
      headers,
    }),
    fetch(`https://registry.npmjs.org/colla-ot/${encodeURIComponent(candidate.version)}`, {
      headers,
    }),
  ])
  assert.ok([200, 404].includes(crateResponse.status),
    `crates.io returned HTTP ${crateResponse.status}`)
  assert.ok([200, 404].includes(npmResponse.status),
    `npm returned HTTP ${npmResponse.status}`)

  const crateMetadata = crateResponse.status === 200 ? await crateResponse.json() : undefined
  const npmMetadata = npmResponse.status === 200 ? await npmResponse.json() : undefined
  if (crateMetadata !== undefined) {
    assert.equal(
      crateMetadata.version.checksum,
      candidate.artifacts.crate.sha256,
      "published crate checksum does not match this release tag",
    )
  }
  if (npmMetadata !== undefined) {
    assert.equal(
      npmMetadata.dist?.integrity,
      candidate.artifacts.npm.integrity,
      "published npm integrity does not match this release tag",
    )
  }
  return {
    version: candidate.version,
    tag: candidate.tag,
    commit: candidate.commit,
    crate: {
      published: crateMetadata !== undefined,
      checksum: crateMetadata?.version.checksum,
    },
    npm: {
      published: npmMetadata !== undefined,
      integrity: npmMetadata?.dist?.integrity,
      tarball: npmMetadata?.dist?.tarball,
    },
  }
}

function requirementMet(state, requirement) {
  if (requirement === "none") return true
  if (requirement === "crate") return state.crate.published
  if (requirement === "npm") return state.npm.published
  return state.crate.published && state.npm.published
}

const args = argumentsFrom(process.argv.slice(2))
const candidate = JSON.parse(await readFile(resolve(args.manifest), "utf8"))
let state
for (let attempt = 1; attempt <= args.attempts; attempt += 1) {
  state = await registryState(candidate)
  if (requirementMet(state, args.require)) break
  if (attempt < args.attempts) {
    await new Promise(resolveDelay => setTimeout(resolveDelay, 5000))
  }
}
assert.ok(requirementMet(state, args.require),
  `registry requirement '${args.require}' was not met after ${args.attempts} attempts`)

if (args.output !== undefined) {
  const outputPath = resolve(args.output)
  await mkdir(dirname(outputPath), { recursive: true })
  await writeFile(outputPath, `${JSON.stringify(state, null, 2)}\n`)
}
if (process.env.GITHUB_OUTPUT !== undefined) {
  await appendFile(process.env.GITHUB_OUTPUT,
    `crate_published=${state.crate.published}\nnpm_published=${state.npm.published}\n`)
}
process.stdout.write(`${JSON.stringify(state, null, 2)}\n`)
