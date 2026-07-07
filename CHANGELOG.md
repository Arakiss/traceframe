# Changelog

All notable user-facing changes to Slod (formerly Traceframe, renamed
2026-07-07) are tracked here. Entries below the rename keep the old name —
they are history, not documentation.

Slod is pre-1.0. Breaking schema, CLI, and output changes may happen
while the public harness contract is still being proven.

From this release onward the changelog is generated from Conventional Commits;
new versions are appended above this entry automatically.

## [0.1.2](https://github.com/Arakiss/traceframe/compare/v0.1.1...v0.1.2) (2026-05-29)


### Bug fixes

* **release:** build cross targets with the toolchain that has them ([0252a12](https://github.com/Arakiss/traceframe/commit/0252a12f891ff7ebcd256f44642fb85a32058b13))
* **release:** trigger binary build from aggregate release output ([a1c0396](https://github.com/Arakiss/traceframe/commit/a1c0396a8b0a622db350b25512a4770089e3702c))

## [0.1.1](https://github.com/Arakiss/traceframe/compare/v0.1.0...v0.1.1) (2026-05-29)


### Documentation

* document release signing and the automated release flow ([bca68dc](https://github.com/Arakiss/traceframe/commit/bca68dc406d75583c1344ca0ad437e0bf9a8b3f1))

## [0.1.0] - 2026-05-29

### Added

- Local append-only trace files for AI agent workflow events.
- CLI commands for `init`, `record`, `finish`, `exec`, `run`, `verify`,
  `summary`, `inspect`, and `render`.
- Hook ingestion with `traceframe hook ingest` for any agent-harness host
  payloads, tagged by a free-form `--source` label.
- Open-trace inspection for interrupted runs.
- Rebuildable local ledger with `traceframe ledger rebuild/list/show`.
- Rust `TraceRecorder` API for harnesses that want to write traces directly.
- Agent-facing `skills/traceframe` operating instructions.
- Release-readiness script, GitHub templates, and CODEOWNERS.
- Host smoke script that dogfoods success, failure, manual, open, render,
  ledger, and Rust-recorder flows.
- Agent hook smoke script that dogfoods host payload ingestion, rendering,
  and ledger indexing.
- CI coverage floor at 80% line coverage.

### Fixed

- Plain relative output paths such as `run.traceframe`, `report.html`, and
  `ledger.traceframe` now work without trying to create an empty parent
  directory.
- `traceframe hook ingest` now derives a per-session run id from the payload, so
  the command wired by `hook install` runs without a pre-set `--run-id`. Added a
  `--dir` mode that writes one trace per host session (`<dir>/<run_id>.traceframe`),
  created on first use; `hook install` now wires `--dir .traceframe/runs` so
  separate sessions no longer collide in a single trace file.
