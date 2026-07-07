use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde_json::Value;
use time::{OffsetDateTime, macros::format_description};

use crate::trace::{Event, EventKind, Trace, TraceSummary, human_duration};

/// Render a trace into a standalone, self-contained HTML report. The report is
/// built so a human can understand one run at a glance: a status header, metric
/// cards, and a colour-coded timeline where every event spells out its tool,
/// arguments, exit code, duration, result, errors and permission decisions.
pub fn write_html(trace: &Trace, path: &Path) -> Result<()> {
    let html = render_document(trace);

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, html).with_context(|| format!("failed to write {}", path.display()))
}

fn render_document(trace: &Trace) -> String {
    let summary = trace.summary();
    let run_id = escape_html(&summary.run_id);
    let status = escape_html(&summary.status);
    let status_cls = status_class(&summary.status);
    let duration_raw = summary
        .duration_ms
        .map(human_duration)
        .unwrap_or_else(|| "unknown".to_string());
    let duration = escape_html(&duration_raw);
    let events = summary.event_count;
    let stats = render_stats(&summary, &duration_raw);

    let first_ts = trace.events.first().map(|event| event.ts_ms).unwrap_or(0);
    let mut timeline = String::new();
    for event in &trace.events {
        timeline.push_str(&render_event(event, first_ts));
    }
    let styles = STYLES;

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>traceframe report - {run_id}</title>
  <style>{styles}</style>
</head>
<body>
  <main>
    <header class="hero">
      <div class="hero-top">
        <h1>traceframe report</h1>
        <span class="badge lg {status_cls}">{status}</span>
      </div>
      <p class="meta">
        <code class="mono">{run_id}</code>
        <span class="dot-sep">·</span> {events} events
        <span class="dot-sep">·</span> {duration}
      </p>
    </header>
    <section class="stats">
{stats}
    </section>
    <section class="timeline-wrap">
      <h2>Timeline</h2>
      <ol class="timeline">
{timeline}
      </ol>
    </section>
  </main>
</body>
</html>
"#
    )
}

fn render_stats(summary: &TraceSummary, duration: &str) -> String {
    let failure_accent = if summary.tool_failures > 0 { "bad" } else { "" };
    let error_accent = if summary.errors > 0 { "bad" } else { "" };
    let deviation_accent = if summary.deviations > 0 { "warn" } else { "" };
    let cards = [
        stat_card(summary.event_count, "events", ""),
        stat_card(summary.model_calls, "model calls", "model"),
        stat_card(summary.tool_calls, "tool calls", "tool"),
        stat_card(summary.tool_results, "tool results", "tool"),
        stat_card(summary.tool_failures, "tool failures", failure_accent),
        stat_card(summary.permission_decisions, "permissions", "info"),
        stat_card(summary.errors, "errors", error_accent),
        stat_card(summary.deviations, "deviations", deviation_accent),
        text_card(duration, "duration", ""),
    ];
    cards.join("\n")
}

fn stat_card(value: usize, label: &str, accent: &str) -> String {
    text_card(&value.to_string(), label, accent)
}

fn text_card(value: &str, label: &str, accent: &str) -> String {
    let value = escape_html(value);
    let label = escape_html(label);
    format!(
        "      <div class=\"stat {accent}\"><span class=\"num\">{value}</span><span class=\"lbl\">{label}</span></div>"
    )
}

fn render_event(event: &Event, first_ts: i128) -> String {
    let accent = event_accent(event);
    let kind = escape_html(event.kind.as_str());
    let kind_cls = kind_badge_class(event.kind);
    let rel = event.ts_ms.saturating_sub(first_ts);
    let rel_label = escape_html(&format!("+{}", human_duration(rel)));
    let clock = escape_html(&format_clock(event.ts_ms));
    let seq = event.seq;
    let body = render_event_body(event);

    format!(
        r#"        <li class="event {accent}">
          <div class="marker"><span class="dot"></span><span class="seq">#{seq:03}</span></div>
          <div class="card">
            <div class="event-head">
              <span class="badge {kind_cls}">{kind}</span>
              <span class="rel">{rel_label}</span>
              <span class="clock">{clock}</span>
            </div>
            <div class="event-body">
{body}            </div>
          </div>
        </li>
"#
    )
}

