import assert from "node:assert/strict"
import { execFileSync } from "node:child_process"
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { dirname, join, resolve } from "node:path"

const workspaceDir = resolve(import.meta.dirname, "..")
const semverPattern = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/

function readArguments(argv) {
  let version
  let output
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index]
    if (argument === "--version") {
      version = argv[index + 1]
      index += 1
    } else if (argument === "--output") {
      output = argv[index + 1]
      index += 1
    } else {
      throw new Error(`unknown argument: ${argument}`)
    }
  }
  assert.match(version ?? "", semverPattern, "--version must be an exact SemVer")
  return { version, output }
}

async function fetchJson(url) {
  const response = await fetch(url, {
    headers: { "User-Agent": "colla-release-verifier/0.1" },
  })
  assert.equal(response.status, 200, `${url} returned HTTP ${response.status}`)
  return response.json()
}

function runRustConsumer(version, fixtureDir) {
  const rustDir = join(fixtureDir, "rust")
  return mkdir(join(rustDir, "src"), { recursive: true }).then(async () => {
    await writeFile(join(rustDir, "Cargo.toml"), `[package]
name = "colla-release-consumer"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
colla = "=${version}"

[workspace]
`)
    await writeFile(join(rustDir, "src/main.rs"), `use colla::{apply, path, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = Value::text("Draft");
    let mut builder = base.change();
    builder.text_insert(&path!(), 5, " v2")?;
    let change = builder.build();
    let next = apply(&base, &change)?;
    assert_eq!(next, Value::text("Draft v2"));
    assert_eq!(Value::decode(&next.encode())?, next);
    assert_eq!(colla::Change::decode(&change.encode())?, change);
    Ok(())
}
`)
    execFileSync("cargo", ["generate-lockfile"], { cwd: rustDir, stdio: "inherit" })
    execFileSync("cargo", ["run", "--locked", "--quiet"], {
      cwd: rustDir,
      stdio: "inherit",
    })
    const lock = await readFile(join(rustDir, "Cargo.lock"), "utf8")
    assert.match(
      lock,
      new RegExp(`name = "colla"\\nversion = "${version.replaceAll(".", "\\.")}"\\nsource = "registry\\+`),
      "Rust consumer did not resolve colla from a registry",
    )
  })
}

function runJavaScriptConsumers(version) {
  const environment = {
    ...process.env,
    COLLA_PACKAGE_SPEC: `@colla/core@${version}`,
    COLLA_EXPECTED_PACKAGE_VERSION: version,
  }
  for (const test of ["node-tracer.mjs", "bundlers.mjs"]) {
    execFileSync(process.execPath, [resolve(workspaceDir, "packages/core/tests", test)], {
      cwd: workspaceDir,
      env: environment,
      stdio: "inherit",
    })
  }
  if (process.env.COLLA_RUN_BROWSER === "1") {
    execFileSync("pnpm", ["--filter", "@colla/core", "test:e2e"], {
      cwd: workspaceDir,
      env: environment,
      stdio: "inherit",
    })
  }
}

const { version, output } = readArguments(process.argv.slice(2))
const fixtureDir = await mkdtemp(join(tmpdir(), "colla-release-verifier-"))

try {
  const [crateMetadata, npmMetadata] = await Promise.all([
    fetchJson(`https://crates.io/api/v1/crates/colla/${encodeURIComponent(version)}`),
    fetchJson(`https://registry.npmjs.org/@colla%2Fcore/${encodeURIComponent(version)}`),
  ])
  assert.equal(crateMetadata.version.num, version)
  assert.equal(npmMetadata.version, version)
  assert.ok(crateMetadata.version.checksum, "crates.io checksum is missing")
  assert.ok(npmMetadata.dist?.integrity, "npm integrity is missing")

  await runRustConsumer(version, fixtureDir)
  runJavaScriptConsumers(version)

  const evidence = {
    version,
    verifiedAt: new Date().toISOString(),
    environment: {
      node: process.version,
      platform: process.platform,
      arch: process.arch,
    },
    cratesIo: {
      version: crateMetadata.version.num,
      checksum: crateMetadata.version.checksum,
      crate: crateMetadata.version.crate,
    },
    npm: {
      name: npmMetadata.name,
      version: npmMetadata.version,
      integrity: npmMetadata.dist.integrity,
      tarball: npmMetadata.dist.tarball,
    },
  }
  const serialized = `${JSON.stringify(evidence, null, 2)}\n`
  if (output !== undefined) {
    const outputPath = resolve(output)
    await mkdir(dirname(outputPath), { recursive: true })
    await writeFile(outputPath, serialized)
  }
  process.stdout.write(serialized)
} finally {
  await rm(fixtureDir, { recursive: true, force: true })
}
