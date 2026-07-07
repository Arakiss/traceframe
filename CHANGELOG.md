# Changelog

All notable user-facing changes to Traceframe are tracked here.

Traceframe is pre-1.0. Breaking schema, CLI, and output changes may happen
while the public harness contract is still being proven.

From this release onward the changelog is generated from Conventional Commits;
new versions are appended above this entry automatically.

## [0.2.0](https://github.com/Arakiss/traceframe/compare/v0.1.2...v0.2.0) (2026-07-07)


### Features

* agent.guess and plan.deviation event kinds ([9bca0ca](https://github.com/Arakiss/traceframe/commit/9bca0ca7db826a2ffd0467c64bfff65bf97128d1))
* import claude-code session transcripts as closed traces ([9511c8d](https://github.com/Arakiss/traceframe/commit/9511c8dcf681a5bb8ca69e93206261df24a5ac3c))


### Bug fixes

* clamp imported run bounds to min/max transcript timestamps ([d752fd9](https://github.com/Arakiss/traceframe/commit/d752fd9ba99e4bf51a387078e594cd7cb4150113))


### Documentation

* add roadmap, agent working brief, and codex hook wiring guide ([8af5865](https://github.com/Arakiss/traceframe/commit/8af58653fd58d4cb84093075b6512561e4d95eaf))
* **roadmap:** Cursor as a verified capture host in v0.3 ([9597fc3](https://github.com/Arakiss/traceframe/commit/9597fc322c4e96fe4fe6fae8727f6d261a4d60a1))

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
