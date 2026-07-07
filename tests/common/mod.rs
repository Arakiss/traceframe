// Shared test helpers are compiled into every test binary, but each binary only
// exercises the subset it needs; allow the rest without dead-code warnings.
#![allow(dead_code)]

use std::{fs, path::Path, process::Command as StdCommand};

use assert_cmd::Command;
use serde_json::Value;

pub fn slod() -> Command {
    Command::cargo_bin("slod").expect("slod binary")
}

/// Parse the first newline-delimited event of a trace into JSON.
pub fn first_event(path: &Path) -> Value {
    let content = fs::read_to_string(path).unwrap();
    let first = content
        .lines()
        .next()
        .expect("trace has at least one event");
    serde_json::from_str(first).expect("first event parses as JSON")
}

/// Run a git command inside `repo`, isolated from the user's global/system git
/// config so test fixtures never pick up hooks, signing, or templates.
pub fn run_git(repo: &Path, args: &[&str]) {
    let status = StdCommand::new("git")
        .current_dir(repo)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .args(args)
        .status()
        .expect("git command runs");
    assert!(status.success(), "git {args:?} failed");
}

/// Capture trimmed stdout of a read-only git command inside `repo`.
pub fn git_capture(repo: &Path, args: &[&str]) -> String {
    let output = StdCommand::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .expect("git command runs");
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

/// Initialize a throwaway git repo with one commit so branch/commit resolve.
pub fn init_git_repo(repo: &Path) {
    run_git(repo, &["init"]);
    run_git(repo, &["config", "user.email", "test@example.com"]);
    run_git(repo, &["config", "user.name", "Slod Test"]);
    fs::write(repo.join("README.md"), "host context fixture\n").unwrap();
    run_git(repo, &["add", "README.md"]);
    run_git(repo, &["commit", "-m", "chore: seed host context fixture"]);
}

pub fn write_valid_trace(path: &Path) {
    write_trace_with_status(path, "run-demo", "success");
}

pub fn write_trace_with_status(path: &Path, run_id: &str, status: &str) {
    slod()
        .args(["init", "--file"])
        .arg(path)
        .args(["--run-id", run_id])
        .assert()
        .success();

    slod()
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

    slod()
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

    slod()
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

    slod()
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
        slod()
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

    slod()
        .args(["finish", "--file"])
        .arg(path)
        .args(["--status", status])
        .assert()
        .success();
}
