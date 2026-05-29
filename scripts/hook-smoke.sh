#!/usr/bin/env sh
set -eu

repo_root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$repo_root"

traceframe_bin="${TRACEFRAME_BIN:-$repo_root/target/debug/traceframe}"
if [ ! -x "$traceframe_bin" ]; then
  cargo build >/dev/null
fi

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/traceframe-hook-smoke.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT

runs_dir="$tmp_dir/.traceframe/runs"
reports_dir="$tmp_dir/.traceframe/reports"
ledger_file="$tmp_dir/.traceframe/ledger.traceframe"
mkdir -p "$runs_dir" "$reports_dir"

run_tf() {
  "$traceframe_bin" "$@"
}

assert_contains() {
  file="$1"
  needle="$2"
  if ! grep -Fq -e "$needle" "$file"; then
    echo "expected '$needle' in $file" >&2
    echo "--- $file ---" >&2
    cat "$file" >&2
    exit 1
  fi
}

# Per-session ingestion via --dir: the wired-command shape. Each call passes
# only --source --dir (the run id is derived from the payload, the trace is
# created on first use). We pin --run-id here only so the rest of the smoke can
# reference a stable file name; the wired end-to-end check below proves the
# zero-flag form works too. The payloads below mimic a generic agent harness.
hook_trace="$runs_dir/hook-smoke.traceframe"
hook_report="$reports_dir/hook-smoke.html"

run_tf hook ingest \
  --source generic \
  --run-id hook-smoke \
  --dir "$runs_dir" >"$tmp_dir/hook.call" <<'JSON'
{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"cargo test"},"session_id":"hook-session-1","cwd":"/tmp/workspace"}
JSON
assert_contains "$tmp_dir/hook.call" "tool.call#1"

