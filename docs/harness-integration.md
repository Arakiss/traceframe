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
  --source codex \
  --run-id "$TRACEFRAME_RUN_ID" \
  --init-if-missing \
  --file "$TRACEFRAME_FILE"
```

The hook payload is read from stdin and mapped to `tool.call`, `tool.result`,
`permission.decision`, or `error`. See
[`codex-omx-hooks.md`](codex-omx-hooks.md).

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

## With Gommage

Gommage and Traceframe solve adjacent harness problems:

- Gommage decides whether an observed capability should be allowed.
- Traceframe records what happened around the run.

A harness can record Gommage decisions as `permission.decision` events so a
failed run contains both the policy outcome and the surrounding tool/model/error
context.
