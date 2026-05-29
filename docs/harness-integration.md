# Harness Integration

Traceframe can be used as a CLI or as a small Rust library inside an agent
harness.

Use the CLI when the host is shell-first:

```bash
traceframe run --run-id local-test -- cargo test
traceframe ledger rebuild
traceframe ledger list
```

Use the library API when the harness already runs in Rust and should avoid
shelling out for every event:

```rust
use traceframe::trace::TraceRecorder;

fn run_harness() -> anyhow::Result<()> {
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

    Ok(())
}
```

Use hook ingestion when the harness already emits JSON lifecycle hooks:

```bash
traceframe hook ingest \
  --source generic \
  --dir .traceframe/runs
```

With `--dir`, the run id and trace file are derived per host session from the
payload, so the command needs no `--run-id` or `--init-if-missing`. Pass
`--file` instead to target one explicit trace. `--source` is a free-form label
the host chooses; traceframe stores it verbatim and never names a specific
harness.

The hook payload is read from stdin and mapped to `tool.call`, `tool.result`,
`permission.decision`, or `error`. See [`hooks.md`](hooks.md).

## Boundary

`TraceRecorder` is not an agent runtime. It does not approve, deny, sandbox, or
execute tools. It only writes the evidence artifact that another human or agent
can inspect later.

## Recommended Run Layout

```text
.traceframe/
  runs/
    my-agent-run.traceframe
  ledger.traceframe
  reports/
    my-agent-run.html
```

After a batch of runs:

```bash
traceframe ledger rebuild
traceframe ledger list --status failed
```

## With a policy layer

A policy layer and Traceframe solve adjacent harness problems:

- A policy layer decides whether an observed capability should be allowed.
- Traceframe records what happened around the run.

A harness can record policy decisions as `permission.decision` events so a
failed run contains both the policy outcome and the surrounding tool/model/error
context.
