# Storage model

Traceframe records agent runs. The storage decision matters because the trace is
the evidence artifact: it must be readable by humans, parseable by agents, easy
to attach to issues, and robust when a process crashes halfway through a run.

## Decision

The source of truth is one append-only JSONL file per run.

Recommended local layout:

```text
.traceframe/
  runs/
    run-2026-05-03T09-00-00Z.traceframe.jsonl
  reports/
    run-2026-05-03T09-00-00Z.html
```

The CLI currently accepts an explicit `--file` path. That keeps v0.1 simple and
lets agents decide where a trace belongs. A later convenience command can choose
the default `.traceframe/runs/` layout.

## Why JSONL first

JSONL matches the first-principles harness requirement:

- **Append-only.** Each event is one line. The writer does not need to rewrite a
  whole document.
- **Crash tolerant.** If a process dies mid-write, earlier complete lines can
  still be read and verified.
- **Agent-readable.** Codex, Claude Code, shell scripts, and CI can inspect it
  without a database client.
- **Human-readable.** `cat`, `tail`, `rg`, `jq`, and PR attachments work.
- **Versionable.** Sample traces can live in Git, and changes can be reviewed.
- **Portable.** A trace can be copied into an issue, artifact store, or support
  bundle.
- **Exportable.** OpenTelemetry, SQLite, Parquet, HTML, or dashboards can be
  generated from it later.

This is the same kind of narrow contract that makes `gommage` useful: a small
local artifact with explicit semantics is more valuable than a broad platform
that hides the evidence behind a UI.

## Why not a database as source of truth in v0.1

A database is attractive when there are many runs, multiple writers, search
queries, dashboards, retention rules, and aggregate analytics. Traceframe is not
there yet.

Starting with a database would add costs too early:

- harder to inspect with basic tools;
- harder to attach to a GitHub issue or PR;
- schema migrations before the event model is proven;
- binary state that is worse for diffs and review;
- more failure modes around locks, corruption, and client versions;
- risk of designing for dashboards before designing for agent use.

The v0.1 question is not:

```text
Can we query thousands of traces?
```

The v0.1 question is:

```text
Can a real agent run leave behind a trace that explains what happened?
```

## When SQLite should enter

SQLite is the likely first database, but as a derived local index.

Use SQLite when Traceframe needs:

- fast search across many trace files;
- run catalog metadata;
- query by tool, event kind, status, duration, capability, or error;
- joins between traces, eval outcomes, and policy decisions;
- local dashboard support without a server.

Important rule:

```text
SQLite should be rebuildable from JSONL traces.
```

That keeps the event log as evidence and the database as acceleration.

Possible future layout:

```text
.traceframe/
  runs/
    *.traceframe.jsonl
  ledger.jsonl
  index/
    traceframe.sqlite
```

Possible future commands:

```bash
traceframe index rebuild --dir .traceframe/runs --db .traceframe/index/traceframe.sqlite
traceframe index query --status failed --tool shell
```

## Do we need a ledger?

Yes, but not as the first source of truth.

A ledger is useful when there are many traces and agents need a fast catalog of
runs without opening every JSONL file. It should answer:

- which runs exist;
- where each trace file lives;
- which agent/harness created the run;
- when the run started and finished;
- whether it passed, failed, or was cancelled;
- how many events/errors/permission decisions it contains;
- which repository, branch, commit, or task it relates to;
- whether derived reports or indexes exist.

But the ledger should not replace the trace. The trace is the evidence. The
ledger is a catalog.

Recommended rule:

```text
ledger.jsonl must be rebuildable from runs/*.traceframe.jsonl
```

This avoids having two competing truths. If the ledger is corrupt or deleted,
Traceframe can rebuild it. If a trace is deleted, the evidence is gone.

Possible future commands:

```bash
traceframe ledger rebuild --dir .traceframe/runs --out .traceframe/ledger.jsonl
traceframe ledger list --status failed
traceframe ledger show run-agent-demo
```

The ledger should come before SQLite if the next bottleneck is discoverability
for agents. SQLite should come after the ledger if the bottleneck becomes query
speed, joins, dashboards, or aggregate analysis.

## Tradeoffs

### JSONL source of truth

Pros:

- simplest reliable writer;
- excellent for local agents and CI;
- easy to read, diff, copy, and attach;
- works without daemon or service;
- good fit for append-only event streams.

Cons:

- slow search across many runs;
- no indexes;
- weak multi-writer story;
- validation happens at read/verify time;
- large traces may need compression later.

### SQLite primary store

Pros:

- indexes and queries;
- transactions;
- good local dashboard foundation;
- better for thousands of runs.

Cons:

- less transparent for humans and agents;
- migrations required;
- harder to attach/review directly;
- risks turning Traceframe into an app before the trace contract is proven.

### SQLite derived index

Pros:

- keeps JSONL as evidence;
- adds speed when needed;
- rebuildable;
- supports dashboards without sacrificing portability.

Cons:

- duplicate storage;
- index consistency must be checked;
- extra command surface.

### OpenTelemetry export

Pros:

- plugs into existing observability systems;
- useful for teams with collectors and dashboards;
- good long-term export target.

Cons:

- too heavy as v0.1 source of truth;
- agent harness events do not map perfectly to generic spans;
- can hide local review behind infrastructure.

## Current rule

For now:

```text
Write JSONL. Verify JSONL. Render from JSONL. Derive everything else from JSONL.
```

Do not add a database until real trace usage proves that search/indexing is the
actual bottleneck.
