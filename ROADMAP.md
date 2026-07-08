# Roadmap

Slod's mission is unchanged and deliberately narrow: **own layer 3** of
the agent-harness stack — the ordered, append-only, verifiable record of what
an agent run actually did. Everything below serves one consumer story:

> A layer-4 tool (a trace miner, a reviewer agent, an eval harness) reads
> slod ledgers and turns failures into policies, tests, evals, and
> harness fixes. Slod records; consumers decide.

The first such consumer exists as a working prototype (a deterministic census
over two weeks of real agent traces: redundant reads, schema-validation retry
storms, protocol violations, invented file paths). Its needs drive v0.2.

## v0.2 — feed the miners

- **Transcript importer.** ✅ landed for `claude-code` and `codex` (`docs/import.md`). `slod import --format claude-code --input <session.jsonl>`
  (or `--format codex`): backfill traces from harness-native session
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
  `slod record --kind agent.guess`. This is the empirical basis for
  operator-side "guess logs".
- **Ledger export.** ✅ landed: `slod ledger export --jsonl` — a stable, documented
  line format so consumers never parse `.slod` internals directly.

## v0.3 — capture everywhere

- **Session-directory hook wiring guides** for Claude Code and Codex/OMX
  (`docs/hooks.md`, `docs/codex-hooks.md`), including recommended event
  subsets to keep per-call overhead negligible.
- **Cursor as a capture host.** Cursor ships an agent-hook surface
  (`~/.cursor/hooks.json`: `sessionStart`, `preToolUse`/`postToolUse`,
  `beforeShellExecution`/`afterShellExecution`, `afterFileEdit`, `stop` —
  see cursor.com/docs/hooks) whose events map naturally onto `run.started`,
  `tool.call`/`tool.result`, and `run.finished`. Needs: a payload mapping for
  Cursor's field names (`conversation_id`, `generation_id`,
  `hook_event_name`) in hook ingestion, a `docs/cursor-hooks.md` wiring
  guide, and `cursor` exercised in the `--source` tests. Cloud-agent
  sessions have a reduced hook set — document the gap instead of papering
  over it.
- **Cross-harness runs.** One operator, many harnesses, one ledger: document
  and test the `--source` discipline (`claude-code`, `codex`, `cursor`,
  `policy`) so a mixed fleet lands in comparable traces. The source label is
  already free-form at ingestion; the discipline is naming, not code.
- **Retention.** `slod ledger gc --keep-days N` — traces are evidence,
  not hoarding.

## v1.0 — schema freeze

- Stabilize the event schema and CLI surface documented in `docs/storage.md`;
  from then on, additive changes only.
- Conformance: `slod verify` becomes the compatibility gate consumers
  can rely on in CI.

## Non-goals (the narrowness is the product)

- No mining, scoring, or failure analysis — that is layer 4, by design.
- No dashboards, no SaaS, no OpenTelemetry in core (export surfaces may wrap
  slod; they do not enter it).
- No policy decisions — layer 2 (e.g. a policy gateway like gommage) decides;
  slod records the decision.
