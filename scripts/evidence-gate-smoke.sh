#!/bin/sh
# evidence-gate-smoke.sh — prove that verify + policy-check work as a CI/pre-push gate.
#
# The gate idea: a run leaves a trace; before a public action (CI merge, git push)
# you require the trace to be (a) structurally valid (`verify`) and (b) free of
# unresolved policy violations (`policy-check`). A clean run passes the gate
# (exit 0); a run with an unresolved permission deny or an unrecorded sensitive
# capability fails it (exit != 0), blocking the action.
#
# This script asserts both directions so the gate is trustworthy.
set -eu

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
slod_bin="${SLOD_BIN:-$repo_root/target/debug/slod}"
if [ ! -x "$slod_bin" ]; then
  (cd "$repo_root" && cargo build >/dev/null)
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

tf() { "$slod_bin" "$@"; }

# --- gate(): the reusable check. verify AND policy-check must pass. ---
gate() {
  trace="$1"
  tf verify --file "$trace" >/dev/null 2>&1 || return 1
  tf policy-check --file "$trace" >/dev/null 2>&1 || return 1
  return 0
}

# 1) CLEAN run: a sensitive capability (git push) that was explicitly allowed,
#    then executed. No unresolved deny. The gate must PASS (exit 0).
clean="$tmp_dir/clean.slod"
tf init --file "$clean" --run-id gate-clean >/dev/null
tf record --file "$clean" --kind permission.decision \
  --payload '{"capability":"git.push:main","decision":"allow"}' >/dev/null
tf record --file "$clean" --kind tool.call \
  --payload '{"tool":"shell","command":"git push origin main"}' >/dev/null
tf finish --file "$clean" --status success >/dev/null

if gate "$clean"; then
  echo "PASS clean run cleared the gate (exit 0)"
else
  echo "FAIL clean run was blocked by the gate" >&2
  exit 1
fi

# 2) DIRTY run: an unresolved permission deny on a public push. The gate must
#    BLOCK it (policy-check exits != 0). We use the shipped example trace.
dirty="$repo_root/examples/agent-run.slod"
if gate "$dirty"; then
  echo "FAIL dirty run cleared the gate but should have been blocked" >&2
  exit 1
else
  echo "PASS dirty run blocked by the gate (policy violation)"
fi

# 3) The dirty run is still structurally valid — proving verify and policy-check
#    are independent concerns (integrity vs policy).
if tf verify --file "$dirty" >/dev/null 2>&1; then
  echo "PASS dirty run is structurally valid (verify passes); only policy blocks it"
else
  echo "FAIL dirty run failed structural verify unexpectedly" >&2
  exit 1
fi

echo "slod evidence-gate smoke: ok"