run_tf hook ingest \
  --source policy \
  --run-id hook-smoke \
  --dir "$runs_dir" >"$tmp_dir/hook.permission" <<'JSON'
{"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{"command":"edit README.md"},"decision":"allow","reason":"trusted repo workspace","session_id":"hook-session-2"}
JSON
assert_contains "$tmp_dir/hook.permission" "permission.decision#2"

run_tf hook ingest \
  --source generic \
  --run-id hook-smoke \
  --dir "$runs_dir" >"$tmp_dir/hook.result" <<'JSON'
{"hook_event_name":"PostToolUse","tool_name":"Bash","tool_response":{"success":true,"exit_code":0,"stdout":"test result: ok","duration_ms":42},"session_id":"hook-session-1"}
JSON
assert_contains "$tmp_dir/hook.result" "tool.result#3"

run_tf hook ingest \
  --source generic \
  --run-id hook-smoke \
  --dir "$runs_dir" >"$tmp_dir/hook.error" <<'JSON'
{"hook_event_name":"HookError","tool_name":"Bash","error":{"message":"simulated host hook failure"},"session_id":"hook-session-3"}
JSON
assert_contains "$tmp_dir/hook.error" "error#4"

run_tf finish \
  --file "$hook_trace" \
  --status failed \
  --summary "hook smoke completed with a simulated hook error" \
  >"$tmp_dir/hook.finish"
assert_contains "$tmp_dir/hook.finish" "traceframe finish"

run_tf verify --file "$hook_trace" >"$tmp_dir/hook.verify"
assert_contains "$tmp_dir/hook.verify" "valid trace"

run_tf summary --file "$hook_trace" >"$tmp_dir/hook.summary"
assert_contains "$tmp_dir/hook.summary" "run_id: hook-smoke"
assert_contains "$tmp_dir/hook.summary" "tool_calls: 1"
assert_contains "$tmp_dir/hook.summary" "tool_results: 1"
assert_contains "$tmp_dir/hook.summary" "permission_decisions: 1"
assert_contains "$tmp_dir/hook.summary" "errors: 1"

run_tf inspect --file "$hook_trace" >"$tmp_dir/hook.inspect"
assert_contains "$tmp_dir/hook.inspect" "tool.call"
assert_contains "$tmp_dir/hook.inspect" "permission.decision"
assert_contains "$tmp_dir/hook.inspect" "tool.result"
assert_contains "$tmp_dir/hook.inspect" "simulated host hook failure"

run_tf render --file "$hook_trace" --html "$hook_report" >/dev/null
test -s "$hook_report"
assert_contains "$hook_report" "traceframe report"

run_tf ledger rebuild --dir "$runs_dir" --out "$ledger_file" >"$tmp_dir/ledger.rebuild"
assert_contains "$tmp_dir/ledger.rebuild" "entries     1"
run_tf ledger list --file "$ledger_file" --status failed >"$tmp_dir/ledger.list"
assert_contains "$tmp_dir/ledger.list" "hook-smoke"
run_tf ledger show --file "$ledger_file" --run-id hook-smoke >"$tmp_dir/ledger.show"
assert_contains "$tmp_dir/ledger.show" "errors: 1"
assert_contains "$tmp_dir/ledger.show" "permission_decisions: 1"

# hook install: wires a local hooks file inside the temp workspace only.
# Never touches any global host settings file.
hooks_file="$tmp_dir/.agent/hooks.json"
(
  cd "$tmp_dir"
  "$traceframe_bin" hook install --print >"$tmp_dir/install.print"
)
assert_contains "$tmp_dir/install.print" "traceframe hook ingest"
assert_contains "$tmp_dir/install.print" "--dir .traceframe/runs"
assert_contains "$tmp_dir/install.print" "PreToolUse"
test ! -f "$hooks_file"

# Seed a foreign existing entry, then install twice to prove merge + idempotency.
# The entry sits under the top-level `hooks` object a host actually discovers.
mkdir -p "$tmp_dir/.agent"
printf '%s\n' '{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"existing-tool"}]}]}}' >"$hooks_file"
(
  cd "$tmp_dir"
  "$traceframe_bin" hook install >"$tmp_dir/install.first"
)
assert_contains "$tmp_dir/install.first" "wired"
assert_contains "$hooks_file" "existing-tool"
assert_contains "$hooks_file" "traceframe hook ingest"
# Entries must be nested under "hooks" so a host loads them.
assert_contains "$tmp_dir/install.print" "\"hooks\""
(
  cd "$tmp_dir"
  "$traceframe_bin" hook install >"$tmp_dir/install.second"
)
assert_contains "$tmp_dir/install.second" "already wired"
ingest_count="$(grep -c "traceframe hook ingest" "$hooks_file")"
if [ "$ingest_count" -ne 2 ]; then
  echo "expected exactly 2 traceframe hook entries after idempotent install, got $ingest_count" >&2
  cat "$hooks_file" >&2
  exit 1
fi

# End-to-end: extract the EXACT command wired into the hooks file and run it
# with a payload on stdin, proving the wired command works at runtime (the bug
# this design fixes was a wired command that failed for lack of --run-id).
wired_command="$(
  "$traceframe_bin" hook install --print 2>/dev/null \
    | sed -n 's/.*"command": "\(traceframe hook ingest[^"]*\)".*/\1/p' \
    | head -n1
)"
if [ -z "$wired_command" ]; then
  echo "could not extract wired ingest command from install --print" >&2
  exit 1
fi
wired_args="${wired_command#traceframe }"
wired_workspace="$tmp_dir/wired"
mkdir -p "$wired_workspace"
(
  cd "$wired_workspace"
  # shellcheck disable=SC2086
  printf '%s' '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"cargo test"},"session_id":"wired-session"}' \
    | "$traceframe_bin" $wired_args >"$tmp_dir/wired.out"
)
assert_contains "$tmp_dir/wired.out" "tool.call#1"
wired_trace="$wired_workspace/.traceframe/runs/run-wired-session.traceframe"
test -f "$wired_trace" || { echo "wired command did not create $wired_trace" >&2; exit 1; }
run_tf verify --allow-open --file "$wired_trace" >"$tmp_dir/wired.verify"
assert_contains "$tmp_dir/wired.verify" "valid open trace"

