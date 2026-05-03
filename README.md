<p align="center">
  <img src="assets/banner.svg?v=20260503-pipeline" alt="traceframe - inspectable traces for AI agent workflows" width="100%" />
</p>
<p align="center"><sub><em>The frame is the harness. The trace is what lets the next agent understand the run.</em></sub></p>

<p align="center">
  <a href="https://github.com/Arakiss/traceframe/actions/workflows/ci.yml"><img src="https://github.com/Arakiss/traceframe/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-yellow.svg" alt="License: MIT"></a>
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/rust-1.94%2B-orange.svg" alt="Rust 1.94+"></a>
  <a href="examples/agent-run.traceframe"><img src="https://img.shields.io/badge/trace-local%20file-blue.svg" alt="Trace local file"></a>
  <a href="README.md"><img src="https://img.shields.io/badge/status-local%20MVP-blue.svg" alt="Status: local MVP"></a>
</p>

# traceframe

> _Agents do not need more autonomy before they have inspectable traces._

**A local-first Rust library and CLI for AI agent workflow traces.**

> **Development status: local MVP.** Traceframe is public, installable from source, and verified by CI, but the trace schema and CLI are still intentionally narrow. Expect breaking schema/CLI changes while the project is tested against real agent workflows. Use it first for local harness inspection, examples, and failure analysis.

Traceframe records what an AI agent actually did: model calls, tool calls,
permission decisions, command results, errors, final state, and the order in
which those things happened. It is a small Rust crate and CLI for local harness
engineering, not a SaaS dashboard.

## Where it fits

A serious agent harness has multiple layers:

1. **Runtime and sandbox controls**: Codex, Claude Code, containers, OS
   confinement, and native approval modes.
