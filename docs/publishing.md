# Publishing Slod

Slod ships GitHub releases automatically and is **not** published to
crates.io yet. This document defines the release flow and the gate so the first
crates.io publication is deliberate instead of cosmetic.

## Current Channel

- GitHub repository: <https://github.com/Arakiss/slod>
- Domain reserved for future site: <https://slod.dev>
- Install path today: `cargo install --path .`
- Tagged releases: signed binaries + SBOM attached to each GitHub release.
- Package name reserved locally: `slod` (not yet claimed on crates.io).

## Release Flow

Releases are driven by [release-please](https://github.com/googleapis/release-please)
from Conventional Commits. The path from merge to artifacts:

1. Commits land on `main` using Conventional Commits (enforced by `commitlint`).
2. release-please keeps a `chore(release): release X.Y.Z` pull request open that
   accumulates the changelog and the version bump.
3. Merging that PR tags the release (`vX.Y.Z`) and creates the GitHub release.
4. The tag triggers `build-binaries`: cross-platform binaries are built, signed
   with Sigstore (keyless), attested, and attached to the release.
5. `release-evidence` generates a CycloneDX SBOM, attaches it, and attests it.

Verification of the resulting artifacts is documented in
[release-signing.md](release-signing.md).

To pause release automation during maintenance, set the repository variable
`SLOD_RELEASE_HOLD=true`; normal CI keeps running.

## First crates.io Publish Gate

Do not publish until all of these are true:

- `cargo package --list` contains only intended public files.
- `cargo package` succeeds without warnings that affect consumers.
- `cargo install --path . --force` works from a clean checkout.
- README quick start works from the packaged crate.
- `CHANGELOG.md` has a concrete version entry.
- GitHub CI is green on the release commit.
- The maintainer has reviewed the package metadata, license, repository,
  homepage, keywords, and categories.

## Manual crates.io Publish

crates.io publication is intentionally **manual** and kept out of the automated
workflow until the name is claimed. When the gate above is satisfied:

```bash
# Confirm the package contents and metadata first.
cargo package --list
cargo publish --dry-run

# Publish (requires a crates.io token; no CI secret is configured for this).
cargo publish
```

Do this from the release tag's commit so the published crate matches the signed
GitHub release.

## Local Readiness Commands

```bash
just check          # the full CI verdict in one recipe
```

Or run the individual steps:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo llvm-cov --workspace --all-targets --fail-under-lines 80
cargo deny check advisories bans licenses sources
cargo package --list
cargo package
```

## Release Notes Rule

The first published release should explain:

- Slod is local-first and file-based.
- The trace file is the source of truth.
- The ledger is rebuildable and derived.
- The crate is pre-1.0 and may break schema/CLI contracts.
- Slod complements policy layers; it does not replace sandboxing or
  permission controls.

## Non-Goal

Do not publish just to create activity. A release should be useful to someone
installing the tool outside this checkout.
