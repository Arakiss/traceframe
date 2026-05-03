# Publishing Traceframe

Traceframe is not published to crates.io yet. This document defines the release
gate so the first publication is deliberate instead of cosmetic.

## Current Channel

- GitHub repository: <https://github.com/Arakiss/traceframe>
- Domain reserved for future site: <https://traceframe.dev>
- Install path today: `cargo install --path .`
- Package name reserved locally: `traceframe`

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

## Local Readiness Commands

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo llvm-cov --workspace --all-targets --fail-under-lines 80
cargo package --list
cargo package
```

## Release Notes Rule

The first published release should explain:

- Traceframe is local-first and file-based.
- The trace file is the source of truth.
- The ledger is rebuildable and derived.
- The crate is pre-1.0 and may break schema/CLI contracts.
- Traceframe complements policy layers such as Gommage; it does not replace
  sandboxing or permission controls.

## Non-Goal

Do not publish just to create activity. A release should be useful to someone
installing the tool outside this checkout.
