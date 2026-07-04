//! Import harness-native session transcripts as closed trace files.
//!
//! Hooks capture the future; the importer captures the past. It backfills
//! traces from transcripts a harness already wrote on disk, mapping messages
//! onto the existing event kinds only — no schema change — and preserving the
//! transcript's own timestamps so durations stay meaningful to layer-4
//! consumers.

use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::trace::{Event, EventKind, TRACEFRAME_VERSION};

/// Maximum characters kept per string when embedding tool inputs or error
/// previews in payloads. Transcript inputs can carry entire file bodies;
/// traces should stay skimmable.
const PREVIEW_CHARS: usize = 400;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportStats {
    pub lines: usize,
    pub skipped_lines: usize,
    pub model_calls: usize,
    pub tool_calls: usize,
    pub tool_results: usize,
    pub tool_failures: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
}

#[derive(Debug)]
pub struct Imported {
    pub run_id: String,
    pub events: Vec<Event>,
    pub stats: ImportStats,
}

/// Parse a Claude Code session transcript (newline-delimited JSON) into an
/// ordered event list ready to be written as one closed trace.
///
/// Mapping: the first assistant `model` (and every model change) becomes a
/// `model.call`; `tool_use` blocks become `tool.call`; `tool_result` blocks
/// become `tool.result` with `success = !is_error`; compaction summaries are
/// skipped; token usage is accumulated onto the `run.finished` payload.
pub fn parse_claude_code(input: &Path, run_id: Option<&str>, source: &str) -> Result<Imported> {
    let file = File::open(input).with_context(|| format!("failed to open {}", input.display()))?;
    let reader = BufReader::new(file);

    let mut stats = ImportStats::default();
    let mut derived_run_id: Option<String> = run_id.map(str::to_string);
    let mut min_ts: Option<i128> = None;
    let mut max_ts: Option<i128> = None;
    let mut last_ts: Option<i128> = None;
    let mut last_model: Option<String> = None;
    let mut calls: HashMap<String, (String, String)> = HashMap::new();
    let mut body: Vec<(EventKind, i128, Value)> = Vec::new();

    for line in reader.lines() {
        let line = line.with_context(|| format!("failed to read {}", input.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        stats.lines += 1;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            stats.skipped_lines += 1;
            continue;
        };

        if derived_run_id.is_none()
            && let Some(session) = value.get("sessionId").and_then(Value::as_str)
        {
            derived_run_id = Some(format!("run-{session}"));
        }

        let is_summary = value.get("type").and_then(Value::as_str) == Some("summary")
            || value.get("isCompactSummary").and_then(Value::as_bool) == Some(true);
        if is_summary {
            stats.skipped_lines += 1;
            continue;
        }

        let ts = value
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(parse_rfc3339_ms)
            .or(last_ts)
            .unwrap_or_else(now_ms);
        min_ts = Some(min_ts.map_or(ts, |current| current.min(ts)));
        max_ts = Some(max_ts.map_or(ts, |current| current.max(ts)));
        last_ts = Some(ts);

        let Some(message) = value.get("message") else {
            continue;
        };

        if let Some(model) = message.get("model").and_then(Value::as_str)
            && last_model.as_deref() != Some(model)
        {
            body.push((
                EventKind::ModelCall,
                ts,
                json!({ "provider": "anthropic", "model": model, "source": source }),
            ));
            stats.model_calls += 1;
            last_model = Some(model.to_string());
        }

        if let Some(usage) = message.get("usage") {
            stats.input_tokens += u64_field(usage, "input_tokens");
            stats.output_tokens += u64_field(usage, "output_tokens");
            stats.cache_read_tokens += u64_field(usage, "cache_read_input_tokens");
            stats.cache_creation_tokens += u64_field(usage, "cache_creation_input_tokens");
        }

        let Some(content) = message.get("content").and_then(Value::as_array) else {
            continue;
        };
        for block in content {
            match block.get("type").and_then(Value::as_str) {
                Some("tool_use") => {
                    let tool = block
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown");
                    let input_value = block.get("input").cloned().unwrap_or(Value::Null);
                    let command = primary_arg(tool, &input_value);
                    let id = block.get("id").and_then(Value::as_str).unwrap_or_default();
                    if !id.is_empty() {
                        calls.insert(id.to_string(), (tool.to_string(), command.clone()));
                    }
                    body.push((
                        EventKind::ToolCall,
                        ts,
                        json!({
                            "tool": tool,
                            "command": command,
                            "argv": [],
                            "tool_use_id": id,
                            "input": truncate_strings(&input_value),
                            "source": source,
                        }),
                    ));
                    stats.tool_calls += 1;
                }
                Some("tool_result") => {
                    let id = block
                        .get("tool_use_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let (tool, command) = calls
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| ("unknown".to_string(), String::new()));
                    let is_error = block
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let text = result_text(block);
                    let mut payload = json!({
                        "tool": tool,
                        "command": command,
                        "success": !is_error,
                        "tool_use_id": id,
                        "content_chars": text.chars().count(),
                        "source": source,
                    });
                    if is_error {
                        payload["error"] = Value::String(preview(&text));
                        stats.tool_failures += 1;
                    }
                    body.push((EventKind::ToolResult, ts, payload));
                    stats.tool_results += 1;
                }
                _ => {}
            }
        }
    }

    if stats.lines == 0 {
        bail!("no transcript lines found in {}", input.display());
    }

    let run_id = derived_run_id.unwrap_or_else(|| {
        format!(
            "run-{}",
            input
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_else(|| "imported".to_string())
        )
    });

    // Transcripts are not strictly chronological (resumed sessions can carry
    // older trailing entries), so the run bounds are the min/max timestamps
    // seen, never the first/last file positions.
    let started_ts = min_ts.unwrap_or_else(now_ms);
    let finished_ts = max_ts.unwrap_or(started_ts);

    let mut events = Vec::with_capacity(body.len() + 2);
    events.push(build_event(
        &run_id,
        EventKind::RunStarted,
        0,
        started_ts,
        json!({
            "created_by": "traceframe-import",
            "status": "started",
            "format": "claude-code",
            "source": source,
            "input": input.display().to_string(),
        }),
    ));
    for (kind, ts, payload) in body {
        let seq = events.len() as u64;
        events.push(build_event(&run_id, kind, seq, ts, payload));
    }
    let seq = events.len() as u64;
    events.push(build_event(
        &run_id,
        EventKind::RunFinished,
        seq,
        finished_ts,
        json!({
            "status": "imported",
            "summary": format!(
                "imported {} transcript lines ({} skipped) from {}",
                stats.lines,
                stats.skipped_lines,
                input.display(),
            ),
            "usage": {
                "input_tokens": stats.input_tokens,
                "output_tokens": stats.output_tokens,
                "cache_read_input_tokens": stats.cache_read_tokens,
                "cache_creation_input_tokens": stats.cache_creation_tokens,
            },
            "source": source,
        }),
    ));

    Ok(Imported {
        run_id,
        events,
        stats,
    })
}

/// Write an imported event list as one trace file, in a single pass.
///
/// The importer deliberately does not use `Trace::append` (which re-reads the
/// whole file per event): imported sessions can carry thousands of events.
pub fn write_trace(target: &Path, events: &[Event], force: bool) -> Result<()> {
    if target.exists() && target.metadata()?.len() > 0 && !force {
        bail!(
            "trace already exists: {} (use --force to overwrite)",
            target.display()
        );
    }
    if let Some(parent) = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut out = String::new();
    for event in events {
        out.push_str(&serde_json::to_string(event)?);
        out.push('\n');
    }
    fs::write(target, out).with_context(|| format!("failed to write {}", target.display()))?;
    Ok(())
}

fn build_event(run_id: &str, kind: EventKind, seq: u64, ts_ms: i128, payload: Value) -> Event {
    Event {
        version: TRACEFRAME_VERSION,
        run_id: run_id.to_string(),
        event_id: Uuid::new_v4().to_string(),
        kind,
        ts_ms,
        seq,
        payload,
    }
}

fn parse_rfc3339_ms(value: &str) -> Option<i128> {
    OffsetDateTime::parse(value, &Rfc3339)
        .ok()
        .map(|ts| ts.unix_timestamp_nanos() / 1_000_000)
}

fn now_ms() -> i128 {
    OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000
}

fn u64_field(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// The one argument that identifies a tool call to a human scanning a trace.
fn primary_arg(tool: &str, input: &Value) -> String {
    let key = match tool {
        "Bash" => "command",
        "Read" | "Write" | "Edit" | "NotebookEdit" => "file_path",
        "Skill" => "skill",
        "WebFetch" => "url",
        "WebSearch" => "query",
        "Glob" | "Grep" => "pattern",
        "Agent" => "description",
        _ => return String::new(),
    };
    input
        .get(key)
        .and_then(Value::as_str)
        .map(preview)
        .unwrap_or_default()
}

/// Recursively cap every string in a payload to a skimmable preview.
fn truncate_strings(value: &Value) -> Value {
    match value {
        Value::String(text) => Value::String(preview(text)),
        Value::Array(items) => Value::Array(items.iter().map(truncate_strings).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, item)| (key.clone(), truncate_strings(item)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn preview(text: &str) -> String {
    let total = text.chars().count();
    if total <= PREVIEW_CHARS {
        return text.to_string();
    }
    let head: String = text.chars().take(PREVIEW_CHARS).collect();
    format!("{head}…(+{} chars)", total - PREVIEW_CHARS)
}

fn result_text(block: &Value) -> String {
    match block.get("content") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}
