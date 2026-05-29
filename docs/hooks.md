# Agent hook ingestion

Traceframe can ingest host hook payloads from stdin and turn them into local
trace events. This is the first integration surface for any agent harness that
emits lifecycle hook payloads, where tools, permission decisions, and failures
happen outside a single wrapped shell command.

The command a wired host hook runs is:

```bash
traceframe hook ingest \
  --source generic \
  --dir .traceframe/runs
```

The hook JSON payload is read from stdin. With `--dir`, traceframe derives a
per-session run id from the payload (`run-<session_id>`, or a deterministic
fallback when the host sends no session id), writes to
`<dir>/<run_id>.traceframe`, and creates that trace on first use. The wired
command therefore never has to know the run id up front, and every event from
the same host session lands in the same trace while different sessions stay in
separate files.

You can still target one explicit file instead of a per-session directory:

```bash
traceframe hook ingest \
  --source generic \
  --file "$TRACEFRAME_FILE" \
  --init-if-missing
```

In `--file` mode, `--init-if-missing` lets the first hook event create the
trace file. The run id is taken from `--run-id` when given, or derived from the
payload otherwise. Pass exactly one of `--file` or `--dir`.

## The source label

`--source` is a free-form label the host chooses. Traceframe stores it verbatim
on every mapped event and never special-cases any harness name, so it stays
agnostic to the tool that produced the hook. The label defaults to `generic`
(an empty or whitespace-only value falls back to `generic`). Use it to tag
events by harness, by policy layer, or by any source you find useful, e.g.
`--source generic`, `--source policy`, or `--source my-harness`.

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

runs_dir="${TRACEFRAME_DIR:-.traceframe/runs}"
source="${TRACEFRAME_SOURCE:-generic}"

traceframe hook ingest \
  --source "$source" \
  --dir "$runs_dir"
```

The host should pipe its hook JSON payload into the script:

```bash
printf '%s' "$HOOK_PAYLOAD_JSON" | ./traceframe-hook.sh
```

## Example payloads

These mirror the shape many agent harnesses send a hook on stdin for a shell
command: `tool_name` is the canonical `"Bash"`, the command is under
`tool_input.command`, and the result is under `tool_response`
(`output` + `exit_code`), alongside host-specific
`turn_id`/`tool_use_id`/`permission_mode` fields. Traceframe only reads the
fields it recognizes and ignores the rest.

Tool call (`PreToolUse`):

```json
{
  "session_id": "00000000-0000-0000-0000-000000000000",
  "turn_id": "11111111-1111-1111-1111-111111111111",
  "transcript_path": "/path/to/transcript.jsonl",
  "cwd": "/path/to/workspace",
  "hook_event_name": "PreToolUse",
  "permission_mode": "default",
  "tool_name": "Bash",
  "tool_input": {
    "command": "cargo test"
  },
  "tool_use_id": "call-aaaa"
}
```

Tool result (`PostToolUse`):

```json
{
  "session_id": "00000000-0000-0000-0000-000000000000",
  "turn_id": "11111111-1111-1111-1111-111111111111",
  "hook_event_name": "PostToolUse",
  "tool_name": "Bash",
  "tool_input": {
    "command": "cargo test"
  },
  "tool_response": {
    "output": "test result: ok. 12 passed",
    "exit_code": 0
  },
  "tool_use_id": "call-aaaa"
}
```

The adapter also accepts a `success`/`stdout`/`duration_ms` result shape from
other hosts; when only an exit code is present it derives success from
`tool_response.exit_code`.

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

## Installing the wiring

`traceframe hook install` writes the host wiring so a harness pipes its hook
payloads into `hook ingest`. It is idempotent and opt-in.

```bash
traceframe hook install
```

- `--file <path>` is the local hooks file to merge into (default
  `.agent/hooks.json`, relative to the current directory).
- `--source <label>` is the free-form source label pinned into the wired
  command (default `generic`).
- `--print` prints the planned wiring without writing anything.
- `--run-id <id>` pins a run id into the generated ingest commands.

Install merges traceframe `PreToolUse` and `PostToolUse` entries into the local
hooks file. The entries are nested under the top-level `hooks` object a host
discovers, and each uses `"matcher": "Bash"`. Most agent harnesses surface
every shell command they run to hooks as the canonical tool name `"Bash"`, and
the matcher is a regex applied to that `tool_name`, so `"Bash"` is what captures
shell tool calls. The generated file looks like:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "traceframe hook ingest --source generic --dir .traceframe/runs" }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "traceframe hook ingest --source generic --dir .traceframe/runs" }
        ]
      }
    ]
  }
}
```

The wired command is `traceframe hook ingest --source generic --dir
.traceframe/runs`, so each host session gets its own per-session trace with no
`--run-id` or `--init-if-missing` to manage. Install preserves existing entries
(including unrelated config under `hooks`) and never duplicates a traceframe
entry, so re-running is safe. It only ever touches the local file you point it
at, never a global one.

### When hooks fire

Whether a hook fires depends on the host. Some harnesses run `PreToolUse` and
`PostToolUse` hooks only during interactive sessions and skip them in a
non-interactive batch mode; others run them in both. Traceframe writes the
standard hook wiring and ingests whatever the host actually sends. When a host
does not run hooks in a given mode, capture traces with the `traceframe run`
wrapper or by piping host payloads into `hook ingest` directly instead.

When a host's settings file is global or otherwise delicate, prefer
`hook install --print` and paste the printed snippet by hand. Traceframe never
writes a file in `--print` mode.

## Auditing a trace against policy

`traceframe policy-check` audits a recorded trace and exits non-zero when it
finds a violation:

```bash
traceframe policy-check --file .traceframe/runs/session.traceframe
```

- `--file <path>` is the trace to audit (required).
- `--allow-open` audits an open (not yet finished) trace, matching `verify`.

It reports two classes of violation:

1. a `permission.decision` deny/denied/block with no later `allow` resolving the
   same capability/command (an unresolved deny);
2. a `tool.call` that maps to a sensitive public capability (`git push` /
   `git.push` in the command or capability) with no prior or simultaneous
   `permission.decision` allow.

Run the simulated integration smoke, which now also exercises `hook install`
and `policy-check`:

```bash
sh scripts/hook-smoke.sh
```

That smoke proves the full local lifecycle: hook ingestion, finish, verify,
summary, inspect, render, ledger rebuild/list/show, hook install (merge +
idempotency), and policy-check (clean and violating traces).
