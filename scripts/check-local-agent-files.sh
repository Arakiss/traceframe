#!/usr/bin/env sh
# Fail if local-only agent instruction or run-state files are tracked.
#
# Slod is a public, harness-agnostic project. Files that belong to a
# private agent setup must never be committed. This is the repository's safety
# net: a fresh clone does not carry a developer's global gitignore, so the
# check lives in-tree.

set -eu

tracked="$(
  git ls-files | awk '
    # Root AGENTS.md is the public repo working brief; local agent state stays blocked.
    $0 == "CLAUDE.md" { print }
    $0 ~ /^\.claude(\/|$)/ { print }
    $0 ~ /^\.codex(\/|$)/ { print }
    $0 ~ /^\.omx(\/|$)/ { print }
    $0 ~ /^\.nahuali(\/|$)/ { print }
    $0 ~ /^\.codex-runs(\/|$)/ { print }
  '
)"

if [ -n "$tracked" ]; then
  echo "local agent files must not be tracked:" >&2
  printf '%s\n' "$tracked" | sed 's/^/- /' >&2
  echo "move private agent instructions to local-only storage or .git/info/exclude" >&2
  exit 1
fi

echo "local agent files not tracked"
