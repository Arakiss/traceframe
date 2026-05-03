use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::trace::Trace;

pub fn write_html(trace: &Trace, path: &Path) -> Result<()> {
    let summary = trace.summary();
    let mut rows = String::new();

    for event in &trace.events {
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td><pre>{}</pre></td></tr>\n",
            event.seq,
            escape_html(event.kind.as_str()),
            event.ts_ms,
            escape_html(&event.payload.to_string())
        ));
    }

    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>traceframe report - {run_id}</title>
  <style>
    body {{ font-family: ui-sans-serif, system-ui, sans-serif; margin: 2rem; color: #1f2933; background: #f8fafc; }}
    main {{ max-width: 1040px; margin: 0 auto; }}
    h1 {{ margin-bottom: .25rem; }}
    .meta {{ color: #52616b; margin-bottom: 1.5rem; }}
    table {{ width: 100%; border-collapse: collapse; background: white; }}
    th, td {{ border: 1px solid #d9e2ec; padding: .6rem; text-align: left; vertical-align: top; }}
    th {{ background: #eef2f7; }}
    pre {{ margin: 0; white-space: pre-wrap; word-break: break-word; }}
  </style>
</head>
<body>
  <main>
    <h1>traceframe report</h1>
    <p class="meta">run_id: {run_id} | status: {status} | events: {events} | errors: {errors}</p>
    <table>
      <thead><tr><th>seq</th><th>kind</th><th>ts_ms</th><th>payload</th></tr></thead>
      <tbody>
{rows}
      </tbody>
    </table>
  </main>
</body>
</html>
"#,
        run_id = escape_html(&summary.run_id),
        status = escape_html(&summary.status),
        events = summary.event_count,
        errors = summary.errors,
        rows = rows
    );

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, html).with_context(|| format!("failed to write {}", path.display()))
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

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;
    use tempfile::tempdir;

    use crate::trace::{Event, EventKind, TRACEFRAME_VERSION, Trace};

    #[test]
    fn render_escapes_payload_html() {
        let trace = Trace {
            events: vec![
                Event {
                    version: TRACEFRAME_VERSION,
                    run_id: "run-html".into(),
                    event_id: "e0".into(),
                    kind: EventKind::RunStarted,
                    ts_ms: 1,
                    seq: 0,
                    payload: json!({"status":"started"}),
                },
                Event {
                    version: TRACEFRAME_VERSION,
                    run_id: "run-html".into(),
                    event_id: "e1".into(),
                    kind: EventKind::Error,
                    ts_ms: 2,
                    seq: 1,
                    payload: json!({"message":"<script>alert('x')</script> & bad"}),
                },
                Event {
                    version: TRACEFRAME_VERSION,
                    run_id: "run-html".into(),
                    event_id: "e2".into(),
                    kind: EventKind::RunFinished,
                    ts_ms: 3,
                    seq: 2,
                    payload: json!({"status":"failed"}),
                },
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
}
