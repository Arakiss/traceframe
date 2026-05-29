use tempfile::tempdir;

mod common;
use common::*;

#[test]
fn init_run_started_carries_cwd_and_git_context() {
    let dir = tempdir().unwrap();
    let repo = dir.path();
    init_git_repo(repo);

    let trace_path = repo.join("host.traceframe");
    traceframe()
        .current_dir(repo)
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-host"])
        .assert()
        .success();

    let started = first_event(&trace_path);
    assert_eq!(started["kind"], "run.started");

    let host = &started["payload"]["host"];
    let cwd = host["cwd"].as_str().expect("cwd recorded on run.started");
    let repo_name = repo.file_name().unwrap().to_str().unwrap();
    assert!(
        cwd.ends_with(repo_name),
        "cwd {cwd} should end with repo dir {repo_name}"
    );

    let git = &host["git"];
    let branch = git_capture(repo, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let commit = git_capture(repo, &["rev-parse", "--short", "HEAD"]);
    assert_eq!(git["branch"].as_str(), Some(branch.as_str()));
    assert_eq!(git["commit"].as_str(), Some(commit.as_str()));
    assert_eq!(git["repo"].as_str(), Some(repo_name));
}

#[test]
fn run_run_started_carries_host_context() {
    let dir = tempdir().unwrap();
    let repo = dir.path();
    init_git_repo(repo);

    let trace_path = repo.join("run-host.traceframe");
    traceframe()
        .current_dir(repo)
        .args(["run", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-host", "--", "cargo", "--version"])
        .assert()
        .success();

    let started = first_event(&trace_path);
    assert_eq!(started["kind"], "run.started");

    let host = &started["payload"]["host"];
    assert!(
        host["cwd"].as_str().is_some(),
        "run.started should record cwd"
    );
    let branch = git_capture(repo, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let commit = git_capture(repo, &["rev-parse", "--short", "HEAD"]);
    assert_eq!(host["git"]["branch"].as_str(), Some(branch.as_str()));
    assert_eq!(host["git"]["commit"].as_str(), Some(commit.as_str()));
}

#[test]
fn init_outside_git_repo_keeps_cwd_and_omits_git() {
    let dir = tempdir().unwrap();
    let workdir = dir.path();
    // No git init: the directory is not inside any repository.

    let trace_path = workdir.join("nogit.traceframe");
    traceframe()
        .current_dir(workdir)
        .args(["init", "--file"])
        .arg(&trace_path)
        .args(["--run-id", "run-nogit"])
        .assert()
        .success();

    let started = first_event(&trace_path);
    let host = &started["payload"]["host"];
    assert!(
        host["cwd"].as_str().is_some(),
        "cwd should still be recorded without git"
    );
    assert!(
        host.get("git").is_none(),
        "git context must be absent outside a repository"
    );
}
