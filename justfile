# Slod task runner (just).
#
# Install: https://just.systems
#
# Recipes mirror the CI jobs in .github/workflows/ci.yml and the local-gate
# section of CONTRIBUTING.md. Running `just check` locally should produce the
# same verdict as CI on a clean PR.

set shell := ["bash", "-euo", "pipefail", "-c"]

# Default recipe — list what's available.
_default:
    @just --list --unsorted

# Run the same checks CI runs on every PR.
check: fmt clippy test coverage deny private-files release-readiness smoke
    @echo "--- local check: ok ---"

# Check formatting without modifying files.
fmt:
    cargo fmt --all --check

# Apply formatting fixes in place.
fmt-fix:
    cargo fmt --all

# Clippy with -D warnings across all targets.
clippy:
    cargo clippy --all-targets -- -D warnings

# Test suite.
test:
    cargo test

# Line coverage with an 80% floor (matches CI).
coverage:
    cargo llvm-cov --workspace --all-targets --fail-under-lines 80

# cargo-deny: advisories, bans, licenses, sources.
deny:
    cargo deny check advisories bans licenses sources

# Ensure local-only agent instruction and run-state files are not tracked.
private-files:
    sh scripts/check-local-agent-files.sh

# Assert the crate packages cleanly with every required file.
release-readiness:
    sh scripts/check-release-readiness.sh

# End-to-end smokes: host flows, hook ingestion, and the evidence gate.
smoke:
    sh scripts/host-smoke.sh
    sh scripts/hook-smoke.sh
    sh scripts/evidence-gate-smoke.sh

# Release-profile build of the binary.
release-build:
    cargo build --release

# Remove all build artefacts.
clean:
    cargo clean
