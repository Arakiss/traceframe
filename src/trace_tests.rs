use serde_json::json;
use tempfile::tempdir;

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
fn open_trace_verifies_without_finished_event() {
    let mut trace = valid_trace();
    trace.events.pop();

    trace.verify_open().unwrap();
    assert_eq!(trace.summary().status, "open");
}

#[test]
fn trace_rejects_run_finished_before_last_event() {
    let mut trace = valid_trace();
    trace.events.push(Event {
        version: TRACEFRAME_VERSION,
        run_id: "run-test".into(),
        event_id: "e3".into(),
        kind: EventKind::ToolCall,
        ts_ms: 130,
        seq: 3,
        payload: json!({"tool":"shell"}),
    });

    let error = trace.verify_open().unwrap_err();
    assert!(error.to_string().contains("run.finished must be last"));
}

#[test]
fn summary_counts_events() {
    let summary = valid_trace().summary();
    assert_eq!(summary.status, "success");
    assert_eq!(summary.permission_decisions, 1);
    assert_eq!(summary.event_count, 3);
}

#[test]
fn recorder_captures_harness_events() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("run.traceframe");

    let recorder = TraceRecorder::start(&path, "run-recorder", false).unwrap();
    recorder.model_call("openai", "gpt-5.5").unwrap();
    recorder
        .permission_decision("fs.write:README.md", "allow")
        .unwrap();
    recorder
        .tool_call("shell", "cargo test", ["cargo", "test"])
        .unwrap();
    recorder
        .tool_result("shell", "cargo test", true, Some(0), Some(42))
        .unwrap();
    recorder.finish("success", Some("recorder test")).unwrap();

    let summary = recorder.summary().unwrap();

    assert_eq!(recorder.path(), path);
    assert_eq!(summary.run_id, "run-recorder");
    assert_eq!(summary.status, "success");
    assert_eq!(summary.model_calls, 1);
    assert_eq!(summary.permission_decisions, 1);
    assert_eq!(summary.tool_calls, 1);
    assert_eq!(summary.tool_results, 1);
}

#[test]
fn recorder_can_attach_to_existing_trace() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("attached.traceframe");
    Trace::init(&path, "run-attached", false).unwrap();

    let recorder = TraceRecorder::open(&path);
    recorder.error("attached failure").unwrap();
    recorder.finish("failed", None).unwrap();

    let summary = recorder.summary().unwrap();

    assert_eq!(summary.status, "failed");
    assert_eq!(summary.errors, 1);
}
