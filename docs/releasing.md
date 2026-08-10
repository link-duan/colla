# Coordinated release runbook

Colla publishes one version as two artifacts: the Rust `colla` crate and the
`colla-ot` npm package. The registries do not provide a cross-registry
transaction. The release workflow therefore coordinates preflight, publishes
only missing artifacts, verifies both public artifacts, and promotes a draft
GitHub Release only after verification succeeds.

## Preconditions

- The release commit is on `master` and every required CI job is green.
- Cargo, npm and lockfile versions match exactly.
- `CHANGELOG.md` contains the release notes and no longer marks the version as
  Unreleased.
- The target version is absent from both registries for a new release, or an
  existing artifact belongs to the exact same tag during recovery.
- The GitHub `release` environment is configured and its
  `CARGO_REGISTRY_TOKEN` secret is available.
- The npm `colla-ot` package trusts GitHub Actions from repository
  `link-duan/colla`, workflow `release.yml`, and environment `release`.
- npm publishing uses GitHub OIDC trusted publishing. Do not add an npm token
  to the workflow or the `release` environment.

Run the local gates from a clean checkout:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
cargo fmt --all --check
cargo +1.81.0 check -p colla --lib --locked
pnpm install --frozen-lockfile
pnpm check
pnpm test:js
pnpm test:e2e
pnpm release:preflight
```

## Start a release

Create one annotated tag after reviewing the release commit. The tag version
must equal both package manifests.

```sh
git tag -a v0.1.0 -m "Colla Core 0.1.0"
git push origin v0.1.0
```

Pushing the tag starts the Release workflow. The workflow:

1. checks that the tag, Cargo manifest, npm manifest and lockfiles agree;
2. runs all release gates before either registry publish starts;
3. creates or reuses a draft GitHub Release for the tag;
4. inspects both registries and publishes only artifacts that are absent;
5. exchanges the GitHub Actions OIDC identity for npm publishing access;
6. installs the exact public versions from crates.io and npm;
7. records registry checksum/integrity evidence;
8. promotes the GitHub Release only after public verification passes.

Do not move or recreate a release tag after publishing starts.

## Recover a partial publish

If one registry succeeds and the other fails, leave the successful artifact and
the draft GitHub Release unchanged. Fix the credential or registry problem, then
rerun the Release workflow for the same tag using `workflow_dispatch` and the
exact tag name.

The recovery run must:

- resolve the same tag commit and manifest version;
- skip an already-published matching artifact;
- publish only the missing artifact;
- rerun public-registry verification for both artifacts;
- promote the existing draft release only after both verify.

Never change the version, create a replacement tag, overwrite the successful
artifact or claim that the two registry writes are atomic.

## Verify manually

The public verification command rejects aliases such as `latest` and local
paths. It accepts only an exact SemVer:

```sh
pnpm release:verify --version 0.1.0 --output artifacts/release-0.1.0.json
```

Attach the evidence file and workflow URL to the release tracking issue before
closing its milestone.
