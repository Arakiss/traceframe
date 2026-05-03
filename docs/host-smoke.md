# Host Smoke

`scripts/host-smoke.sh` is the real local dogfood path for Traceframe. It uses
the built CLI against a temporary workspace and checks the behavior an agent
harness actually relies on:

- successful command traces;
- failed command traces that preserve the wrapped command's exit code;
- manual event recording;
- open trace verification;
- HTML rendering;
- ledger rebuild/list/show with `success`, `failed`, and `open` runs;
- the Rust `TraceRecorder` example.

Run it from the repository root:

```bash
sh scripts/host-smoke.sh
```

To test a specific binary:

```bash
TRACEFRAME_BIN=target/debug/traceframe sh scripts/host-smoke.sh
```

CI runs this script after the release-readiness gate so regressions are caught
before a public commit is treated as ready.
