use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

mod common;
use common::*;

/// A miniature Claude Code session transcript: a user turn, an assistant turn
/// with a model + a Bash tool call, its successful result, a second tool call
/// whose result is an error, and a compaction summary that must be skipped.
const TRANSCRIPT: &str = concat!(
    r#"{"type":"user","sessionId":"abc123","timestamp":"2026-07-01T10:00:00.000Z","message":{"role":"user","content":"hola"}}"#,
    "\n",
    r#"{"type":"assistant","timestamp":"2026-07-01T10:00:05.000Z","message":{"role":"assistant","model":"claude-fable-5","usage":{"input_tokens":10,"output_tokens":50,"cache_read_input_tokens":100},"content":[{"type":"text","text":"ok"},{"type":"tool_use","id":"tu1","name":"Bash","input":{"command":"cargo test"}}]}}"#,
    "\n",
    r#"{"type":"user","timestamp":"2026-07-01T10:00:07.000Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"tu1","content":"test result: ok","is_error":false}]}}"#,
    "\n",
    r#"{"type":"assistant","timestamp":"2026-07-01T10:00:09.000Z","message":{"role":"assistant","model":"claude-fable-5","usage":{"output_tokens":20},"content":[{"type":"tool_use","id":"tu2","name":"Read","input":{"file_path":"/tmp/missing.rs"}}]}}"#,
    "\n",
    r#"{"type":"user","timestamp":"2026-07-01T10:00:10.000Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"tu2","content":"File does not exist.","is_error":true}]}}"#,
    "\n",
    r#"{"type":"summary","summary":"compacted"}"#,
    "\n",
);

/// 2026-07-01T10:00:00Z in unix milliseconds.
const FIRST_TS_MS: i64 = 1_782_900_000_000;

#[test]
fn import_maps_a_claude_code_transcript_to_a_closed_verified_trace() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("session.jsonl");
    fs::write(&input, TRANSCRIPT).unwrap();
    let runs = dir.path().join("runs");

    traceframe()
        .args(["import", "--format", "claude-code", "--input"])
        .arg(&input)
        .arg("--dir")
        .arg(&runs)
        .assert()
        .success()
        .stdout(predicate::str::contains("traceframe import"))
        .stdout(predicate::str::contains("run-abc123"))
        .stdout(predicate::str::contains("tool_failures 1"));

    let trace_path = runs.join("run-abc123.traceframe");
    assert!(trace_path.exists(), "derived per-run trace file exists");

    traceframe()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .success();

    traceframe()
        .args(["summary", "--file"])
        .arg(&trace_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("run_id: run-abc123"))
        .stdout(predicate::str::contains("status: imported"))
        .stdout(predicate::str::contains("model_calls: 1"))
        .stdout(predicate::str::contains("tool_calls: 2"))
        .stdout(predicate::str::contains("tool_results: 2"))
        .stdout(predicate::str::contains("tool_failures: 1"));

    // Timestamps come from the transcript, not from import time.
    let first = first_event(&trace_path);
    assert_eq!(first["kind"], "run.started");
    assert_eq!(first["ts_ms"], serde_json::json!(FIRST_TS_MS));

    // The failing result keeps an error preview; usage lands on run.finished.
    let content = fs::read_to_string(&trace_path).unwrap();
    let last = content.lines().last().unwrap();
    let finished: serde_json::Value = serde_json::from_str(last).unwrap();
    assert_eq!(finished["kind"], "run.finished");
    assert_eq!(finished["payload"]["usage"]["output_tokens"], 70);
    assert_eq!(finished["payload"]["usage"]["cache_read_input_tokens"], 100);
    assert!(content.contains("File does not exist."));
}

#[test]
fn import_refuses_existing_target_unless_forced() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("session.jsonl");
    fs::write(&input, TRANSCRIPT).unwrap();
    let target = dir.path().join("imported.traceframe");

    traceframe()
        .args(["import", "--format", "claude-code", "--input"])
        .arg(&input)
        .arg("--file")
        .arg(&target)
        .assert()
        .success();

    traceframe()
        .args(["import", "--format", "claude-code", "--input"])
        .arg(&input)
        .arg("--file")
        .arg(&target)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    traceframe()
        .args(["import", "--format", "claude-code", "--force", "--input"])
        .arg(&input)
        .arg("--file")
        .arg(&target)
        .assert()
        .success();
}

#[test]
fn import_rejects_unknown_formats_and_empty_inputs() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("session.jsonl");
    fs::write(&input, TRANSCRIPT).unwrap();

    traceframe()
        .args(["import", "--format", "martian", "--input"])
        .arg(&input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported --format"));

    let empty = dir.path().join("empty.jsonl");
    fs::write(&empty, "").unwrap();
    traceframe()
        .args(["import", "--format", "claude-code", "--input"])
        .arg(&empty)
        .arg("--dir")
        .arg(dir.path().join("runs"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("no transcript lines"));
}
