use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

mod common;
use common::*;

#[test]
fn ledger_rebuild_lists_filters_and_shows_runs() {
    let dir = tempdir().unwrap();
    let runs_dir = dir.path().join("runs");
    fs::create_dir(&runs_dir).unwrap();
    let success_path = runs_dir.join("success.slod");
    let failed_path = runs_dir.join("failed.slod");
    let ledger_path = dir.path().join("ledger.slod");

    write_trace_with_status(&success_path, "run-success", "success");
    write_trace_with_status(&failed_path, "run-failed", "failed");

    slod()
        .args(["ledger", "rebuild", "--dir"])
        .arg(&runs_dir)
        .args(["--out"])
        .arg(&ledger_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("slod ledger rebuild"))
        .stdout(predicate::str::contains("entries     2"));

    assert!(ledger_path.exists());

    slod()
        .args(["ledger", "list", "--file"])
        .arg(&ledger_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("run_id"))
        .stdout(predicate::str::contains("run-success"))
        .stdout(predicate::str::contains("run-failed"));

    slod()
        .args(["ledger", "list", "--file"])
        .arg(&ledger_path)
        .args(["--status", "failed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("run-failed"))
        .stdout(predicate::str::contains("run-success").not());

    slod()
        .args(["ledger", "show", "--file"])
        .arg(&ledger_path)
        .args(["--run-id", "run-success"])
        .assert()
        .success()
        .stdout(predicate::str::contains("run_id: run-success"))
        .stdout(predicate::str::contains("status: success"))
        .stdout(predicate::str::contains("trace_path:"));
}

#[test]
fn ledger_indexes_open_traces() {
    let dir = tempdir().unwrap();
    let runs_dir = dir.path().join("runs");
    fs::create_dir(&runs_dir).unwrap();
    let open_path = runs_dir.join("open.slod");
    let ledger_path = dir.path().join("ledger.slod");

    slod()
        .args(["init", "--file"])
        .arg(&open_path)
        .args(["--run-id", "run-open"])
        .assert()
        .success();

    slod()
        .args(["ledger", "rebuild", "--dir"])
        .arg(&runs_dir)
        .args(["--out"])
        .arg(&ledger_path)
        .assert()
        .success();

    slod()
        .args(["ledger", "show", "--file"])
        .arg(&ledger_path)
        .args(["--run-id", "run-open"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status: open"))
        .stdout(predicate::str::contains("finished_ms: open"));
}

#[test]
fn ledger_show_reports_missing_run() {
    let dir = tempdir().unwrap();
    let runs_dir = dir.path().join("runs");
    fs::create_dir(&runs_dir).unwrap();
    let trace_path = runs_dir.join("sample.slod");
    let ledger_path = dir.path().join("ledger.slod");

    write_trace_with_status(&trace_path, "run-present", "success");

    slod()
        .args(["ledger", "rebuild", "--dir"])
        .arg(&runs_dir)
        .args(["--out"])
        .arg(&ledger_path)
        .assert()
        .success();

    slod()
        .args(["ledger", "show", "--file"])
        .arg(&ledger_path)
        .args(["--run-id", "run-missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "run not found in ledger: run-missing",
        ));
}

#[test]
fn ledger_rebuild_rejects_malformed_trace_file() {
    let dir = tempdir().unwrap();
    let runs_dir = dir.path().join("runs");
    fs::create_dir(&runs_dir).unwrap();
    let trace_path = runs_dir.join("bad.slod");
    let ledger_path = dir.path().join("ledger.slod");
    fs::write(&trace_path, "{bad}\n").unwrap();

    slod()
        .args(["ledger", "rebuild", "--dir"])
        .arg(&runs_dir)
        .args(["--out"])
        .arg(&ledger_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid trace event"));
}
