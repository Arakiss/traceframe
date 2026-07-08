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

/// A miniature Codex rollout: session metadata, one model turn, a failing
/// shell tool, a successful custom patch tool, token usage, one explicit error,
/// and a compaction marker that must be skipped.
const CODEX_ROLLOUT: &str = concat!(
    r#"{"timestamp":"2026-07-02T10:00:04.000Z","type":"session_meta","payload":{"session_id":"codex123","id":"codex123","timestamp":"2026-07-02T10:00:04.000Z","cwd":"/workspace/project","originator":"codex","cli_version":"0.0.0","source":"cli","model_provider":"openai"}}"#,
    "\n",
    r#"{"timestamp":"2026-07-02T10:00:06.000Z","type":"turn_context","payload":{"turn_id":"turn-1","cwd":"/workspace/project","workspace_roots":["/workspace/project"],"current_date":"2026-07-02","timezone":"UTC","approval_policy":"never","sandbox_policy":{"type":"danger-full-access"},"permission_profile":{"type":"disabled"},"model":"gpt-example","effort":"high"}}"#,
    "\n",
    r#"{"timestamp":"2026-07-02T10:00:08.000Z","type":"response_item","payload":{"type":"function_call","id":"fc1","name":"exec_command","arguments":"{\"cmd\":\"cargo test\",\"workdir\":\"/workspace/project\"}","call_id":"call1"}}"#,
    "\n",
    r#"{"timestamp":"2026-07-02T10:00:09.000Z","type":"response_item","payload":{"type":"function_call_output","call_id":"call1","output":"Process exited with code 1\nstderr: failed"}}"#,
    "\n",
    r#"{"timestamp":"2026-07-02T10:00:10.000Z","type":"response_item","payload":{"type":"custom_tool_call","id":"patch1","name":"apply_patch","status":"completed","input":"*** Begin Patch\n*** End Patch","call_id":"call2"}}"#,
    "\n",
    r#"{"timestamp":"2026-07-02T10:00:11.000Z","type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"call2","output":"Success. Updated files."}}"#,
    "\n",
    r#"{"timestamp":"2026-07-02T10:00:12.000Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":10,"cached_input_tokens":3,"output_tokens":5,"reasoning_output_tokens":1,"total_tokens":18}}}}"#,
    "\n",
    r#"{"timestamp":"2026-07-02T10:00:13.000Z","type":"event_msg","payload":{"type":"error","message":"synthetic transient failure"}}"#,
    "\n",
    r#"{"timestamp":"2026-07-02T10:00:14.000Z","type":"event_msg","payload":{"type":"context_compacted"}}"#,
    "\n",
);

const OUT_OF_ORDER_CODEX_ROLLOUT: &str = concat!(
    r#"{"timestamp":"2026-07-02T10:00:10.000Z","type":"session_meta","payload":{"session_id":"codex-clamp","id":"codex-clamp","timestamp":"2026-07-02T10:00:10.000Z","cwd":"/workspace/project","originator":"codex","cli_version":"0.0.0","source":"cli","model_provider":"openai"}}"#,
    "\n",
    r#"{"timestamp":"2026-07-02T10:00:08.000Z","type":"turn_context","payload":{"turn_id":"turn-1","cwd":"/workspace/project","workspace_roots":["/workspace/project"],"current_date":"2026-07-02","timezone":"UTC","approval_policy":"never","sandbox_policy":{"type":"danger-full-access"},"permission_profile":{"type":"disabled"},"model":"gpt-example","effort":"high"}}"#,
    "\n",
    r#"{"timestamp":"2026-07-02T10:00:00.000Z","type":"event_msg","payload":{"type":"user_message","message":"synthetic prompt"}}"#,
    "\n",
);

/// 2026-07-01T10:00:00Z in unix milliseconds.
const FIRST_TS_MS: i64 = 1_782_900_000_000;
/// 2026-07-02T10:00:00Z in unix milliseconds.
const CODEX_FIRST_TS_MS: i64 = 1_782_986_400_000;

