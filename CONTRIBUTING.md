# Contributing to Slod

Slod is early, but the project is intentionally strict about evidence.
Small, verified contributions are preferred over broad rewrites.

## Ground Rules

- Keep trace files as the source of truth.
- Keep ledgers, reports, indexes, and exports rebuildable from trace files.
- Do not add a database, daemon, SaaS dependency, or dashboard-first surface
  without first proving the local trace contract needs it.
- Do not change the event schema silently. Document the impact and update tests.
- Keep CLI output stable enough for agents to parse visually and for humans to
  inspect quickly.
- Prefer focused commits: behavior, tests, docs/CI, and review fixes should be
  separable when the work naturally splits that way.

## Local Gate

Run the same baseline as CI before opening or merging work. With
[`just`](https://just.systems) installed, one recipe reproduces the CI verdict:

```bash
just check
```

That runs format, clippy, tests, the 80% coverage floor, `cargo deny`, the
private-files guard, release readiness, and the smokes — the same gates CI
enforces. The individual steps are also available without `just`:

```bash
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo llvm-cov --workspace --all-targets --locked --fail-under-lines 80
cargo deny check advisories bans licenses sources
sh scripts/check-local-agent-files.sh
sh scripts/check-release-readiness.sh
sh scripts/host-smoke.sh
sh scripts/hook-smoke.sh
sh scripts/codex-omx-hook-smoke.sh
sh scripts/evidence-gate-smoke.sh
```

GitHub CI also runs tests and smokes on Linux and macOS, performs a locked
release build, runs a scheduled RustSec audit, and publishes an OpenSSF
Scorecard signal for the public repository.

For changes that affect day-to-day usage, also dogfood Slod against
itself:

```bash
slod run --run-id local-gate -- cargo test
slod ledger rebuild
slod ledger list
```

## Commit Shape

Slod uses [Conventional Commits](https://www.conventionalcommits.org).
Commit messages drive the changelog and the automated release, so the shape is
enforced by `commitlint` on every pull request.

Format: `type(scope): imperative subject` — subject ≤72 chars, no trailing
period.

Accepted types:

| Type | Use for |
| --- | --- |
| `feat` | A user-facing capability |
| `fix` | A bug fix |
| `perf` | A performance improvement |
| `security` | A security fix or hardening change |
| `refactor` | Internal change with no behavior change |
| `docs` | Documentation only |
| `test` | Tests only |
| `build` | Build system or dependency packaging |
| `ci` | CI configuration |
| `chore` | Maintenance that does not fit above |
| `revert` | Reverting a previous commit |

The scope is optional. When present it must be a domain area — `trace`,
`ledger`, `hook`, `policy`, `render`, `host`, `verify`, `report`,
`lifecycle` — or a cross-cutting scope — `cli`, `lib`, `docs`, `ci`, `deps`,
`release`, `security`, `examples`, `scripts`, `skill`.

```text
feat(ledger): make local traces discoverable before database indexing
fix(hook): support plain relative output paths
docs(storage): document the storage contract
test(verify): cover malformed trace rejection
```

Do not create artificial commits only to inflate activity. Public cadence should
show real maintenance, not noise.

## Pull Request Checklist

- [ ] The change keeps raw trace files as evidence.
- [ ] Tests cover new public behavior.
- [ ] The local gate passed.
- [ ] Documentation changed if CLI, schema, storage, or release behavior changed.
- [ ] Any breaking change is called out explicitly.
