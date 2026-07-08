#!/usr/bin/env sh
set -eu

package_args=""
if ! git diff --quiet || ! git diff --cached --quiet; then
  package_args="--allow-dirty"
fi

tmp_file="${TMPDIR:-/tmp}/slod-package-list.$$"
trap 'rm -f "$tmp_file"' EXIT

cargo package $package_args --locked --list > "$tmp_file"

required_files='
CHANGELOG.md
CODE_OF_CONDUCT.md
CONTRIBUTING.md
LICENSE
README.md
SECURITY.md
docs/ci-gate.md
docs/codex-hooks.md
docs/harness-integration.md
docs/hooks.md
docs/import.md
docs/publishing.md
docs/release-signing.md
docs/storage.md
examples/agent-run.slod
examples/agent-session.slod
examples/harness-recorder.rs
scripts/check-local-agent-files.sh
scripts/capture-session.sh
scripts/codex-omx-hook-smoke.sh
scripts/evidence-gate-smoke.sh
scripts/hook-smoke.sh
scripts/check-release-readiness.sh
scripts/host-smoke.sh
skills/slod/SKILL.md
src/hook.rs
src/trace.rs
src/ledger.rs
'

for file in $required_files; do
  if ! grep -qx "$file" "$tmp_file"; then
    echo "missing from cargo package: $file" >&2
    exit 1
  fi
done

cargo package $package_args --locked >/dev/null
cargo run --locked --example harness-recorder >/dev/null

echo "slod release readiness: ok"
