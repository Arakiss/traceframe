use std::{fs, path::Path};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn traceframe() -> Command {
    Command::cargo_bin("traceframe").expect("traceframe binary")
}

fn write_valid_trace(path: &Path) {
    write_trace_with_status(path, "run-demo", "success");
}

fn write_trace_with_status(path: &Path, run_id: &str, status: &str) {
    traceframe()
        .args(["init", "--file"])
        .arg(path)
        .args(["--run-id", run_id])
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
        .args([
            "--kind",
            "tool.result",
            "--payload",
            &format!(
                r#"{{"exit_code":{},"success":{}}}"#,
                if status == "success" { 0 } else { 1 },
                status == "success"
            ),
        ])
        .assert()
        .success();

    if status != "success" {
        traceframe()
            .args(["record", "--file"])
            .arg(path)
            .args([
                "--kind",
                "error",
                "--payload",
                r#"{"message":"simulated failure"}"#,
            ])
            .assert()
            .success();
    }

    traceframe()
        .args(["finish", "--file"])
        .arg(path)
        .args(["--status", status])
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
fn verify_allow_open_rejects_finished_event_before_end() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("bad-order.traceframe");
    fs::write(
        &trace_path,
        r#"{"version":1,"run_id":"run-bad","event_id":"e0","kind":"run.started","ts_ms":1,"seq":0,"payload":{}}
{"version":1,"run_id":"run-bad","event_id":"e1","kind":"run.finished","ts_ms":2,"seq":1,"payload":{"status":"success"}}
{"version":1,"run_id":"run-bad","event_id":"e2","kind":"tool.call","ts_ms":3,"seq":2,"payload":{"tool":"shell"}}
"#,
    )
    .unwrap();

    traceframe()
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
fn plain_relative_output_paths_work() {
    let dir = tempdir().unwrap();

    traceframe()
        .current_dir(dir.path())
        .args([
            "run",
            "--file",
            "run.traceframe",
            "--run-id",
            "plain-run",
            "--",
            "cargo",
            "--version",
        ])
        .assert()
        .success();

    traceframe()
        .current_dir(dir.path())
        .args([
            "render",
            "--file",
            "run.traceframe",
            "--html",
            "report.html",
        ])
        .assert()
        .success();

    let runs_dir = dir.path().join("runs");
    fs::create_dir(&runs_dir).unwrap();
    fs::copy(
        dir.path().join("run.traceframe"),
        runs_dir.join("plain-run.traceframe"),
    )
    .unwrap();

    traceframe()
        .current_dir(dir.path())
        .args([
            "ledger",
            "rebuild",
            "--dir",
            "runs",
            "--out",
            "ledger.traceframe",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("entries     1"));

    assert!(dir.path().join("run.traceframe").exists());
    assert!(dir.path().join("report.html").exists());
    assert!(dir.path().join("ledger.traceframe").exists());
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

#[test]
fn run_closes_trace_when_command_cannot_spawn() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("spawn-failed.traceframe");

    traceframe()
        .args(["run", "--file"])
        .arg(&trace_path)
        .args([
            "--run-id",
            "run-spawn-failed",
            "--",
            "traceframe-test-command-that-should-not-exist",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("traceframe finish"))
        .stderr(predicate::str::contains("failed to execute command"));

    traceframe()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .success();

    let trace = fs::read_to_string(trace_path).unwrap();
    assert!(trace.contains(r#""kind":"tool.result""#));
    assert!(trace.contains(r#""kind":"error""#));
    assert!(trace.contains(r#""kind":"run.finished""#));
    assert!(trace.contains(r#""status":"failed""#));
}

#[test]
fn hook_ingest_can_initialize_missing_trace_and_record_codex_tool_events() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("hook.traceframe");

    traceframe()
        .args([
            "hook",
            "ingest",
            "--source",
            "codex",
            "--run-id",
            "hook-run",
            "--init-if-missing",
            "--file",
        ])
        .arg(&trace_path)
        .write_stdin(
            r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"cargo test"},"session_id":"codex-session"}"#,
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("traceframe hook ingest"))
        .stdout(predicate::str::contains("tool.call#1"));

    traceframe()
        .args(["hook", "ingest", "--source", "codex", "--file"])
        .arg(&trace_path)
        .write_stdin(
            r#"{"hook_event_name":"PostToolUse","tool_name":"Bash","tool_response":{"success":true,"exit_code":0,"stdout":"ok"}}"#,
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("tool.result#2"));

    traceframe()
        .args(["summary", "--file"])
        .arg(&trace_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("run_id: hook-run"))
        .stdout(predicate::str::contains("tool_calls: 1"))
        .stdout(predicate::str::contains("tool_results: 1"));

    traceframe()
        .args(["finish", "--file"])
        .arg(&trace_path)
        .args(["--status", "success"])
        .assert()
        .success();

    traceframe()
        .args(["verify", "--file"])
        .arg(&trace_path)
        .assert()
        .success();

    let trace = fs::read_to_string(trace_path).unwrap();
    assert!(trace.contains(r#""source":"codex""#));
    assert!(trace.contains(r#""hook_event":"PreToolUse""#));
    assert!(trace.contains(r#""session_id":"codex-session""#));
}

#[test]
fn hook_ingest_records_permission_and_error_events() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("hook.traceframe");

    traceframe()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "hook-run"])
        .assert()
        .success();

    traceframe()
        .args(["hook", "ingest", "--source", "omx", "--file"])
        .arg(&trace_path)
        .write_stdin(
            r#"{"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{"command":"edit README.md"},"decision":"allow"}"#,
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("permission.decision#1"));

    traceframe()
        .args(["hook", "ingest", "--source", "generic", "--file"])
        .arg(&trace_path)
        .write_stdin(
            r#"{"hook_event_name":"HookError","tool_name":"Bash","error":{"message":"hook failed"}}"#,
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("error#2"));

    traceframe()
        .args(["inspect", "--file"])
        .arg(&trace_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("permission.decision"))
        .stdout(predicate::str::contains("error"));
}

#[test]
fn hook_ingest_rejects_missing_trace_without_init_flag() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("missing.traceframe");

    traceframe()
        .args(["hook", "ingest", "--file"])
        .arg(&trace_path)
        .write_stdin(r#"{"hook_event_name":"PreToolUse","tool_name":"Bash"}"#)
        .assert()
        .failure()
        .stderr(predicate::str::contains("trace does not exist"));
}

#[test]
fn hook_ingest_rejects_empty_stdin() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("hook.traceframe");

    traceframe()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "hook-run"])
        .assert()
        .success();

    traceframe()
        .args(["hook", "ingest", "--file"])
        .arg(&trace_path)
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing hook JSON on stdin"));
}

#[test]
fn ledger_rebuild_lists_filters_and_shows_runs() {
    let dir = tempdir().unwrap();
    let runs_dir = dir.path().join("runs");
    fs::create_dir(&runs_dir).unwrap();
    let success_path = runs_dir.join("success.traceframe");
    let failed_path = runs_dir.join("failed.traceframe");
    let ledger_path = dir.path().join("ledger.traceframe");

    write_trace_with_status(&success_path, "run-success", "success");
    write_trace_with_status(&failed_path, "run-failed", "failed");

    traceframe()
        .args(["ledger", "rebuild", "--dir"])
        .arg(&runs_dir)
        .args(["--out"])
        .arg(&ledger_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("traceframe ledger rebuild"))
        .stdout(predicate::str::contains("entries     2"));

    assert!(ledger_path.exists());

    traceframe()
        .args(["ledger", "list", "--file"])
        .arg(&ledger_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("run_id"))
        .stdout(predicate::str::contains("run-success"))
        .stdout(predicate::str::contains("run-failed"));

    traceframe()
        .args(["ledger", "list", "--file"])
        .arg(&ledger_path)
        .args(["--status", "failed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("run-failed"))
        .stdout(predicate::str::contains("run-success").not());

    traceframe()
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
    let open_path = runs_dir.join("open.traceframe");
    let ledger_path = dir.path().join("ledger.traceframe");

    traceframe()
        .args(["init", "--file"])
        .arg(&open_path)
        .args(["--run-id", "run-open"])
        .assert()
        .success();

    traceframe()
        .args(["ledger", "rebuild", "--dir"])
        .arg(&runs_dir)
        .args(["--out"])
        .arg(&ledger_path)
        .assert()
        .success();

    traceframe()
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
    let trace_path = runs_dir.join("sample.traceframe");
    let ledger_path = dir.path().join("ledger.traceframe");

    write_trace_with_status(&trace_path, "run-present", "success");

    traceframe()
        .args(["ledger", "rebuild", "--dir"])
        .arg(&runs_dir)
        .args(["--out"])
        .arg(&ledger_path)
        .assert()
        .success();

    traceframe()
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
    let trace_path = runs_dir.join("bad.traceframe");
    let ledger_path = dir.path().join("ledger.traceframe");
    fs::write(&trace_path, "{bad}\n").unwrap();

    traceframe()
        .args(["ledger", "rebuild", "--dir"])
        .arg(&runs_dir)
        .args(["--out"])
        .arg(&ledger_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid trace event"));
}
