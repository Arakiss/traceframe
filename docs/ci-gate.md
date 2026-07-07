# Gating on evidence (CI and pre-push)

Slod records what an agent run did. The trace becomes useful as a *gate*:
before a public action — a CI merge, a release, a `git push` — you require the
run's trace to clear two independent checks.

| Check | Question | Command | Exit |
| :-- | :-- | :-- | :-- |
| `verify` | Is the trace structurally sound (ordered, closed, unique ids)? | `slod verify --file <trace>` | `0` ok / non-zero broken |
| `policy-check` | Did the run avoid unresolved policy violations? | `slod policy-check --file <trace>` | `0` clean / non-zero violation |

`policy-check` fails when:

1. a `permission.decision` **deny** has no later **allow** resolving the same
   capability (an unresolved deny — e.g. a public `git.push` that was blocked
   and never cleared); or
2. a `tool.call` runs a **sensitive public capability** (`git push` / `git.push`)
   with no recorded permission allow.

The two are independent: a trace can be structurally valid yet fail policy. The
shipped `examples/agent-run.slod` is exactly that case — it `verify`s fine
but `policy-check` blocks it because `git.push:main` was denied and never resolved.

## In CI (GitHub Actions)

See [`.github/workflows/evidence-gate.yml`](../.github/workflows/evidence-gate.yml).
The gate step is just:

```yaml
- name: Gate on the run trace
  run: |
    slod verify --file "$RUN_TRACE"
    slod policy-check --file "$RUN_TRACE"
```

If either command exits non-zero the job fails, blocking the merge/release.
Point `$RUN_TRACE` at the trace your agent session produced (for example one
written by `slod hook install` wiring, or by `slod run`).

## As a pre-push hook

Block a push when the current session's trace has an unresolved violation:

```sh
#!/bin/sh
# .git/hooks/pre-push — gate pushes on a clean agent trace
trace=".slod/runs/session.slod"
[ -f "$trace" ] || exit 0          # no trace recorded; nothing to gate
slod verify --file "$trace" || { echo "trace failed verify" >&2; exit 1; }
slod policy-check --file "$trace" || { echo "trace failed policy-check" >&2; exit 1; }
```

This is the evidence half of a layered guard: a deterministic git hook can scan
the commit, and this step additionally requires the *run* behind it to be clean.

## Proving the gate

`scripts/evidence-gate-smoke.sh` asserts both directions end to end: a clean run
clears the gate (exit 0) and the example run is blocked by `policy-check` while
still passing `verify`.
