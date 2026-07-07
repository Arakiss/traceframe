#!/usr/bin/env sh
set -eu

package_args=""
if ! git diff --quiet || ! git diff --cached --quiet; then
  package_args="--allow-dirty"
fi

tmp_file="${TMPDIR:-/tmp}/slod-package-list.$$"
trap 'rm -f "$tmp_file"' EXIT

cargo package $package_args --list > "$tmp_file"

required_files='
CHANGELOG.md
CODE_OF_CONDUCT.md
CONTRIBUTING.md
LICENSE
README.md
SECURITY.md
docs/harness-integration.md
docs/hooks.md
docs/publishing.md
docs/release-signing.md
docs/storage.md
examples/agent-run.slod
examples/harness-recorder.rs
scripts/check-local-agent-files.sh
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

cargo package $package_args >/dev/null
cargo run --example harness-recorder >/dev/null

echo "slod release readiness: ok"
