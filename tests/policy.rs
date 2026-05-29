use predicates::prelude::*;
use tempfile::tempdir;

mod common;
use common::*;

#[test]
fn policy_check_passes_on_clean_trace() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("clean.traceframe");

    traceframe()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-clean"])
        .assert()
        .success();

    traceframe()
        .args(["record", "--file"])
        .arg(&trace_path)
        .args([
            "--kind",
            "permission.decision",
            "--payload",
            r#"{"capability":"git.push","decision":"allow"}"#,
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
            r#"{"tool":"shell","command":"git push origin main"}"#,
        ])
        .assert()
        .success();

    traceframe()
        .args(["finish", "--file"])
        .arg(&trace_path)
        .args(["--status", "success"])
        .assert()
        .success();

    traceframe()
        .args(["policy-check", "--file"])
        .arg(&trace_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("result      clean"));
}

#[test]
fn policy_check_fails_on_unresolved_deny() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("deny.traceframe");

    traceframe()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-deny"])
        .assert()
        .success();

    traceframe()
        .args(["record", "--file"])
        .arg(&trace_path)
        .args([
            "--kind",
            "permission.decision",
            "--payload",
            r#"{"capability":"fs.write:secrets","decision":"deny"}"#,
        ])
        .assert()
        .success();

    traceframe()
        .args(["finish", "--file"])
        .arg(&trace_path)
        .args(["--status", "failed"])
        .assert()
        .success();

    traceframe()
        .args(["policy-check", "--file"])
        .arg(&trace_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("unresolved deny"))
        .stderr(predicate::str::contains("fs.write:secrets"));
}

#[test]
fn policy_check_fails_on_git_push_without_allow() {
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("push.traceframe");

    traceframe()
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-push"])
        .assert()
        .success();

    traceframe()
        .args(["record", "--file"])
        .arg(&trace_path)
        .args([
            "--kind",
            "tool.call",
            "--payload",
            r#"{"tool":"shell","command":"git push --force origin main"}"#,
        ])
        .assert()
        .success();

    traceframe()
        .args(["finish", "--file"])
        .arg(&trace_path)
        .args(["--status", "success"])
        .assert()
        .success();

    traceframe()
        .args(["policy-check", "--file"])
        .arg(&trace_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("sensitive tool.call"));
}
