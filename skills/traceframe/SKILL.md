---
name: traceframe
description: Operate Traceframe, the local-first trace recorder and inspector for AI agent workflows.
---

# Traceframe Skill

Use this skill when working in a repository that should leave inspectable local
evidence for AI-agent runs, or when modifying Traceframe itself.

## Core Model

- Trace files are the source of truth.
- Ledgers, reports, indexes, dashboards, and exports are derived artifacts.
- Traceframe records what happened; it does not approve, deny, sandbox, or
  execute tools beyond the explicit `run` / `exec` wrapper commands.
- Use Gommage or host-native policy layers for permission decisions, then record
  those decisions as `permission.decision` events.

## Common Commands

```bash
traceframe run --run-id local-test -- cargo test
traceframe verify --file .traceframe/runs/local-test.traceframe
traceframe summary --file .traceframe/runs/local-test.traceframe
traceframe inspect --file .traceframe/runs/local-test.traceframe
traceframe render --file .traceframe/runs/local-test.traceframe --html .traceframe/reports/local-test.html
traceframe ledger rebuild
traceframe ledger list
traceframe ledger list --status failed
```

## Rust Harness API

Use `TraceRecorder` when the harness is already Rust:

```rust
use traceframe::trace::TraceRecorder;

let recorder = TraceRecorder::start(
    ".traceframe/runs/my-agent-run.traceframe",
    "my-agent-run",
    true,
)?;

recorder.model_call("openai", "gpt-5.5")?;
recorder.permission_decision("fs.write:README.md", "allow")?;
recorder.tool_call("shell", "cargo test", ["cargo", "test"])?;
recorder.tool_result("shell", "cargo test", true, Some(0), Some(320))?;
recorder.finish("success", Some("harness completed"))?;
```

## Traceframe Repo Gate

Before claiming a Traceframe change is complete:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo llvm-cov --workspace --all-targets --fail-under-lines 80
sh scripts/check-release-readiness.sh
```

Prefer small public commits per verified block:

- behavior/API;
- tests/regressions;
- docs/CI/dogfood;
- review fix.

Do not create artificial commits only to increase GitHub activity.
