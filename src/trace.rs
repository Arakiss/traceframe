use std::{
    fmt,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
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
    #[serde(rename = "agent.guess")]
    AgentGuess,
    #[serde(rename = "plan.deviation")]
    PlanDeviation,
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
            EventKind::AgentGuess => "agent.guess",
            EventKind::PlanDeviation => "plan.deviation",
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
            "agent.guess" => Ok(EventKind::AgentGuess),
            "plan.deviation" => Ok(EventKind::PlanDeviation),
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

#[derive(Debug, Clone)]
pub struct TraceRecorder {
    path: PathBuf,
}

impl TraceRecorder {
    pub fn start(path: impl AsRef<Path>, run_id: &str, force: bool) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        Trace::init(&path, run_id, force)?;
        Ok(Self { path })
    }

    pub fn open(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn record(&self, kind: EventKind, payload: Value) -> Result<Event> {
        Trace::append(&self.path, kind, payload)
    }

    pub fn model_call(&self, provider: &str, model: &str) -> Result<Event> {
        self.record(
            EventKind::ModelCall,
            json!({
                "provider": provider,
                "model": model,
            }),
        )
    }

    pub fn permission_decision(&self, capability: &str, decision: &str) -> Result<Event> {
        self.record(
            EventKind::PermissionDecision,
            json!({
                "capability": capability,
                "decision": decision,
            }),
        )
    }

    pub fn tool_call<I, S>(&self, tool: &str, command: &str, argv: I) -> Result<Event>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let argv = argv.into_iter().map(Into::into).collect::<Vec<_>>();
        self.record(
            EventKind::ToolCall,
            json!({
                "tool": tool,
                "command": command,
                "argv": argv,
            }),
        )
    }

    pub fn tool_result(
        &self,
        tool: &str,
        command: &str,
        success: bool,
        exit_code: Option<i32>,
        duration_ms: Option<u64>,
    ) -> Result<Event> {
        self.record(
            EventKind::ToolResult,
            json!({
                "tool": tool,
                "command": command,
                "success": success,
                "exit_code": exit_code,
                "duration_ms": duration_ms,
            }),
        )
    }

    pub fn error(&self, message: &str) -> Result<Event> {
        self.record(
            EventKind::Error,
            json!({
                "message": message,
            }),
        )
    }

    pub fn finish(&self, status: &str, summary: Option<&str>) -> Result<Event> {
        let mut payload = json!({ "status": status });
        if let Some(summary) = summary {
            payload["summary"] = Value::String(summary.to_string());
        }
        self.record(EventKind::RunFinished, payload)
    }

    pub fn read(&self) -> Result<Trace> {
        Trace::read(&self.path)
    }

    pub fn summary(&self) -> Result<TraceSummary> {
        Ok(self.read()?.summary())
    }
}

impl Trace {
    pub fn init(path: &Path, run_id: &str, force: bool) -> Result<()> {
        if path.exists() && path.metadata()?.len() > 0 && !force {
            bail!("trace already exists: {}", path.display());
        }

        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let mut payload = json!({"created_by":"traceframe","status":"started"});
        if let Some(host) = crate::host::context() {
            payload["host"] = host;
        }
        let event = Event::new(run_id, EventKind::RunStarted, 0, payload);
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
        self.verify_integrity(true)
    }

    pub fn verify_open(&self) -> Result<()> {
        self.verify_integrity(false)
    }

    fn verify_integrity(&self, require_finished: bool) -> Result<()> {
        if self.events.is_empty() {
            bail!("trace is empty");
        }

        let first = &self.events[0];
        if first.kind != EventKind::RunStarted {
            bail!("first event must be run.started");
        }

        let run_id = &first.run_id;
        let mut previous_seq = None;

        for (index, event) in self.events.iter().enumerate() {
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
            if event.kind == EventKind::RunFinished && index + 1 != self.events.len() {
                bail!("run.finished must be last");
            }
            if let Some(previous) = previous_seq
                && event.seq <= previous
            {
                bail!("seq must increase strictly at seq {}", event.seq);
            }
            previous_seq = Some(event.seq);
        }

        if require_finished && self.events.last().map(|e| e.kind) != Some(EventKind::RunFinished) {
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
            tool_failures: 0,
            permission_decisions: 0,
            errors: 0,
            deviations: 0,
            status: "open".to_string(),
            duration_ms: None,
        };

        for event in &self.events {
            match event.kind {
                EventKind::ModelCall => summary.model_calls += 1,
                EventKind::ToolCall => summary.tool_calls += 1,
                EventKind::ToolResult => {
                    summary.tool_results += 1;
                    if event.payload.get("success").and_then(Value::as_bool) == Some(false) {
                        summary.tool_failures += 1;
                    }
                }
                EventKind::PermissionDecision => summary.permission_decisions += 1,
                EventKind::Error => summary.errors += 1,
                EventKind::AgentGuess | EventKind::PlanDeviation => summary.deviations += 1,
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
    pub tool_failures: usize,
    pub permission_decisions: usize,
    pub errors: usize,
    pub deviations: usize,
    pub status: String,
    pub duration_ms: Option<i128>,
}

impl TraceSummary {
    pub fn render_text(&self) -> String {
        let duration = self
            .duration_ms
            .map(human_duration)
            .unwrap_or_else(|| "unknown".to_string());
        let duration_ms = self
            .duration_ms
            .map(|duration| duration.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        format!(
            "run_id: {}\nstatus: {}\nevents: {}\nmodel_calls: {}\ntool_calls: {}\ntool_results: {}\ntool_failures: {}\npermission_decisions: {}\nerrors: {}\ndeviations: {}\nduration: {duration}\nduration_ms: {duration_ms}\n",
            self.run_id,
            self.status,
            self.event_count,
            self.model_calls,
            self.tool_calls,
            self.tool_results,
            self.tool_failures,
            self.permission_decisions,
            self.errors,
            self.deviations,
        )
    }
}

/// Format a millisecond duration into a compact human-readable string:
/// `850 ms`, `1.50 s`, or `2m 5s` for longer runs.
pub fn human_duration(ms: i128) -> String {
    if ms < 1_000 {
        return format!("{ms} ms");
    }
    if ms < 60_000 {
        let secs = ms as f64 / 1_000.0;
        return format!("{secs:.2} s");
    }
    let total_secs = ms / 1_000;
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    format!("{minutes}m {seconds}s")
}

fn now_ms() -> i128 {
    OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000
}

#[cfg(test)]
#[path = "trace_tests.rs"]
mod tests;