fn render_event_body(event: &Event) -> String {
    let payload = &event.payload;
    let mut out = String::new();

    match event.kind {
        EventKind::RunStarted => {
            if let Some(status) = pstr(payload, "status") {
                kv(&mut out, "status", &badge(status, "info"));
            }
            if let Some(created_by) = pstr(payload, "created_by") {
                kv(&mut out, "created by", &escape_html(created_by));
            }
        }
        EventKind::ModelCall => {
            if let Some(provider) = pstr(payload, "provider") {
                kv(&mut out, "provider", &badge(provider, "model"));
            }
            if let Some(model) = pstr(payload, "model") {
                kv(&mut out, "model", &chip(model));
            }
        }
        EventKind::ToolCall => {
            if let Some(tool) = pstr(payload, "tool") {
                kv(&mut out, "tool", &badge(tool, "tool"));
            }
            if let Some(command) = pstr(payload, "command") {
                kv(&mut out, "command", &code_block(command));
            }
            if let Some(argv) = payload.get("argv").and_then(Value::as_array)
                && !argv.is_empty()
            {
                kv(&mut out, "argv", &argv_chips(argv));
            }
        }
        EventKind::ToolResult => {
            let (label, class) = match pbool(payload, "success") {
                Some(true) => ("success", "ok"),
                Some(false) => ("failed", "bad"),
                None => ("unknown", "warn"),
            };
            kv(&mut out, "result", &badge(label, class));
            if let Some(tool) = pstr(payload, "tool") {
                kv(&mut out, "tool", &badge(tool, "tool"));
            }
            if let Some(command) = pstr(payload, "command") {
                kv(&mut out, "command", &code_block(command));
            }
            kv(&mut out, "exit code", &exit_badge(payload.get("exit_code")));
            if let Some(ms) = payload.get("duration_ms").and_then(Value::as_i64) {
                kv(
                    &mut out,
                    "duration",
                    &escape_html(&human_duration(ms as i128)),
                );
            }
            let stdout_bytes = payload.get("stdout_bytes").and_then(Value::as_u64);
            let stderr_bytes = payload.get("stderr_bytes").and_then(Value::as_u64);
            if stdout_bytes.is_some() || stderr_bytes.is_some() {
                let line = format!(
                    "stdout {} B · stderr {} B",
                    stdout_bytes.unwrap_or(0),
                    stderr_bytes.unwrap_or(0)
                );
                kv(&mut out, "output", &escape_html(&line));
            }
            if let Some(error) = pstr(payload, "error") {
                kv(&mut out, "error", &callout(error));
            }
            push_preview(&mut out, "stdout", payload.get("stdout_preview"));
            push_preview(&mut out, "stderr", payload.get("stderr_preview"));
        }
        EventKind::PermissionDecision => {
            if let Some(capability) = pstr(payload, "capability") {
                kv(&mut out, "capability", &chip(capability));
            }
            if let Some(decision) = pstr(payload, "decision") {
                let class = decision_class(Some(decision));
                kv(&mut out, "decision", &badge(decision, class));
            }
        }
        EventKind::Error => {
            if let Some(message) = pstr(payload, "message") {
                kv(&mut out, "message", &callout(message));
            }
            if let Some(command) = pstr(payload, "command") {
                kv(&mut out, "command", &code_block(command));
            }
            if let Some(detail) = pstr(payload, "error") {
                kv(&mut out, "detail", &callout(detail));
            }
        }
        EventKind::AgentGuess => {
            if let Some(assumption) = pstr(payload, "assumption") {
                kv(&mut out, "assumption", &callout(assumption));
            }
            if let Some(why) = pstr(payload, "why") {
                kv(&mut out, "why", &escape_html(why));
            }
            if let Some(prevention) = pstr(payload, "prevention") {
                kv(&mut out, "prevention", &escape_html(prevention));
            }
        }
        EventKind::PlanDeviation => {
            if let Some(plan) = pstr(payload, "plan") {
                kv(&mut out, "plan", &escape_html(plan));
            }
            if let Some(deviation) = pstr(payload, "deviation") {
                kv(&mut out, "deviation", &callout(deviation));
            }
            if let Some(why) = pstr(payload, "why") {
                kv(&mut out, "why", &escape_html(why));
            }
        }
        EventKind::RunFinished => {
            if let Some(status) = pstr(payload, "status") {
                kv(&mut out, "status", &badge(status, status_class(status)));
            }
            if let Some(summary) = pstr(payload, "summary") {
                kv(&mut out, "summary", &escape_html(summary));
            }
        }
    }

    out.push_str(&raw_details(payload));
    out
}

