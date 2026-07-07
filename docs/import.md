# Importing harness transcripts

Hooks capture the future; the importer captures the past. `slod import`
backfills traces from session transcripts a harness already wrote on disk, so
layer-4 consumers can mine history without waiting for live capture.

```bash
slod import --format claude-code --input session.jsonl
slod ledger rebuild
slod summary --file .slod/runs/run-<session>.slod
```

Supported formats: `claude-code` (a Claude Code session transcript,
newline-delimited JSON, as found under `~/.claude/projects/<project>/`).
A `codex` format is on the roadmap.

## Behavior

- **Target.** With `--dir` (default `.slod/runs`), the trace lands at
  `<dir>/<run_id>.slod`; the run id is `run-<session_id>` derived from
  the transcript, or the input file stem as a fallback (`--run-id` overrides).
  `--file` targets one explicit path instead. An existing non-empty target is
  refused unless `--force`.
- **Timestamps are the transcript's own.** Event `ts_ms` comes from each
  message's timestamp, not from import time, so run duration stays meaningful.
- **Mapping.** The first assistant model (and every model change) becomes a
  `model.call`; `tool_use` blocks become `tool.call` (with the tool's primary
  argument surfaced as `command`); `tool_result` blocks become `tool.result`
  with `success = !is_error` and an error preview on failures; compaction
  summaries are skipped and counted.
- **Skimmability.** Strings embedded in payloads are capped at 400 characters
  with an explicit `…(+N chars)` marker; the original transcript remains the
  full-fidelity source.
- **Closure.** The trace always ends with `run.finished`, `status: imported`,
  carrying accumulated token usage (input/output/cache) — imports never leave
  open traces, so `verify` passes without `--allow-open`.
- **One-pass writes.** The importer writes the whole trace in one pass instead
  of per-event appends; multi-thousand-event sessions import in milliseconds.

## What it does not capture

Permission decisions do not live in Claude Code transcripts (a policy layer
like gommage keeps its own audit log); imported traces therefore contain no
`permission.decision` events. Wire live hooks for those.
