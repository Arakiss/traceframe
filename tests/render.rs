use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

mod common;
use common::*;

#[test]
fn render_html_report_highlights_tools_states_and_permissions() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("rich.traceframe");
    let html_path = dir.path().join("rich.html");

    traceframe()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-render"])
        .assert()
        .success();

    traceframe()
        .args(["record", "--file"])
        .arg(&trace_path)
        .args([
            "--kind",
            "model.call",
            "--payload",
            r#"{"provider":"openai","model":"gpt-5.5"}"#,
        ])
        .assert()
        .success();

    traceframe()
        .args(["record", "--file"])
        .arg(&trace_path)
        .args([
            "--kind",
            "permission.decision",
            "--payload",
            r#"{"capability":"git.push","decision":"deny"}"#,
        ])
        .assert()
        .success();

    traceframe()
        .args(["record", "--file"])
        .arg(&trace_path)
        .args([
            "--kind",
            "tool.call",
            "--payload",
            r#"{"tool":"shell","command":"git push origin main","argv":["git","push","origin","main"]}"#,
        ])
        .assert()
        .success();

    traceframe()
        .args(["record", "--file"])
        .arg(&trace_path)
        .args([
            "--kind",
            "tool.result",
            "--payload",
            r#"{"tool":"shell","command":"git push origin main","success":false,"exit_code":1,"duration_ms":1500,"stdout_bytes":0,"stderr_bytes":24,"stderr_preview":"denied by policy"}"#,
        ])
        .assert()
        .success();

    traceframe()
        .args(["record", "--file"])
        .arg(&trace_path)
        .args([
            "--kind",
            "error",
            "--payload",
            r#"{"message":"push blocked"}"#,
        ])
        .assert()
        .success();

    traceframe()
        .args(["finish", "--file"])
        .arg(&trace_path)
        .args(["--status", "failed", "--summary", "run blocked"])
        .assert()
        .success();

    traceframe()
        .args(["render", "--file"])
        .arg(&trace_path)
        .args(["--html"])
        .arg(&html_path)
        .assert()
        .success();

    let html = fs::read_to_string(&html_path).unwrap();
    // Header and identity.
    assert!(html.contains("traceframe report"));
    assert!(html.contains("run-render"));
    // Timeline with tool detail.
    assert!(html.contains("Timeline"));
    assert!(html.contains("git push origin main"));
    assert!(html.contains("exit 1"));
    assert!(html.contains("1.50 s"));
    // Permission decision and error surfaced with state colours.
    assert!(html.contains("deny"));
    assert!(html.contains("denied by policy"));
    assert!(html.contains("push blocked"));
    assert!(html.contains("ev-bad"));
    // Metric cards.
    assert!(html.contains("tool failures"));

    // The compact summary gains failure and human-duration clarity.
    traceframe()
        .args(["summary", "--file"])
        .arg(&trace_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("status: failed"))
        .stdout(predicate::str::contains("tool_failures: 1"))
        .stdout(predicate::str::contains("duration:"));
}