fn push_preview(out: &mut String, label: &str, value: Option<&Value>) {
    let Some(text) = value
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
    else {
        return;
    };
    let summary = escape_html(&format!("{label} ({} chars)", text.chars().count()));
    let text = escape_html(text);
    out.push_str(&format!(
        "              <details class=\"out\"><summary>{summary}</summary><pre>{text}</pre></details>\n"
    ));
}

fn raw_details(payload: &Value) -> String {
    let pretty = serde_json::to_string_pretty(payload).unwrap_or_else(|_| payload.to_string());
    let pretty = escape_html(&pretty);
    format!(
        "              <details class=\"raw\"><summary>raw payload</summary><pre>{pretty}</pre></details>\n"
    )
}

fn kv(out: &mut String, key: &str, value_html: &str) {
    let key = escape_html(key);
    out.push_str(&format!(
        "              <div class=\"kv\"><span class=\"k\">{key}</span><span class=\"v\">{value_html}</span></div>\n"
    ));
}

fn badge(text: &str, class: &str) -> String {
    let text = escape_html(text);
    format!("<span class=\"badge {class}\">{text}</span>")
}

fn chip(text: &str) -> String {
    let text = escape_html(text);
    format!("<code class=\"chip\">{text}</code>")
}

fn code_block(text: &str) -> String {
    let text = escape_html(text);
    format!("<pre class=\"cmd\">{text}</pre>")
}

fn callout(text: &str) -> String {
    let text = escape_html(text);
    format!("<div class=\"callout bad\">{text}</div>")
}

fn argv_chips(values: &[Value]) -> String {
    let mut out = String::from("<div class=\"chips\">");
    for value in values {
        let text = value
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| value.to_string());
        out.push_str(&chip(&text));
    }
    out.push_str("</div>");
    out
}

fn exit_badge(value: Option<&Value>) -> String {
    match value {
        Some(Value::Number(code)) => {
            let class = if code.as_i64() == Some(0) {
                "ok"
            } else {
                "bad"
            };
            badge(&format!("exit {code}"), class)
        }
        _ => badge("exit —", "warn"),
    }
}

fn event_accent(event: &Event) -> &'static str {
    match event.kind {
        EventKind::RunStarted => "ev-info",
        EventKind::ModelCall => "ev-model",
        EventKind::ToolCall => "ev-tool",
        EventKind::ToolResult => match pbool(&event.payload, "success") {
            Some(true) => "ev-ok",
            Some(false) => "ev-bad",
            None => "ev-warn",
        },
        EventKind::PermissionDecision => match decision_class(pstr(&event.payload, "decision")) {
            "ok" => "ev-ok",
            "bad" => "ev-bad",
            _ => "ev-warn",
        },
        EventKind::Error => "ev-bad",
        EventKind::AgentGuess | EventKind::PlanDeviation => "ev-warn",
        EventKind::RunFinished => match pstr(&event.payload, "status") {
            Some("success") => "ev-ok",
            Some("failed") => "ev-bad",
            _ => "ev-warn",
        },
    }
}

fn kind_badge_class(kind: EventKind) -> &'static str {
    match kind {
        EventKind::RunStarted | EventKind::RunFinished | EventKind::PermissionDecision => "info",
        EventKind::ModelCall => "model",
        EventKind::ToolCall | EventKind::ToolResult => "tool",
        EventKind::Error => "bad",
        EventKind::AgentGuess | EventKind::PlanDeviation => "warn",
    }
}

fn status_class(status: &str) -> &'static str {
    match status {
        "success" => "ok",
        "failed" | "error" => "bad",
        "open" | "started" | "running" => "warn",
        _ => "info",
    }
}

fn decision_class(decision: Option<&str>) -> &'static str {
    match decision.map(str::to_ascii_lowercase).as_deref() {
        Some("allow" | "allowed" | "approve" | "approved" | "permit") => "ok",
        Some("deny" | "denied" | "block" | "blocked" | "reject" | "rejected") => "bad",
        _ => "warn",
    }
}

fn pstr<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(Value::as_str)
}

fn pbool(payload: &Value, key: &str) -> Option<bool> {
    payload.get(key).and_then(Value::as_bool)
}

fn format_clock(ts_ms: i128) -> String {
    let nanos = ts_ms.saturating_mul(1_000_000);
    OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .ok()
        .and_then(|dt| {
            dt.format(format_description!(
                "[hour]:[minute]:[second].[subsecond digits:3]Z"
            ))
            .ok()
        })
        .unwrap_or_else(|| format!("{ts_ms} ms"))
}

