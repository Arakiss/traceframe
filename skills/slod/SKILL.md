---
name: slod
description: Operate Slod, the local-first trace recorder and inspector for AI agent workflows.
---

# Slod Skill

Use this skill when working in a repository that should leave inspectable local
evidence for AI-agent runs, or when modifying Slod itself.

## Core Model

- Trace files are the source of truth.
- Ledgers, reports, indexes, dashboards, and exports are derived artifacts.
- Slod records what happened; it does not approve, deny, sandbox, or
  execute tools beyond the explicit `run` / `exec` wrapper commands.
- Use a policy layer or host-native permission controls for permission
  decisions, then record those decisions as `permission.decision` events.

## Common Commands

```bash
slod run --run-id local-test -- cargo test
slod verify --file .slod/runs/local-test.slod
slod summary --file .slod/runs/local-test.slod
slod inspect --file .slod/runs/local-test.slod
slod render --file .slod/runs/local-test.slod --html .slod/reports/local-test.html
slod ledger rebuild
slod ledger list
slod ledger list --status failed
```

For host hook payloads, pipe one JSON hook payload into `hook ingest`. Use
`--dir` for per-session traces (the run id is derived from the payload, the
trace is created on first use, so no `--run-id` or `--init-if-missing` is
needed):

```bash
slod hook ingest \
  --source generic \
  --dir .slod/runs
```

`--source` is a free-form label the host chooses (default `generic`). Pass
`--file` instead of `--dir` to append to one explicit trace.

## Rust Harness API

Use `TraceRecorder` when the harness is already Rust:

```rust
use slod::trace::TraceRecorder;

let recorder = TraceRecorder::start(
    ".slod/runs/my-agent-run.slod",
    "my-agent-run",
    true,
)?;

recorder.model_call("openai", "gpt-5.5")?;
recorder.permission_decision("fs.write:README.md", "allow")?;
recorder.tool_call("shell", "cargo test", ["cargo", "test"])?;
recorder.tool_result("shell", "cargo test", true, Some(0), Some(320))?;
recorder.finish("success", Some("harness completed"))?;
```

## Slod Repo Gate

Before claiming a Slod change is complete:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo llvm-cov --workspace --all-targets --fail-under-lines 80
sh scripts/check-release-readiness.sh
sh scripts/host-smoke.sh
sh scripts/hook-smoke.sh
```

Prefer small public commits per verified block:

- behavior/API;
- tests/regressions;
- docs/CI/dogfood;
- review fix.

Do not create artificial commits only to increase GitHub activity.
