use std::{fs, path::Path};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn traceframe() -> Command {
    Command::cargo_bin("traceframe").expect("traceframe binary")
}

fn write_valid_trace(path: &Path) {
    traceframe()
        .args(["init", "--file"])
        .arg(path)
        .args(["--run-id", "run-demo"])
        .assert()
        .success();

    traceframe()
        .args(["record", "--file"])
        .arg(path)
        .args([
            "--kind",
            "model.call",
            "--payload",
            r#"{"provider":"openai","model":"gpt"}"#,
        ])
        .assert()
        .success();

    traceframe()
        .args(["record", "--file"])
        .arg(path)
        .args([
            "--kind",
            "permission.decision",
            "--payload",
            r#"{"capability":"fs.write:README.md","decision":"allow"}"#,
        ])
        .assert()
        .success();

    traceframe()
        .args(["record", "--file"])
        .arg(path)
        .args([
            "--kind",
            "tool.call",
            "--payload",
            r#"{"tool":"shell","command":"cargo test"}"#,
        ])
        .assert()
        .success();

    traceframe()
        .args(["record", "--file"])
        .arg(path)
        .args(["--kind", "tool.result", "--payload", r#"{"exit_code":0}"#])
        .assert()
        .success();

    traceframe()
        .args(["finish", "--file"])
        .arg(path)
        .args(["--status", "success"])
        .assert()
        .success();
}

#[test]
fn cli_happy_path_creates_verifies_summarizes_inspects_and_renders() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("sample.traceframe");
    let html_path = dir.path().join("sample.html");

    write_valid_trace(&trace_path);

    traceframe()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("valid trace"));

    traceframe()
        .args(["summary", "--file"])
        .arg(&trace_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("run_id: run-demo"))
        .stdout(predicate::str::contains("status: success"))
        .stdout(predicate::str::contains("permission_decisions: 1"));

    traceframe()
        .args(["inspect", "--file"])
        .arg(&trace_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("tool.call"))
        .stdout(predicate::str::contains("permission.decision"));

    traceframe()
        .args(["render", "--file"])
        .arg(&trace_path)
        .args(["--html"])
        .arg(&html_path)
        .assert()
        .success();

    let rendered = fs::read_to_string(html_path).unwrap();
    assert!(rendered.contains("traceframe report"));
    assert!(rendered.contains("run-demo"));
}

#[test]
fn cli_rejects_malformed_payload() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("sample.traceframe");
    traceframe()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-demo"])
        .assert()
        .success();

    traceframe()
        .args(["record", "--file"])
        .arg(&trace_path)
        .args(["--kind", "model.call", "--payload", "{not-json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid JSON payload"));
}

#[test]
fn cli_rejects_unknown_event_kind() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("sample.traceframe");
    traceframe()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-demo"])
        .assert()
        .success();

    traceframe()
        .args(["record", "--file"])
        .arg(&trace_path)
        .args(["--kind", "agent.magic", "--payload", "{}"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown event kind"));
}

#[test]
fn verify_rejects_malformed_trace_file() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("bad.traceframe");
    fs::write(&trace_path, "{bad}\n").unwrap();

    traceframe()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid trace event"));
}

#[test]
fn verify_rejects_trace_without_finished_event() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("sample.traceframe");
    traceframe()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-demo"])
        .assert()
        .success();

    traceframe()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("last event must be run.finished"));
}

#[test]
fn open_trace_can_be_summarized_inspected_rendered_and_optionally_verified() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("open.traceframe");
    let html_path = dir.path().join("open.html");

    traceframe()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-open"])
        .assert()
        .success();

    traceframe()
        .args(["summary", "--file"])
        .arg(&trace_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("run_id: run-open"))
        .stdout(predicate::str::contains("status: open"));

    traceframe()
        .args(["inspect", "--file"])
        .arg(&trace_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("run.started"));

    traceframe()
        .args(["render", "--file"])
        .arg(&trace_path)
        .args(["--html"])
        .arg(&html_path)
        .assert()
        .success();

    traceframe()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .args(["--allow-open"])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid open trace"));

    let rendered = fs::read_to_string(html_path).unwrap();
    assert!(rendered.contains("run-open"));
}