fn escape_html(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

const STYLES: &str = r#"
:root {
  --bg: #f4f6fb;
  --card: #ffffff;
  --ink: #1f2933;
  --muted: #64748b;
  --line: #e2e8f0;
  --ok: #16a34a; --ok-bg: #dcfce7;
  --bad: #dc2626; --bad-bg: #fee2e2;
  --warn: #d97706; --warn-bg: #fef3c7;
  --tool: #2563eb; --tool-bg: #dbeafe;
  --model: #7c3aed; --model-bg: #ede9fe;
  --info: #475569; --info-bg: #e2e8f0;
}
* { box-sizing: border-box; }
body { font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif; margin: 0; padding: 2.5rem 1.25rem; color: var(--ink); background: var(--bg); }
main { max-width: 980px; margin: 0 auto; }
.mono, code, pre { font-family: ui-monospace, "SF Mono", "JetBrains Mono", Menlo, monospace; }

.hero { margin-bottom: 1.75rem; }
.hero-top { display: flex; align-items: center; gap: .85rem; flex-wrap: wrap; }
h1 { font-size: 1.6rem; margin: 0; letter-spacing: -.01em; }
h2 { font-size: 1rem; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); margin: 0 0 .9rem; }
.meta { color: var(--muted); margin: .55rem 0 0; font-size: .95rem; }
.meta .mono { background: #fff; padding: .15rem .45rem; border-radius: 6px; border: 1px solid var(--line); color: var(--ink); }
.dot-sep { margin: 0 .35rem; color: #cbd5e1; }

.stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(120px, 1fr)); gap: .75rem; margin-bottom: 2rem; }
.stat { background: var(--card); border: 1px solid var(--line); border-radius: 12px; padding: .85rem 1rem; display: flex; flex-direction: column; gap: .15rem; }
.stat .num { font-size: 1.5rem; font-weight: 650; line-height: 1; }
.stat .lbl { font-size: .78rem; color: var(--muted); text-transform: uppercase; letter-spacing: .04em; }
.stat.bad .num { color: var(--bad); }
.stat.tool .num { color: var(--tool); }
.stat.model .num { color: var(--model); }
.stat.info .num { color: var(--info); }
.stat.warn .num { color: var(--warn); }

.timeline { list-style: none; margin: 0; padding: 0; position: relative; }
.timeline::before { content: ""; position: absolute; left: 7px; top: 6px; bottom: 6px; width: 2px; background: var(--line); }
.event { position: relative; display: grid; grid-template-columns: 64px 1fr; gap: .9rem; padding-bottom: 1.1rem; --accent: var(--info); --accent-bg: var(--info-bg); }
.event.ev-ok { --accent: var(--ok); --accent-bg: var(--ok-bg); }
.event.ev-bad { --accent: var(--bad); --accent-bg: var(--bad-bg); }
.event.ev-warn { --accent: var(--warn); --accent-bg: var(--warn-bg); }
.event.ev-tool { --accent: var(--tool); --accent-bg: var(--tool-bg); }
.event.ev-model { --accent: var(--model); --accent-bg: var(--model-bg); }
.event.ev-info { --accent: var(--info); --accent-bg: var(--info-bg); }
.marker { display: flex; align-items: center; gap: .35rem; padding-top: .15rem; }
.dot { width: 16px; height: 16px; border-radius: 50%; background: var(--accent); border: 3px solid var(--bg); box-shadow: 0 0 0 1px var(--accent); flex: none; }
.seq { font-size: .72rem; color: var(--muted); font-variant-numeric: tabular-nums; }
.card { background: var(--card); border: 1px solid var(--line); border-left: 4px solid var(--accent); border-radius: 10px; padding: .8rem .95rem; overflow: hidden; }
.event-head { display: flex; align-items: center; gap: .6rem; flex-wrap: wrap; margin-bottom: .5rem; }
.event-head .rel { font-weight: 600; font-variant-numeric: tabular-nums; font-size: .9rem; }
.event-head .clock { color: var(--muted); font-size: .8rem; font-family: ui-monospace, monospace; }

.badge { display: inline-block; padding: .12rem .55rem; border-radius: 999px; font-size: .78rem; font-weight: 600; background: var(--info-bg); color: var(--info); }
.badge.lg { font-size: .95rem; padding: .25rem .8rem; }
.badge.ok { background: var(--ok-bg); color: var(--ok); }
.badge.bad { background: var(--bad-bg); color: var(--bad); }
.badge.warn { background: var(--warn-bg); color: var(--warn); }
.badge.tool { background: var(--tool-bg); color: var(--tool); }
.badge.model { background: var(--model-bg); color: var(--model); }
.badge.info { background: var(--info-bg); color: var(--info); }

