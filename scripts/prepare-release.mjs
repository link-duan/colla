import assert from "node:assert/strict"
import { execFileSync } from "node:child_process"
import { createHash } from "node:crypto"
import { mkdir, readFile, writeFile } from "node:fs/promises"
import { basename, dirname, resolve } from "node:path"

const semverPattern = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/

function argumentsFrom(argv) {
  const values = new Map()
  for (let index = 0; index < argv.length; index += 2) {
    const name = argv[index]
    const value = argv[index + 1]
    assert.ok(name?.startsWith("--") && value, `invalid argument near ${name ?? "end"}`)
    values.set(name.slice(2), value)
  }
  for (const required of ["tag", "crate", "npm", "output"]) {
    assert.ok(values.has(required), `--${required} is required`)
  }
  return Object.fromEntries(values)
}

async function digest(path, algorithm, encoding) {
  const bytes = await readFile(path)
  return createHash(algorithm).update(bytes).digest(encoding)
}

const args = argumentsFrom(process.argv.slice(2))
const workspaceDir = resolve(import.meta.dirname, "..")
const npmPackage = JSON.parse(await readFile(
  resolve(workspaceDir, "packages/core/package.json"),
  "utf8",
))
const version = npmPackage.version
assert.match(version, semverPattern)
assert.equal(args.tag, `v${version}`, "tag and package version do not match")

const commit = execFileSync("git", ["rev-parse", "HEAD"], {
  cwd: workspaceDir,
  encoding: "utf8",
}).trim()
const worktree = execFileSync("git", ["status", "--porcelain"], {
  cwd: workspaceDir,
  encoding: "utf8",
}).trim()
assert.equal(worktree, "", "release candidate must be built from a clean checkout")
const tagType = execFileSync("git", ["cat-file", "-t", `refs/tags/${args.tag}`], {
  cwd: workspaceDir,
  encoding: "utf8",
}).trim()
assert.equal(tagType, "tag", "release tag must be annotated")
const tagCommit = execFileSync("git", ["rev-list", "-n", "1", args.tag], {
  cwd: workspaceDir,
  encoding: "utf8",
}).trim()
assert.equal(tagCommit, commit, "release tag does not point at HEAD")

const cratePath = resolve(args.crate)
const npmPath = resolve(args.npm)
const candidate = {
  version,
  tag: args.tag,
  commit,
  artifacts: {
    crate: {
      file: basename(cratePath),
      sha256: await digest(cratePath, "sha256", "hex"),
    },
    npm: {
      file: basename(npmPath),
      integrity: `sha512-${await digest(npmPath, "sha512", "base64")}`,
    },
  },
}

const outputPath = resolve(args.output)
await mkdir(dirname(outputPath), { recursive: true })
await writeFile(outputPath, `${JSON.stringify(candidate, null, 2)}\n`)
process.stdout.write(`${JSON.stringify(candidate, null, 2)}\n`)
