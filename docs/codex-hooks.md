# Codex / OMX hook wiring

How to wire a Codex or OMX (oh-my-codex) style harness into slod. The
event mapping, payload tolerance, and `--source` semantics are the generic
ones documented in [hooks.md](hooks.md) — `--source` is a free-form label, so
`codex` and `omx` are conventions, not special cases. This page only covers
the Codex-side wiring pattern.

## Shell hook pattern

Point the host hook at a tiny wrapper; the host pipes its hook JSON payload
into it on stdin:

```bash
#!/usr/bin/env sh
set -eu

slod hook ingest \
  --source "${SLOD_SOURCE:-codex}" \
  --dir "${SLOD_DIR:-.slod/runs}"
```

With `--dir`, slod derives `run-<session_id>` from the payload, so every
event from the same Codex session lands in one trace and parallel sessions
stay separate. For a single explicitly-managed trace, use the
`--file "$SLOD_FILE" --init-if-missing` form instead (see hooks.md).

Label discipline for mixed fleets: `--source codex`, `--source omx`, and
`--source claude-code` keep one operator's two harnesses distinguishable in
the same ledger.

## Current boundary

Slod does not install itself into `~/.codex/hooks.json` (or any host
config) yet. The adapter and smoke test are versioned first, so real hook
installation can be added later as a separate opt-in command with reviewable
behavior.

## Smoke test

`scripts/codex-omx-hook-smoke.sh` simulates the full integration — codex and
omx payloads, tool call, permission decision, tool result — and asserts the
resulting trace verifies, summarizes, and renders:

```bash
sh scripts/codex-omx-hook-smoke.sh
```