#[test]
fn import_maps_a_claude_code_transcript_to_a_closed_verified_trace() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("session.jsonl");
    fs::write(&input, TRANSCRIPT).unwrap();
    let runs = dir.path().join("runs");

    slod()
        .args(["import", "--format", "claude-code", "--input"])
        .arg(&input)
        .arg("--dir")
        .arg(&runs)
        .assert()
        .success()
        .stdout(predicate::str::contains("slod import"))
        .stdout(predicate::str::contains("run-abc123"))
        .stdout(predicate::str::contains("tool_failures 1"));

    let trace_path = runs.join("run-abc123.slod");
    assert!(trace_path.exists(), "derived per-run trace file exists");

    slod()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .success();

    slod()
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
    let target = dir.path().join("imported.slod");

    slod()
        .args(["import", "--format", "claude-code", "--input"])
        .arg(&input)
        .arg("--file")
        .arg(&target)
        .assert()
        .success();

    slod()
        .args(["import", "--format", "claude-code", "--input"])
        .arg(&input)
        .arg("--file")
        .arg(&target)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    slod()
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

    slod()
        .args(["import", "--format", "martian", "--input"])
        .arg(&input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported --format"));

    let empty = dir.path().join("empty.jsonl");
    fs::write(&empty, "").unwrap();
    slod()
        .args(["import", "--format", "claude-code", "--input"])
        .arg(&empty)
        .arg("--dir")
        .arg(dir.path().join("runs"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("no transcript lines"));
}

#[test]
fn import_maps_a_codex_rollout_to_a_closed_verified_trace() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("rollout.jsonl");
    fs::write(&input, CODEX_ROLLOUT).unwrap();
    let runs = dir.path().join("runs");

    slod()
        .args(["import", "--format", "codex", "--input"])
        .arg(&input)
        .arg("--dir")
        .arg(&runs)
        .assert()
        .success()
        .stdout(predicate::str::contains("slod import"))
        .stdout(predicate::str::contains("run-codex123"))
        .stdout(predicate::str::contains("model_calls 1"))
        .stdout(predicate::str::contains("tool_calls  2"))
        .stdout(predicate::str::contains("tool_failures 1"))
        .stdout(predicate::str::contains("skipped     1"));

    let trace_path = runs.join("run-codex123.slod");
    assert!(trace_path.exists(), "derived per-run trace file exists");

    slod()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .success();

    slod()
        .args(["summary", "--file"])
        .arg(&trace_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("run_id: run-codex123"))
        .stdout(predicate::str::contains("status: imported"))
        .stdout(predicate::str::contains("model_calls: 1"))
        .stdout(predicate::str::contains("tool_calls: 2"))
        .stdout(predicate::str::contains("tool_results: 2"))
        .stdout(predicate::str::contains("tool_failures: 1"))
        .stdout(predicate::str::contains("errors: 1"));

    let content = fs::read_to_string(&trace_path).unwrap();
    assert!(content.contains(r#""format":"codex""#));
    assert!(content.contains(r#""tool":"exec_command""#));
    assert!(content.contains(r#""command":"cargo test""#));
    assert!(content.contains(r#""is_error":true"#));
    assert!(content.contains("synthetic transient failure"));

    let finished = last_event(&trace_path);
    assert_eq!(finished["kind"], "run.finished");
    assert_eq!(finished["payload"]["usage"]["input_tokens"], 10);
    assert_eq!(finished["payload"]["usage"]["output_tokens"], 5);
    assert_eq!(finished["payload"]["usage"]["cache_read_input_tokens"], 3);
}

#[test]
fn codex_import_refuses_existing_target_unless_forced() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("rollout.jsonl");
    fs::write(&input, CODEX_ROLLOUT).unwrap();
    let target = dir.path().join("imported.slod");

    slod()
        .args(["import", "--format", "codex", "--input"])
        .arg(&input)
        .arg("--file")
        .arg(&target)
        .assert()
        .success();
    let first_import = fs::read_to_string(&target).unwrap();

    slod()
        .args(["import", "--format", "codex", "--input"])
        .arg(&input)
        .arg("--file")
        .arg(&target)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
    assert_eq!(fs::read_to_string(&target).unwrap(), first_import);

    slod()
        .args(["import", "--format", "codex", "--force", "--input"])
        .arg(&input)
        .arg("--file")
        .arg(&target)
        .assert()
        .success();
}

#[test]
fn codex_import_clamps_bounds_to_min_and_max_transcript_timestamps() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("rollout.jsonl");
    fs::write(&input, OUT_OF_ORDER_CODEX_ROLLOUT).unwrap();
    let trace_path = dir.path().join("codex-clamp.slod");

    slod()
        .args(["import", "--format", "codex", "--input"])
        .arg(&input)
        .arg("--file")
        .arg(&trace_path)
        .assert()
        .success();

    let first = first_event(&trace_path);
    let last = last_event(&trace_path);
    assert_eq!(first["kind"], "run.started");
    assert_eq!(first["ts_ms"], serde_json::json!(CODEX_FIRST_TS_MS));
    assert_eq!(last["kind"], "run.finished");
    assert_eq!(last["ts_ms"], serde_json::json!(CODEX_FIRST_TS_MS + 10_000));
}

fn last_event(path: &std::path::Path) -> serde_json::Value {
    let content = fs::read_to_string(path).unwrap();
    let last = content
        .lines()
        .last()
        .expect("trace has at least one event");
    serde_json::from_str(last).expect("last event parses as JSON")
}
