# traceframe

Local-first trace recorder and inspector for AI agent workflows.

```text
Agents do not need more autonomy before they have inspectable traces.
```

`traceframe` records what an AI agent actually did: model calls, tool calls,
permission decisions, errors, final state, and the order in which those things
happened. It is a small Rust CLI and JSONL trace format designed for local
harness engineering, not a SaaS dashboard.

## Why

Agent failures are often hard to review after the fact. A transcript is not a
trace. A shell log is not a trace. A permission decision alone does not explain
the full episode around it.

`traceframe` exists so Codex, Claude Code, `gommage`, and local agent workflows
can leave behind a stable artifact that another human or agent can inspect,
verify, summarize, and turn into tests, policies, evals, or workflow changes.

## Where it fits

`gommage` answers:

```text
What is this agent allowed to do?
```

`traceframe` answers:

```text
What did this agent actually do, and why did it fail?
```

Use it beside guardrails, native sandboxing, review loops, and evals. It does
not replace those layers.

## Non-goals

- Not a SaaS product.
- Not a dashboard-first observability platform.
- Not an agent runtime.
- Not a replacement for OpenTelemetry.
- Not a replacement for `gommage`, sandboxing, or native agent permissions.
- Not an eval framework in v0.1.

## Install

From this repository:

```bash
cargo install --path .
```

## Quick start

```bash
traceframe init --file traceframe.jsonl --run-id run-demo
traceframe record --file traceframe.jsonl --kind model.call --payload '{"provider":"openai","model":"gpt"}'
traceframe record --file traceframe.jsonl --kind permission.decision --payload '{"capability":"fs.write:README.md","decision":"allow"}'
traceframe record --file traceframe.jsonl --kind tool.call --payload '{"tool":"shell","command":"cargo test"}'
traceframe record --file traceframe.jsonl --kind tool.result --payload '{"exit_code":0}'
traceframe record --file traceframe.jsonl --kind run.finished --payload '{"status":"success"}'
traceframe verify --file traceframe.jsonl
traceframe summary --file traceframe.jsonl
traceframe inspect --file traceframe.jsonl
traceframe render --file traceframe.jsonl --html traceframe.html
```

## Event model

v0.1 supports one run per trace file. Events are append-only JSONL.

Required event fields:

- `version`
- `run_id`
- `event_id`
- `kind`
- `ts_ms`
- `seq`
- `payload`

Supported event kinds:

- `run.started`
- `model.call`
- `tool.call`
- `tool.result`
- `permission.decision`
- `error`
- `run.finished`

## Harness engineering principles

- Agents are primary operators; humans are reviewers and operators.
- A trace must help explain a real run, not decorate a dashboard.
- A failed run should become a test, policy, eval, or workflow improvement.
- Local JSONL comes before SaaS.
- OpenTelemetry can be an export surface later, not the core contract.

## Status

Early local MVP. The event schema and CLI are intentionally narrow while the
core contract is tested against real agent workflows.
