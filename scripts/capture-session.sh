#!/bin/sh
# capture-session.sh — capture a REAL agent session as a trace.
#
# This is a hands-on demo, not a CI test: it needs `slod` on PATH and an
# agent harness that emits PostToolUse hooks. It shows the full loop:
# wire -> run a real agent -> verify -> render, all in a throwaway project.
#
#   1. make a scratch project,
#   2. wire its local hooks file from the `slod hook install` snippet,
#   3. run a small real agent task that uses a shell/Bash tool,
#   4. verify, summarize, and render the per-session trace the hooks produced.
#
# Set AGENT_CMD to the command that launches your agent harness in the scratch
# project. It must read its hook config from the wired hooks file below and run
# the shell commands the prompt asks for. As a generic, harness-free fallback
# this demo can instead record a trace with the `slod run` wrapper.
#
# Each shell tool call fires the PostToolUse hook, which pipes the payload to
# `slod hook ingest --dir .slod/runs`; ingest derives the run id
# from the session id, so one trace file is written per session automatically.
set -eu

command -v slod >/dev/null || { echo "install slod first (cargo install --path .)" >&2; exit 1; }

# Path to the wired hooks file inside the scratch project. Point your agent
# harness at this file (most harnesses read PreToolUse/PostToolUse entries
# nested under a top-level `hooks` object).
hooks_file=".agent/hooks.json"

proj="$(mktemp -d)"
trap 'echo "scratch project left at $proj for inspection"' EXIT
cd "$proj"
git init -q
printf 'line one\nline two\nline three\n' > notes.txt

# Wire the project: write the hook snippet (the print-only installer output),
# pinning an absolute --dir so it resolves wherever the hook runs.
mkdir -p "$(dirname "$hooks_file")"
cat > "$hooks_file" <<JSON
{
  "hooks": {
    "PostToolUse": [
      { "matcher": "Bash", "hooks": [ { "type": "command", "command": "slod hook ingest --source generic --dir $proj/.slod/runs" } ] }
    ]
  }
}
JSON

if [ -n "${AGENT_CMD:-}" ]; then
  echo "running a real agent session in $proj ..."
  echo "agent command: $AGENT_CMD"
  # The agent should read $hooks_file and run a few shell commands (e.g.
  # 'ls -la', 'wc -l notes.txt', 'cat notes.txt'), firing PostToolUse hooks.
  sh -c "$AGENT_CMD" < /dev/null || true
else
  echo "AGENT_CMD not set; recording a generic command trace with 'slod run' instead."
  echo "set AGENT_CMD to wire and capture a real agent harness session."
  slod run --file "$proj/.slod/runs/demo-session.slod" --run-id demo-session -- sh -c 'ls -la; wc -l notes.txt; cat notes.txt' || true
fi

trace="$(ls "$proj"/.slod/runs/*.slod 2>/dev/null | head -1 || true)"
[ -n "$trace" ] || { echo "no trace was produced — is the PostToolUse hook wired (or AGENT_CMD set)?" >&2; exit 1; }

echo ""; echo "captured trace: $trace"
slod verify  --file "$trace" --allow-open
slod summary --file "$trace"
slod render  --file "$trace" --html "$proj/report.html"
echo "open the report: $proj/report.html"
