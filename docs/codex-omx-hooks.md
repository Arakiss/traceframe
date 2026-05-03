# Codex and OMX hook ingestion

Traceframe can ingest host hook payloads from stdin and turn them into local
trace events. This is the first integration surface for Codex/OMX-style agent
harnesses where tools, permission decisions, and failures happen outside a
single wrapped shell command.

The command is:

```bash
traceframe hook ingest \
  --source codex \
  --run-id "$TRACEFRAME_RUN_ID" \
  --init-if-missing \
  --file "$TRACEFRAME_FILE"
```

The hook JSON payload is read from stdin. `--init-if-missing` lets the first hook
event create the trace file. Later hook calls can omit `--run-id` and append to
the same file.

## Supported sources

- `codex`
- `omx` / `oh-my-codex`
- `generic` / `host`

The adapter is intentionally tolerant. It looks for common host fields such as
`hook_event_name`, `tool_name`, `tool_input`, `tool_response`, `decision`,
`success`, `exit_code`, and `error`. Unknown host fields are not part of the
public v0.1 contract.

## Event mapping

| Host payload shape | Traceframe event |
| --- | --- |
| `PreToolUse`, `tool_input`, or `tool_name` | `tool.call` |
| `PostToolUse`, `tool_response`, `success`, or `exit_code` | `tool.result` |
| `decision` / `permission_decision` | `permission.decision` |
| `HookError` / `error` | `error` |

Permission decisions take precedence over tool calls because many host systems
make the decision inside a pre-tool hook.

## Shell hook pattern

Use a tiny shell wrapper from the host hook. The exact environment variables
depend on the host, but the pattern is stable:

```bash
#!/usr/bin/env sh
set -eu

trace_file="${TRACEFRAME_FILE:-.traceframe/runs/${TRACEFRAME_RUN_ID}.traceframe}"
source="${TRACEFRAME_SOURCE:-codex}"

traceframe hook ingest \
  --source "$source" \
  --run-id "$TRACEFRAME_RUN_ID" \
  --init-if-missing \
  --file "$trace_file"
```

The host should pipe its hook JSON payload into the script:

```bash
printf '%s' "$HOOK_PAYLOAD_JSON" | ./traceframe-hook.sh
```

## Example payloads

Tool call:

```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {
    "command": "cargo test"
  },
  "session_id": "codex-session"
}
```

Tool result:

```json
{
  "hook_event_name": "PostToolUse",
  "tool_name": "Bash",
  "tool_response": {
    "success": true,
    "exit_code": 0,
    "stdout": "test result: ok",
    "duration_ms": 320
  }
}
```

Permission decision:

```json
{
  "hook_event_name": "PreToolUse",
  "tool_name": "Write",
  "tool_input": {
    "command": "edit README.md"
  },
  "decision": "allow",
  "reason": "trusted repo workspace"
}
```

## Current boundary

Traceframe does not install itself into `~/.codex/hooks.json` yet. The adapter
and smoke tests are versioned first so real hook installation can be added as a
separate opt-in command with reviewable behavior.

Run the simulated integration smoke:

```bash
sh scripts/codex-omx-hook-smoke.sh
```

That smoke proves the full local lifecycle: hook ingestion, finish, verify,
summary, inspect, render, ledger rebuild/list/show.
