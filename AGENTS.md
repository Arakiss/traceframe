# Working brief for agents

Context an agent needs before touching this repo. Humans: read README.md first.

## What this is

A local-first Rust crate + CLI that records AI agent runs as append-only,
verifiable trace files. It owns **layer 3** (trace capture) of the five-layer
harness model described in README «Where it fits» — and nothing else. Layer 4
consumers (trace miners, reviewers, eval harnesses) read what this produces;
the concrete direction is in ROADMAP.md, which is the source of truth for what
to build next.

## Engineering discipline (enforced, not aspirational)

- **Conventional commits** — commitlint runs in CI (`.commitlintrc.yaml`);
  releases are cut by release-please from commit messages. A wrong prefix
  breaks the release train.
- **Toolchain** pinned in `rust-toolchain.toml`. Run `just ci` (see `justfile`)
  before declaring work done — it mirrors CI (fmt, clippy, test, deny).
- **Tests are per-domain** (`tests/hook.rs`, `tests/ledger.rs`,
  `tests/lifecycle.rs`, `tests/policy.rs`, `tests/render.rs`,
  `tests/verify.rs`). New behavior lands with a test in the matching file;
  smoke scripts live in `scripts/` (`hook-smoke.sh`, `host-smoke.sh`).
- **Schema changes** must update `docs/storage.md` in the same commit. The
  schema is pre-v1: breaking changes are allowed but always documented.
- **No network dependencies** in core, no SaaS assumptions. `deny.toml` gates
  licenses and advisories.

## Hard constraints

- Keep it narrow. If a feature analyzes, scores, or proposes, it belongs to a
  layer-4 consumer, not here. When in doubt, re-read ROADMAP «Non-goals».
- Trace files are append-only and must remain recoverable after partial
  writes; `slod verify` must keep passing on files written by any prior
  released version.
- The hook adapter stays harness-agnostic: `--source` is a free-form label,
  never a switch that special-cases a vendor.

## Current state (2026-07-04)

- v0.1.2 published; CI green; full test suite passes on rustc 1.96.
- Working today: `run`/`exec`/`record`/`init`/`finish`, `hook ingest`
  (per-session `--dir` mode), `ledger rebuild|list`, `verify`, `summary`,
  `inspect`, `render --html`, Rust `TraceRecorder` API.
- Branch `archive/local-2026-05-03` preserves the pre-release local lineage
  (historical reference only; the squashed public history supersedes it).
- Next up: ROADMAP v0.2 — the transcript importer is the highest-value item;
  a layer-4 consumer prototype already exists and is waiting on it.
