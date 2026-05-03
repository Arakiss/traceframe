use std::{
    fmt,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    str::FromStr,
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use uuid::Uuid;

pub const TRACEFRAME_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventKind {
    #[serde(rename = "run.started")]
    RunStarted,
    #[serde(rename = "model.call")]
    ModelCall,
    #[serde(rename = "tool.call")]
    ToolCall,
    #[serde(rename = "tool.result")]
    ToolResult,
    #[serde(rename = "permission.decision")]
    PermissionDecision,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "run.finished")]
    RunFinished,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::RunStarted => "run.started",
            EventKind::ModelCall => "model.call",
            EventKind::ToolCall => "tool.call",
            EventKind::ToolResult => "tool.result",
            EventKind::PermissionDecision => "permission.decision",
            EventKind::Error => "error",
            EventKind::RunFinished => "run.finished",
        }
    }
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EventKind {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "run.started" => Ok(EventKind::RunStarted),
            "model.call" => Ok(EventKind::ModelCall),
            "tool.call" => Ok(EventKind::ToolCall),
            "tool.result" => Ok(EventKind::ToolResult),
            "permission.decision" => Ok(EventKind::PermissionDecision),
            "error" => Ok(EventKind::Error),
            "run.finished" => Ok(EventKind::RunFinished),
            other => bail!("unknown event kind: {other}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub version: u16,
    pub run_id: String,
    pub event_id: String,
    pub kind: EventKind,
    pub ts_ms: i128,
    pub seq: u64,
    pub payload: Value,
}

impl Event {
    pub fn new(run_id: impl Into<String>, kind: EventKind, seq: u64, payload: Value) -> Self {
        Self {
            version: TRACEFRAME_VERSION,
            run_id: run_id.into(),
            event_id: Uuid::new_v4().to_string(),
            kind,
            ts_ms: now_ms(),
            seq,
            payload,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Trace {
    pub events: Vec<Event>,
}

impl Trace {
    pub fn init(path: &Path, run_id: &str, force: bool) -> Result<()> {
        if path.exists() && path.metadata()?.len() > 0 && !force {
            bail!("trace already exists: {}", path.display());
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let event = Event::new(
            run_id,
            EventKind::RunStarted,
            0,
            json!({"created_by":"traceframe","status":"started"}),
        );
        fs::write(path, format!("{}\n", serde_json::to_string(&event)?))
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn append(path: &Path, kind: EventKind, payload: Value) -> Result<Event> {
        let trace = Self::read(path)?;
        if trace.events.is_empty() {
            bail!("cannot append to empty trace: {}", path.display());
        }
        if trace
            .events
            .last()
            .is_some_and(|e| e.kind == EventKind::RunFinished)
        {
            bail!("cannot append after run.finished");
        }

        let run_id = trace.events[0].run_id.clone();
        let seq = trace.events.last().map_or(0, |event| event.seq + 1);
        let event = Event::new(run_id, kind, seq, payload);

        let mut file = OpenOptions::new()
            .append(true)
            .open(path)
            .with_context(|| format!("failed to open {}", path.display()))?;
        writeln!(file, "{}", serde_json::to_string(&event)?)
            .with_context(|| format!("failed to append {}", path.display()))?;

        Ok(event)
    }

    pub fn read(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read trace file {}", path.display()))?;
        let mut events = Vec::new();

        for (line_index, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let event = serde_json::from_str::<Event>(trimmed)
                .with_context(|| format!("invalid trace event at line {}", line_index + 1))?;
            events.push(event);
        }

        Ok(Self { events })
    }

    pub fn verify(&self) -> Result<()> {
        if self.events.is_empty() {
            bail!("trace is empty");
        }

        let first = &self.events[0];
        if first.kind != EventKind::RunStarted {
            bail!("first event must be run.started");
        }

        let run_id = &first.run_id;
        let mut previous_seq = None;

        for event in &self.events {
            if event.version != TRACEFRAME_VERSION {
                bail!(
                    "unsupported event version {} at seq {}",
                    event.version,
                    event.seq
                );
            }
            if event.run_id != *run_id {
                bail!("mixed run_id at seq {}", event.seq);
            }
            if event.event_id.trim().is_empty() {
                bail!("empty event_id at seq {}", event.seq);
            }
            if let Some(previous) = previous_seq
                && event.seq <= previous
            {
                bail!("seq must increase strictly at seq {}", event.seq);
            }
            previous_seq = Some(event.seq);
        }

        if self.events.last().map(|e| e.kind) != Some(EventKind::RunFinished) {
            bail!("last event must be run.finished");
        }

        Ok(())
    }

    pub fn inspect(&self) -> String {
        let mut output = String::new();
        for event in &self.events {
            output.push_str(&format!(
                "#{:03} {} run={} event={} payload={}\n",
                event.seq, event.kind, event.run_id, event.event_id, event.payload
            ));
        }
        output
    }

    pub fn summary(&self) -> TraceSummary {
        let mut summary = TraceSummary {
            run_id: self
                .events
                .first()
                .map(|e| e.run_id.clone())
                .unwrap_or_default(),
            event_count: self.events.len(),
            model_calls: 0,
            tool_calls: 0,
            tool_results: 0,
            permission_decisions: 0,
            errors: 0,
            status: "unknown".to_string(),
            duration_ms: None,
        };

        for event in &self.events {
            match event.kind {
                EventKind::ModelCall => summary.model_calls += 1,
                EventKind::ToolCall => summary.tool_calls += 1,
                EventKind::ToolResult => summary.tool_results += 1,
                EventKind::PermissionDecision => summary.permission_decisions += 1,
                EventKind::Error => summary.errors += 1,
                EventKind::RunFinished => {
                    if let Some(status) = event.payload.get("status").and_then(Value::as_str) {
                        summary.status = status.to_string();
                    }
                }
                EventKind::RunStarted => {}
            }
        }

        if let (Some(first), Some(last)) = (self.events.first(), self.events.last()) {
            summary.duration_ms = Some(last.ts_ms.saturating_sub(first.ts_ms));
        }

        summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceSummary {
    pub run_id: String,
    pub event_count: usize,
    pub model_calls: usize,
    pub tool_calls: usize,
    pub tool_results: usize,
    pub permission_decisions: usize,
    pub errors: usize,
    pub status: String,
    pub duration_ms: Option<i128>,
}

impl TraceSummary {
    pub fn render_text(&self) -> String {
        format!(
            "run_id: {}\nstatus: {}\nevents: {}\nmodel_calls: {}\ntool_calls: {}\ntool_results: {}\npermission_decisions: {}\nerrors: {}\nduration_ms: {}\n",
            self.run_id,
            self.status,
            self.event_count,
            self.model_calls,
            self.tool_calls,
            self.tool_results,
            self.permission_decisions,
            self.errors,
            self.duration_ms
                .map(|duration| duration.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        )
    }
}

fn now_ms() -> i128 {
    OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn valid_trace() -> Trace {
        Trace {
            events: vec![
                Event {
                    version: TRACEFRAME_VERSION,
                    run_id: "run-test".into(),
                    event_id: "e0".into(),
                    kind: EventKind::RunStarted,
                    ts_ms: 100,
                    seq: 0,
                    payload: json!({"status":"started"}),
                },
                Event {
                    version: TRACEFRAME_VERSION,
                    run_id: "run-test".into(),
                    event_id: "e1".into(),
                    kind: EventKind::PermissionDecision,
                    ts_ms: 110,
                    seq: 1,
                    payload: json!({"decision":"allow"}),
                },
                Event {
                    version: TRACEFRAME_VERSION,
                    run_id: "run-test".into(),
                    event_id: "e2".into(),
                    kind: EventKind::RunFinished,
                    ts_ms: 120,
                    seq: 2,
                    payload: json!({"status":"success"}),
                },
            ],
        }
    }

    #[test]
    fn event_kind_rejects_unknown_values() {
        let error = "agent.magic".parse::<EventKind>().unwrap_err();
        assert!(error.to_string().contains("unknown event kind"));
    }

    #[test]
    fn event_serialization_round_trips() {
        let event = Event {
            version: TRACEFRAME_VERSION,
            run_id: "run-test".into(),
            event_id: "event-test".into(),
            kind: EventKind::ToolCall,
            ts_ms: 42,
            seq: 1,
            payload: json!({"tool":"shell"}),
        };

        let encoded = serde_json::to_string(&event).unwrap();
        assert!(encoded.contains("tool.call"));
        let decoded: Event = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, event);
    }

    #[test]
    fn valid_trace_verifies() {
        valid_trace().verify().unwrap();
    }

    #[test]
    fn trace_requires_run_started_first() {
        let mut trace = valid_trace();
        trace.events[0].kind = EventKind::ModelCall;
        assert!(trace.verify().is_err());
    }

    #[test]
    fn trace_requires_increasing_sequence() {
        let mut trace = valid_trace();
        trace.events[2].seq = 1;
        assert!(trace.verify().is_err());
    }

    #[test]
    fn trace_requires_run_finished_last() {
        let mut trace = valid_trace();
        trace.events.pop();
        assert!(trace.verify().is_err());
    }

    #[test]
    fn summary_counts_events() {
        let summary = valid_trace().summary();
        assert_eq!(summary.status, "success");
        assert_eq!(summary.permission_decisions, 1);
        assert_eq!(summary.event_count, 3);
    }
}
