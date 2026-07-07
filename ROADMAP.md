# Roadmap

Traceframe's mission is unchanged and deliberately narrow: **own layer 3** of
the agent-harness stack — the ordered, append-only, verifiable record of what
an agent run actually did. Everything below serves one consumer story:

> A layer-4 tool (a trace miner, a reviewer agent, an eval harness) reads
> traceframe ledgers and turns failures into policies, tests, evals, and
> harness fixes. Traceframe records; consumers decide.

The first such consumer exists as a working prototype (a deterministic census
over two weeks of real agent traces: redundant reads, schema-validation retry
storms, protocol violations, invented file paths). Its needs drive v0.2.

## v0.2 — feed the miners

- **Transcript importer.** ✅ landed for `claude-code` (`docs/import.md`); `codex` format pending. `traceframe import --format claude-code --input <session.jsonl>`
  (and `--format codex`): backfill traces from harness-native session
  transcripts, mapping messages to `model.call`, `tool.call`, `tool.result`
  (with `is_error`), and `run.finished`. Hooks capture the future; the importer
  captures the past. Idempotent, append-only, safe to re-run.
- **Run metadata event.** `run.meta` carrying model id, token usage
  (input/output/cache), and source label — miners rank findings by cost, so
  cost must live in the trace.
- **Deviation events.** ✅ landed: event kinds `agent.guess` and `plan.deviation`
  — the moment an agent improvises through an unknown (unverified assumption,
  conservative fallback, deviation from an approved plan). Payload conventions:
  `assumption`/`why`/`prevention` and `plan`/`deviation`/`why`. Counted as
  `deviations` in summaries, rendered with a warn accent, emitted via
  `traceframe record --kind agent.guess`. This is the empirical basis for
  operator-side "guess logs".
- **Ledger export.** `traceframe ledger export --jsonl` — a stable, documented
  line format so consumers never parse `.traceframe` internals directly.

## v0.3 — capture everywhere

- **Session-directory hook wiring guides** for Claude Code and Codex/OMX
  (`docs/hooks.md`, `docs/codex-hooks.md`), including recommended event
  subsets to keep per-call overhead negligible.
- **Cross-harness runs.** One operator, two harnesses, one ledger: document
  and test the `--source` discipline (`claude-code`, `codex`, `policy`) so a
  mixed fleet lands in comparable traces.
- **Retention.** `traceframe ledger gc --keep-days N` — traces are evidence,
  not hoarding.

## v1.0 — schema freeze

- Stabilize the event schema and CLI surface documented in `docs/storage.md`;
  from then on, additive changes only.
- Conformance: `traceframe verify` becomes the compatibility gate consumers
  can rely on in CI.

## Non-goals (the narrowness is the product)

- No mining, scoring, or failure analysis — that is layer 4, by design.
- No dashboards, no SaaS, no OpenTelemetry in core (export surfaces may wrap
  traceframe; they do not enter it).
- No policy decisions — layer 2 (e.g. a policy gateway like gommage) decides;
  traceframe records the decision.