2. **Policy decision gateways**: tools such as
   [`gommage`](https://github.com/Arakiss/gommage) that decide whether an agent
   may perform a capability.
3. **Trace capture**: the ordered run artifact showing what happened, which
   decisions were made, which tools ran, what failed, and how the run ended.
4. **Review and conversion**: humans or follow-up agents turn failed traces into
   policies, tests, evals, or workflow fixes.
5. **Export surfaces**: OpenTelemetry, dashboards, issue reports, PR comments,
   or HTML summaries.

Traceframe owns layer 3. It does not try to own the whole stack.

`gommage` answers:

```text
What is this agent allowed to do?
```

Traceframe answers:

```text
What did this agent actually do, and why did it fail?
```

## Why

Agent failures are often hard to review after the fact. A transcript is not a
trace. A shell log is not a trace. A permission decision alone does not explain
the full episode around it.

Traceframe comes from private harness engineering work and real daily use of AI
coding agents. It is not a first pass at the problem. The public version is a
small extraction of a recurring operational need: agents need to leave evidence
that another agent or human can inspect after the run.

Traceframe takes a narrow stance:

- **Local-first.** A trace is a file you can inspect, diff, archive, attach to
  an issue, or hand to another agent.
- **Append-only trace files.** Each event is one durable record. Partial writes
  are recoverable and agent-readable.
- **Harness-oriented.** Events are about runs, model calls, tool calls,
  permission decisions, errors, and final state.
- **No SaaS dependency.** Dashboards and OpenTelemetry can come later as export
  surfaces, not as the core contract.
- **Useful failure artifacts.** A failed run should become a policy, test, eval,
  or workflow improvement.

## Install

From this repository:

```bash
cargo install --path .
```

## Quick start

```bash
traceframe run --file run.traceframe --run-id run-demo -- cargo test
traceframe summary --file run.traceframe
traceframe inspect --file run.traceframe
traceframe render --file run.traceframe --html traceframe.html
```

For longer workflows, keep a trace open and append events as the harness runs:

```bash
traceframe init --file workflow.traceframe --run-id run-demo
traceframe record --file workflow.traceframe --kind model.call --payload '{"provider":"openai","model":"gpt"}'
traceframe record --file workflow.traceframe --kind permission.decision --payload '{"capability":"fs.write:README.md","decision":"allow"}'
traceframe exec --file workflow.traceframe -- cargo test
traceframe finish --file workflow.traceframe --status success
traceframe verify --file workflow.traceframe
traceframe summary --file workflow.traceframe
traceframe inspect --file workflow.traceframe
traceframe render --file workflow.traceframe --html traceframe.html
```

`record` remains available for raw structured events. For day-to-day harness
use, `run`, `exec`, and `finish` avoid hand-writing the common `tool.call`,
`tool.result`, and `run.finished` payloads. `summary`, `inspect`, and `render`
also work on open traces so interrupted agent runs can still be reviewed.

Once runs accumulate under `.traceframe/runs/`, rebuild the local ledger. Omit
`--file` when you want `run` to use the default local run directory:

```bash
traceframe run --run-id run-demo -- cargo test
traceframe ledger rebuild
traceframe ledger list
traceframe ledger list --status failed
traceframe ledger show --run-id run-demo
```

The ledger is a derived catalog, not a database and not a second source of
truth. If it is stale or deleted, rebuild it from the trace files.

For host hooks, ingest the JSON payload from stdin instead of wrapping each
command manually:

```bash
traceframe hook ingest \
  --source codex \
  --run-id codex-run-demo \
  --init-if-missing \
  --file .traceframe/runs/codex-run-demo.traceframe <<'JSON'
{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"cargo test"}}
JSON
```

See [`docs/codex-omx-hooks.md`](docs/codex-omx-hooks.md) for the Codex/OMX
hook pattern and the current installation boundary.

## Example output

```text
run_id: run-agent-demo
status: failed
events: 8
model_calls: 1
tool_calls: 1
tool_results: 1
permission_decisions: 2
errors: 1
duration_ms: 110
```

See [`examples/agent-run.traceframe`](examples/agent-run.traceframe)
for a sample run with an allowed permission, a denied permission, a failed tool
result, and a final failed state.

## Library API

Rust harnesses can write traces directly with `TraceRecorder`:

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

See [`docs/harness-integration.md`](docs/harness-integration.md) and
[`examples/harness-recorder.rs`](examples/harness-recorder.rs).

## Event model

v0.1 supports one run per trace file. The public contract is the event model;
the current local file encoding is line-delimited JSON for simple append,
inspection, and recovery.

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

## CLI experience

Traceframe's CLI is designed for both humans and agents:

- commands print stable, aligned summaries;
- `run` creates, records, and closes a command trace in one step;
- wrapped command stdout/stderr is preserved;
- `exec` returns the wrapped command's exit code;
- `hook ingest` lets Codex/OMX-style hosts append tool, result, permission, and
  error events from stdin;
- command traces include argv, exit code, duration, byte counts, and bounded
  stdout/stderr previews;
- open traces can still be summarized, inspected, and rendered;
- `ledger rebuild/list/show` gives agents a stable catalog once many local runs
  exist;
- the raw trace file remains the source of truth when an agent needs to inspect
  or pass the run evidence to another step.

## Quality gate

Every public change should pass the same gate that CI runs:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo llvm-cov --workspace --all-targets --fail-under-lines 80
sh scripts/check-release-readiness.sh
sh scripts/host-smoke.sh
sh scripts/codex-omx-hook-smoke.sh
```

The 80% line-coverage threshold is intentionally modest for v0.1, but it is a
floor, not a target. New command behavior should come with focused tests before
it is treated as part of the tool.

`scripts/host-smoke.sh` is the deeper dogfood path. It creates real success,
failed, manual, and open traces in a temporary workspace, renders HTML, rebuilds
the ledger, filters by status, and verifies the Rust harness example.
`scripts/codex-omx-hook-smoke.sh` separately simulates the host-hook path used
by Codex/OMX-style agent workflows.

## Storage model

Traceframe stores traces as local append-only files. The trace file is the
source of truth. The current implementation uses line-delimited JSON because it
is simple to append, inspect, diff, and recover from. A database may be added
later as a derived local index, but not as the primary record of what the agent
did.

The run ledger is the first derived storage layer. It catalogs local trace files
for discovery, filtering, and handoff, but it is intentionally rebuildable from
`.traceframe/runs/*.traceframe`.

See [`docs/storage.md`](docs/storage.md) for the storage decision record and
tradeoffs.

## Agent Skill

This repo ships an agent-facing skill at
[`skills/traceframe`](skills/traceframe). Install or copy it into Codex/Claude
skill directories when agents should know the correct Traceframe operating
contract, commands, and release gate.

## Product boundaries

Traceframe deliberately starts with one contract: capture a local, ordered,
inspectable record of an agent run. Runtime control, permission policy,
dashboards, OpenTelemetry export, eval suites, and prompt management can connect
around that trace contract, but they should not define v0.1.

## Versioning and changelog

Traceframe is pre-1.0. While the project is in local-MVP/alpha territory,
breaking changes to the event schema, CLI flags, or output contracts may happen
without a major version bump. The short-term goal is not feature breadth; it is
to prove that the local trace contract is useful inside real agent workflows.

## Harness engineering principles

- Agents are primary operators; humans are reviewers and operators.
- A trace must help explain a real run, not decorate a dashboard.
- A failed run should become a test, policy, eval, or workflow improvement.
- Local trace evidence comes before SaaS.
- Export surfaces come after the core trace contract is useful.
