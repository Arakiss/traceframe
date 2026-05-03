# Changelog

All notable user-facing changes to Traceframe are tracked here.

Traceframe is pre-1.0. Breaking schema, CLI, and output changes may happen
while the public harness contract is still being proven.

## Unreleased

### Added

- Local append-only trace files for AI agent workflow events.
- CLI commands for `init`, `record`, `finish`, `exec`, `run`, `verify`,
  `summary`, `inspect`, and `render`.
- Hook ingestion with `traceframe hook ingest` for Codex/OMX-style host
  payloads.
- Open-trace inspection for interrupted runs.
- Rebuildable local ledger with `traceframe ledger rebuild/list/show`.
- Rust `TraceRecorder` API for harnesses that want to write traces directly.
- Agent-facing `skills/traceframe` operating instructions.
- Release-readiness script, GitHub templates, and CODEOWNERS.
- Host smoke script that dogfoods success, failure, manual, open, render,
  ledger, and Rust-recorder flows.
- Codex/OMX hook smoke script that dogfoods host payload ingestion, rendering,
  and ledger indexing.
- CI coverage floor at 80% line coverage.

### Fixed

- Plain relative output paths such as `run.traceframe`, `report.html`, and
  `ledger.traceframe` now work without trying to create an empty parent
  directory.
