#!/usr/bin/env sh
set -eu

repo_root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$repo_root"

traceframe_bin="${TRACEFRAME_BIN:-$repo_root/target/debug/traceframe}"
if [ ! -x "$traceframe_bin" ]; then
  cargo build >/dev/null
fi

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/traceframe-host-smoke.XXXXXX")"
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
  if ! grep -Fq "$needle" "$file"; then
    echo "expected '$needle' in $file" >&2
    echo "--- $file ---" >&2
    cat "$file" >&2
    exit 1
  fi
}

success_trace="$runs_dir/smoke-success.traceframe"
failed_trace="$runs_dir/smoke-failed.traceframe"
manual_trace="$runs_dir/smoke-manual.traceframe"
open_trace="$runs_dir/smoke-open.traceframe"

run_tf run --file "$success_trace" --run-id smoke-success --force -- cargo --version \
  >"$tmp_dir/success.out" 2>"$tmp_dir/success.err"
assert_contains "$tmp_dir/success.out" "cargo"
assert_contains "$tmp_dir/success.out" "traceframe finish"
assert_contains "$tmp_dir/success.err" "traceframe exec"
run_tf verify --file "$success_trace" >"$tmp_dir/success.verify"
assert_contains "$tmp_dir/success.verify" "valid trace"

set +e
run_tf run --file "$failed_trace" --run-id smoke-failed --force -- sh -c 'echo expected failure >&2; exit 7' \
  >"$tmp_dir/failed.out" 2>"$tmp_dir/failed.err"
failed_code="$?"
set -e
if [ "$failed_code" -ne 7 ]; then
  echo "expected failed command exit code 7, got $failed_code" >&2
  exit 1
fi
assert_contains "$tmp_dir/failed.out" "traceframe finish"
assert_contains "$tmp_dir/failed.err" "expected failure"
run_tf summary --file "$failed_trace" >"$tmp_dir/failed.summary"
assert_contains "$tmp_dir/failed.summary" "status: failed"
run_tf verify --file "$failed_trace" >"$tmp_dir/failed.verify"
assert_contains "$tmp_dir/failed.verify" "valid trace"

run_tf init --file "$manual_trace" --run-id smoke-manual --force >/dev/null
run_tf record --file "$manual_trace" --kind model.call --payload '{"provider":"openai","model":"gpt-5.5"}' >/dev/null
run_tf record --file "$manual_trace" --kind permission.decision --payload '{"capability":"fs.write:README.md","decision":"deny"}' >/dev/null
run_tf record --file "$manual_trace" --kind error --payload '{"message":"denied by policy"}' >/dev/null
run_tf finish --file "$manual_trace" --status failed --summary "manual denied run" >/dev/null
run_tf inspect --file "$manual_trace" >"$tmp_dir/manual.inspect"
assert_contains "$tmp_dir/manual.inspect" "permission.decision"
assert_contains "$tmp_dir/manual.inspect" "denied by policy"
run_tf render --file "$manual_trace" --html "$reports_dir/manual.html" >/dev/null
test -s "$reports_dir/manual.html"
assert_contains "$reports_dir/manual.html" "traceframe report"

run_tf init --file "$open_trace" --run-id smoke-open --force >/dev/null
run_tf record --file "$open_trace" --kind tool.call --payload '{"tool":"shell","command":"long-running task"}' >/dev/null
run_tf verify --file "$open_trace" --allow-open >"$tmp_dir/open.verify"
assert_contains "$tmp_dir/open.verify" "valid open trace"
run_tf summary --file "$open_trace" >"$tmp_dir/open.summary"
assert_contains "$tmp_dir/open.summary" "status: open"
run_tf render --file "$open_trace" --html "$reports_dir/open.html" >/dev/null
test -s "$reports_dir/open.html"

run_tf ledger rebuild --dir "$runs_dir" --out "$ledger_file" >"$tmp_dir/ledger.rebuild"
assert_contains "$tmp_dir/ledger.rebuild" "entries     4"
run_tf ledger list --file "$ledger_file" >"$tmp_dir/ledger.list"
assert_contains "$tmp_dir/ledger.list" "smoke-success"
assert_contains "$tmp_dir/ledger.list" "smoke-failed"
assert_contains "$tmp_dir/ledger.list" "smoke-open"
run_tf ledger list --file "$ledger_file" --status failed >"$tmp_dir/ledger.failed"
assert_contains "$tmp_dir/ledger.failed" "smoke-failed"
assert_contains "$tmp_dir/ledger.failed" "smoke-manual"
run_tf ledger list --file "$ledger_file" --status open >"$tmp_dir/ledger.open"
assert_contains "$tmp_dir/ledger.open" "smoke-open"
run_tf ledger show --file "$ledger_file" --run-id smoke-manual >"$tmp_dir/ledger.show"
assert_contains "$tmp_dir/ledger.show" "errors: 1"
assert_contains "$tmp_dir/ledger.show" "permission_decisions: 1"

(
  cd "$tmp_dir"
  cargo run --quiet --manifest-path "$repo_root/Cargo.toml" --example harness-recorder \
    >"$tmp_dir/example.out"
)
assert_contains "$tmp_dir/example.out" "run_id: example-harness"
run_tf verify --file "$tmp_dir/.traceframe/runs/example-harness.traceframe" >/dev/null

echo "traceframe host smoke: ok"
