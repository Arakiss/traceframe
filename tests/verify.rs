use std::{
    fs::{self, OpenOptions},
    io::Write,
    process::{Command as StdCommand, Stdio},
    thread,
    time::Duration,
};

use predicates::prelude::*;
use serde_json::json;
use slod::trace::{Event, EventKind};
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
fn verify_missing_trace_preserves_read_error_without_creating_a_lock() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("missing.slod");

    slod()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to read trace file"));

    assert!(!trace_path.with_extension("slod.lock").exists());
}

#[test]
fn read_commands_wait_for_an_in_progress_append() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("concurrent-read.slod");

    slod()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-concurrent-read"])
        .assert()
        .success();

    let lock_path = trace_path.with_extension("slod.lock");
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .unwrap();
    lock_file.lock().unwrap();

    let event = Event::new(
        "run-concurrent-read",
        EventKind::ToolCall,
        1,
        json!({"tool":"shell","command":"cargo test"}),
    );
    let mut encoded = serde_json::to_vec(&event).unwrap();
    encoded.push(b'\n');
    let split = encoded.len() / 2;
    let mut trace_file = OpenOptions::new().append(true).open(&trace_path).unwrap();
    trace_file.write_all(&encoded[..split]).unwrap();
    trace_file.flush().unwrap();

    let partial = fs::read_to_string(&trace_path).unwrap();
    assert!(
        serde_json::from_str::<Event>(partial.lines().last().unwrap()).is_err(),
        "fixture must expose a transient partial JSON row while the writer owns the lock"
    );

    let reader_specs: [(&str, &[&str]); 3] = [
        ("verify", &["verify", "--allow-open", "--file"]),
        ("summary", &["summary", "--file"]),
        ("inspect", &["inspect", "--file"]),
    ];
    let mut readers = reader_specs
        .into_iter()
        .map(|(name, args)| {
            let child = StdCommand::new(env!("CARGO_BIN_EXE_slod"))
                .args(args)
                .arg(&trace_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn trace reader");
            (name, child)
        })
        .collect::<Vec<_>>();

    thread::sleep(Duration::from_millis(250));
    let completed_while_partial = readers
        .iter_mut()
        .filter_map(|(name, child)| child.try_wait().unwrap().map(|status| (*name, status)))
        .collect::<Vec<_>>();

    trace_file.write_all(&encoded[split..]).unwrap();
    trace_file.flush().unwrap();
    drop(trace_file);
    drop(lock_file);

    let outputs = readers
        .into_iter()
        .map(|(name, child)| {
            (
                name,
                child.wait_with_output().expect("wait for trace reader"),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        completed_while_partial.is_empty(),
        "readers observed the partial append: {completed_while_partial:?}"
    );
    for (name, output) in outputs {
        assert!(
            output.status.success(),
            "{name} failed after the append completed\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    slod()
        .args(["finish", "--file"])
        .arg(&trace_path)
        .args(["--status", "success"])
        .assert()
        .success();
    slod()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .success();
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