.kv { display: grid; grid-template-columns: 110px 1fr; gap: .6rem; padding: .22rem 0; align-items: start; }
.kv .k { color: var(--muted); font-size: .82rem; text-transform: uppercase; letter-spacing: .03em; padding-top: .1rem; }
.kv .v { min-width: 0; }
.chips { display: flex; flex-wrap: wrap; gap: .35rem; }
.chip { background: #f1f5f9; border: 1px solid var(--line); border-radius: 6px; padding: .1rem .4rem; font-size: .82rem; }
pre.cmd { margin: 0; background: #0f172a; color: #e2e8f0; padding: .55rem .7rem; border-radius: 8px; font-size: .85rem; white-space: pre-wrap; word-break: break-word; }
.callout { background: var(--bad-bg); color: var(--bad); border-radius: 8px; padding: .45rem .65rem; font-size: .88rem; white-space: pre-wrap; word-break: break-word; }

details { margin-top: .4rem; }
details > summary { cursor: pointer; color: var(--muted); font-size: .82rem; }
details pre { margin: .4rem 0 0; background: #f8fafc; border: 1px solid var(--line); border-radius: 8px; padding: .55rem .7rem; font-size: .82rem; white-space: pre-wrap; word-break: break-word; max-height: 320px; overflow: auto; }
details.raw > summary { color: #94a3b8; }
"#;

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;
    use tempfile::tempdir;

    use crate::trace::{Event, EventKind, TRACEFRAME_VERSION, Trace};

    fn event(kind: EventKind, seq: u64, ts_ms: i128, payload: serde_json::Value) -> Event {
        Event {
            version: TRACEFRAME_VERSION,
            run_id: "run-html".into(),
            event_id: format!("e{seq}"),
            kind,
            ts_ms,
            seq,
            payload,
        }
    }

    #[test]
    fn render_escapes_payload_html() {
        let trace = Trace {
            events: vec![
                event(EventKind::RunStarted, 0, 1, json!({"status":"started"})),
                event(
                    EventKind::Error,
                    1,
                    2,
                    json!({"message":"<script>alert('x')</script> & bad"}),
                ),
                event(EventKind::RunFinished, 2, 3, json!({"status":"failed"})),
            ],
        };
        let dir = tempdir().unwrap();
        let path = dir.path().join("report.html");

        super::write_html(&trace, &path).unwrap();
        let rendered = fs::read_to_string(path).unwrap();

        assert!(rendered.contains("&lt;script&gt;"));
        assert!(!rendered.contains("<script>alert"));
        assert!(rendered.contains("&amp; bad"));
    }

    #[test]
    fn render_shows_timeline_tools_states_and_durations() {
        let trace = Trace {
            events: vec![
                event(EventKind::RunStarted, 0, 1_000, json!({"status":"started"})),
                event(
                    EventKind::PermissionDecision,
                    1,
                    1_010,
                    json!({"capability":"git.push","decision":"deny"}),
                ),
                event(
                    EventKind::ToolCall,
                    2,
                    1_020,
                    json!({"tool":"shell","command":"git push origin main","argv":["git","push","origin","main"]}),
                ),
                event(
                    EventKind::ToolResult,
                    3,
                    2_520,
                    json!({
                        "tool":"shell",
                        "command":"git push origin main",
                        "success":false,
                        "exit_code":1,
                        "duration_ms":1500,
                        "stdout_bytes":0,
                        "stderr_bytes":24,
                        "stderr_preview":"denied by policy"
                    }),
                ),
                event(EventKind::RunFinished, 4, 2_530, json!({"status":"failed"})),
            ],
        };
        let dir = tempdir().unwrap();
        let path = dir.path().join("rich.html");

        super::write_html(&trace, &path).unwrap();
        let html = fs::read_to_string(path).unwrap();

        // Timeline structure and per-event detail.
        assert!(html.contains("Timeline"));
        assert!(html.contains("git push origin main"));
        assert!(html.contains("exit 1"));
        // Relative time measured from the first event.
        assert!(html.contains("+0 ms"));
        // Duration rendered in human form.
        assert!(html.contains("1.50 s"));
        // Colour-coded states.
        assert!(html.contains("ev-bad"));
        assert!(html.contains("deny"));
        assert!(html.contains("denied by policy"));
        // Stat cards.
        assert!(html.contains("tool failures"));
    }
}