# Realistic host payload shape (sanitized): a harness surfaces shell commands to
# hooks as the canonical tool "Bash" with the command under tool_input.command
# and the result under tool_response (output + exit_code). Prove the wired
# adapter maps a genuine PreToolUse/PostToolUse pair into tool.call + tool.result.
real_runs="$tmp_dir/real/.traceframe/runs"
mkdir -p "$tmp_dir/real"
printf '%s' '{"session_id":"00000000-0000-0000-0000-000000000000","turn_id":"11111111-1111-1111-1111-111111111111","cwd":"/tmp/ws","hook_event_name":"PreToolUse","permission_mode":"default","tool_name":"Bash","tool_input":{"command":"cargo test"},"tool_use_id":"call-aaaa"}' \
  | run_tf hook ingest --source generic --dir "$real_runs" >"$tmp_dir/real.pre"
assert_contains "$tmp_dir/real.pre" "tool.call#1"
printf '%s' '{"session_id":"00000000-0000-0000-0000-000000000000","turn_id":"11111111-1111-1111-1111-111111111111","cwd":"/tmp/ws","hook_event_name":"PostToolUse","permission_mode":"default","tool_name":"Bash","tool_input":{"command":"cargo test"},"tool_response":{"output":"test result: ok. 12 passed","exit_code":0},"tool_use_id":"call-aaaa"}' \
  | run_tf hook ingest --source generic --dir "$real_runs" >"$tmp_dir/real.post"
assert_contains "$tmp_dir/real.post" "tool.result#2"
real_trace="$real_runs/run-00000000-0000-0000-0000-000000000000.traceframe"
test -f "$real_trace" || { echo "real payload did not create $real_trace" >&2; exit 1; }
assert_contains "$real_trace" '"tool":"Bash"'
assert_contains "$real_trace" '"command":"cargo test"'
assert_contains "$real_trace" "test result: ok. 12 passed"
assert_contains "$real_trace" '"exit_code":0'

# hook install --print emits a snippet a host operator can paste by hand.
(
  cd "$tmp_dir"
  "$traceframe_bin" hook install --print >"$tmp_dir/install.snippet"
)
assert_contains "$tmp_dir/install.snippet" "paste this into the host"
assert_contains "$tmp_dir/install.snippet" "source generic"

# policy-check: the hook trace allowed a Write but the run failed via a host
# error, with no deny and no git push, so it passes the audit cleanly.
run_tf policy-check --file "$hook_trace" >"$tmp_dir/policy.clean"
assert_contains "$tmp_dir/policy.clean" "result      clean"

# policy-check failure: a permission deny with no later allow.
deny_trace="$runs_dir/policy-deny.traceframe"
run_tf init --file "$deny_trace" --run-id policy-deny --force >/dev/null
run_tf record --file "$deny_trace" --kind permission.decision --payload '{"capability":"fs.write:secrets","decision":"deny"}' >/dev/null
run_tf finish --file "$deny_trace" --status failed >/dev/null
set +e
run_tf policy-check --file "$deny_trace" >"$tmp_dir/policy.deny" 2>&1
deny_code="$?"
set -e
if [ "$deny_code" -eq 0 ]; then
  echo "expected policy-check to fail on unresolved deny" >&2
  exit 1
fi
assert_contains "$tmp_dir/policy.deny" "unresolved deny"

# policy-check failure: a git push tool.call with no prior allow.
push_trace="$runs_dir/policy-push.traceframe"
run_tf init --file "$push_trace" --run-id policy-push --force >/dev/null
run_tf record --file "$push_trace" --kind tool.call --payload '{"tool":"shell","command":"git push --force origin main"}' >/dev/null
run_tf finish --file "$push_trace" --status success >/dev/null
set +e
run_tf policy-check --file "$push_trace" >"$tmp_dir/policy.push" 2>&1
push_code="$?"
set -e
if [ "$push_code" -eq 0 ]; then
  echo "expected policy-check to fail on unauthorized git push" >&2
  exit 1
fi
assert_contains "$tmp_dir/policy.push" "sensitive tool.call"

echo "traceframe hook smoke: ok"