#[test]
fn finish_closes_trace_without_manual_json_payload() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("sample.traceframe");

    traceframe()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-demo"])
        .assert()
        .success();

    traceframe()
        .args(["finish", "--file"])
        .arg(&trace_path)
        .args(["--status", "success", "--summary", "verified by CLI test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("traceframe finish"))
        .stdout(predicate::str::contains("status      success"));

    traceframe()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .success();

    let trace = fs::read_to_string(trace_path).unwrap();
    assert!(trace.contains(r#""kind":"run.finished""#));
    assert!(trace.contains(r#""summary":"verified by CLI test""#));
}

#[test]
fn exec_records_command_result_and_preserves_child_output() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("sample.traceframe");

    traceframe()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-demo"])
        .assert()
        .success();

    traceframe()
        .args(["exec", "--file"])
        .arg(&trace_path)
        .args(["--", "cargo", "--version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo"))
        .stderr(predicate::str::contains("traceframe exec"))
        .stderr(predicate::str::contains("tool.call#1 -> tool.result#2"));

    traceframe()
        .args(["finish", "--file"])
        .arg(&trace_path)
        .args(["--status", "success"])
        .assert()
        .success();

    traceframe()
        .args(["summary", "--file"])
        .arg(&trace_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("tool_calls: 1"))
        .stdout(predicate::str::contains("tool_results: 1"));

    let trace = fs::read_to_string(trace_path).unwrap();
    assert!(trace.contains(r#""kind":"tool.call""#));
    assert!(trace.contains(r#""kind":"tool.result""#));
    assert!(trace.contains(r#""argv":["cargo","--version"]"#));
    assert!(trace.contains(r#""success":true"#));
    assert!(trace.contains(r#""stdout_preview":"cargo "#));
}

#[test]
fn run_creates_executes_and_closes_trace() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("run.traceframe");

    traceframe()
        .args(["run", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-demo", "--", "cargo", "--version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo"))
        .stdout(predicate::str::contains("traceframe run"))
        .stdout(predicate::str::contains("traceframe finish"))
        .stderr(predicate::str::contains("traceframe exec"));

    traceframe()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .success();

    let trace = fs::read_to_string(trace_path).unwrap();
    assert!(trace.contains(r#""kind":"run.started""#));
    assert!(trace.contains(r#""kind":"tool.call""#));
    assert!(trace.contains(r#""kind":"tool.result""#));
    assert!(trace.contains(r#""kind":"run.finished""#));
    assert!(trace.contains(r#""status":"success""#));
}

#[test]
fn run_uses_default_trace_directory_when_file_is_omitted() {
    let dir = tempdir().unwrap();

    traceframe()
        .current_dir(dir.path())
        .args([
            "run",
            "--run-id",
            "default-demo",
            "--",
            "cargo",
            "--version",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            ".traceframe/runs/default-demo.traceframe",
        ));

    let trace_path = dir
        .path()
        .join(".traceframe")
        .join("runs")
        .join("default-demo.traceframe");
    assert!(trace_path.exists());

    traceframe()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .success();
}

#[test]
fn run_closes_trace_when_command_fails() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("failed-run.traceframe");

    traceframe()
        .args(["run", "--file"])
        .arg(&trace_path)
        .args([
            "--run-id",
            "run-failed",
            "--",
            "cargo",
            "--definitely-not-a-real-flag",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("traceframe finish"))
        .stderr(predicate::str::contains("traceframe exec"));

    traceframe()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .success();

    let trace = fs::read_to_string(trace_path).unwrap();
    assert!(trace.contains(r#""kind":"run.finished""#));
    assert!(trace.contains(r#""status":"failed""#));
}
