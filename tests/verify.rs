use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

mod common;
use common::*;

#[test]
fn verify_rejects_malformed_trace_file() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("bad.slod");
    fs::write(&trace_path, "{bad}\n").unwrap();

    slod()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid trace event"));
}

#[test]
fn verify_rejects_trace_without_finished_event() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("sample.slod");
    slod()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-demo"])
        .assert()
        .success();

    slod()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("last event must be run.finished"));
}

#[test]
fn verify_allow_open_rejects_finished_event_before_end() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("bad-order.slod");
    fs::write(
        &trace_path,
        r#"{"version":1,"run_id":"run-bad","event_id":"e0","kind":"run.started","ts_ms":1,"seq":0,"payload":{}}
{"version":1,"run_id":"run-bad","event_id":"e1","kind":"run.finished","ts_ms":2,"seq":1,"payload":{"status":"success"}}
{"version":1,"run_id":"run-bad","event_id":"e2","kind":"tool.call","ts_ms":3,"seq":2,"payload":{"tool":"shell"}}
"#,
    )
    .unwrap();

    slod()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .args(["--allow-open"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("run.finished must be last"));
}

#[test]
fn open_trace_can_be_summarized_inspected_rendered_and_optionally_verified() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("open.slod");
    let html_path = dir.path().join("open.html");

    slod()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-open"])
        .assert()
        .success();

    slod()
        .args(["summary", "--file"])
        .arg(&trace_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("run_id: run-open"))
        .stdout(predicate::str::contains("status: open"));

    slod()
        .args(["inspect", "--file"])
        .arg(&trace_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("run.started"));

    slod()
        .args(["render", "--file"])
        .arg(&trace_path)
        .args(["--html"])
        .arg(&html_path)
        .assert()
        .success();

    slod()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .args(["--allow-open"])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid open trace"));

    let rendered = fs::read_to_string(html_path).unwrap();
    assert!(rendered.contains("run-open"));
}
