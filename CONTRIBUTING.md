# Contributing to Traceframe

Traceframe is early, but the project is intentionally strict about evidence.
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

Run the same baseline as CI before opening or merging work:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo llvm-cov --workspace --all-targets --fail-under-lines 80
```

For changes that affect day-to-day usage, also dogfood Traceframe against
itself:

```bash
traceframe run --run-id local-gate -- cargo test
traceframe ledger rebuild
traceframe ledger list
```

## Commit Shape

Use semantic commits with an intent-first subject where possible:

```text
feat: make local traces discoverable before database indexing
fix: support plain relative output paths
docs: document the storage contract
test: cover malformed trace rejection
```

Do not create artificial commits only to inflate activity. Public cadence should
show real maintenance, not noise.

## Pull Request Checklist

- [ ] The change keeps raw trace files as evidence.
- [ ] Tests cover new public behavior.
- [ ] The local gate passed.
- [ ] Documentation changed if CLI, schema, storage, or release behavior changed.
- [ ] Any breaking change is called out explicitly.
