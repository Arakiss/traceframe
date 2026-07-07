#!/usr/bin/env sh
set -eu

repo_root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$repo_root"

slod_bin="${SLOD_BIN:-$repo_root/target/debug/slod}"
if [ ! -x "$slod_bin" ]; then
  cargo build >/dev/null
fi

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/slod-codex-omx-hook-smoke.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT

runs_dir="$tmp_dir/.slod/runs"
reports_dir="$tmp_dir/.slod/reports"
ledger_file="$tmp_dir/.slod/ledger.slod"
mkdir -p "$runs_dir" "$reports_dir"

run_tf() {
  "$slod_bin" "$@"
}

assert_contains() {
  file="$1"
  needle="$2"
  if ! grep -Fq "$needle" "$file"; then
    echo "expected '$needle' in $file" >&2
    echo "--- $file ---" >&2
    cat "$file" >&2
    exit 1
  fi
}

hook_trace="$runs_dir/codex-omx-hook.slod"
hook_report="$reports_dir/codex-omx-hook.html"

run_tf hook ingest \
  --source codex \
  --run-id codex-omx-hook \
  --init-if-missing \
  --file "$hook_trace" >"$tmp_dir/hook.call" <<'JSON'
{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"cargo test"},"session_id":"codex-hook-session","cwd":"/tmp/workspace"}
JSON
assert_contains "$tmp_dir/hook.call" "tool.call#1"

run_tf hook ingest \
  --source omx \
  --file "$hook_trace" >"$tmp_dir/hook.permission" <<'JSON'
{"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{"command":"edit README.md"},"decision":"allow","reason":"trusted repo workspace","session_id":"omx-hook-session"}
JSON
assert_contains "$tmp_dir/hook.permission" "permission.decision#2"

run_tf hook ingest \
  --source codex \
  --file "$hook_trace" >"$tmp_dir/hook.result" <<'JSON'
{"hook_event_name":"PostToolUse","tool_name":"Bash","tool_response":{"success":true,"exit_code":0,"stdout":"test result: ok","duration_ms":42},"session_id":"codex-hook-session"}
JSON
assert_contains "$tmp_dir/hook.result" "tool.result#3"

run_tf hook ingest \
  --source generic \
  --file "$hook_trace" >"$tmp_dir/hook.error" <<'JSON'
{"hook_event_name":"HookError","tool_name":"Bash","error":{"message":"simulated host hook failure"},"session_id":"generic-hook-session"}
JSON
assert_contains "$tmp_dir/hook.error" "error#4"

run_tf finish \
  --file "$hook_trace" \
  --status failed \
  --summary "codex/omx hook smoke completed with a simulated hook error" \
  >"$tmp_dir/hook.finish"
assert_contains "$tmp_dir/hook.finish" "slod finish"

run_tf verify --file "$hook_trace" >"$tmp_dir/hook.verify"
assert_contains "$tmp_dir/hook.verify" "valid trace"

run_tf summary --file "$hook_trace" >"$tmp_dir/hook.summary"
assert_contains "$tmp_dir/hook.summary" "run_id: codex-omx-hook"
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
assert_contains "$hook_report" "slod report"

run_tf ledger rebuild --dir "$runs_dir" --out "$ledger_file" >"$tmp_dir/ledger.rebuild"
assert_contains "$tmp_dir/ledger.rebuild" "entries     1"
run_tf ledger list --file "$ledger_file" --status failed >"$tmp_dir/ledger.list"
assert_contains "$tmp_dir/ledger.list" "codex-omx-hook"
run_tf ledger show --file "$ledger_file" --run-id codex-omx-hook >"$tmp_dir/ledger.show"
assert_contains "$tmp_dir/ledger.show" "errors: 1"
assert_contains "$tmp_dir/ledger.show" "permission_decisions: 1"

echo "slod codex/omx hook smoke: ok"
