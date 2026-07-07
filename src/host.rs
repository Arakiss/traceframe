//! Best-effort host context for freshly created traces.
//!
//! When `slod` opens a `run.started` event it tags it with where the run
//! was born: the working directory and, when that directory lives inside a git
//! repository, the branch, short commit, and repository name. Detection is
//! optional by design — a missing git binary or a directory outside any repo
//! simply yields fewer fields and never blocks trace creation.

use std::{path::Path, process::Command};

use serde_json::{Map, Value};

/// Collect host context for a new `run.started` event.
///
/// Returns `None` when nothing could be detected, so callers can omit the
/// `host` field entirely instead of writing an empty object.
pub fn context() -> Option<Value> {
    let mut host = Map::new();

    if let Some(cwd) = current_dir() {
        host.insert("cwd".to_string(), Value::String(cwd));
    }

    let git = git_context();
    if !git.is_empty() {
        host.insert("git".to_string(), Value::Object(git));
    }

    if host.is_empty() {
        None
    } else {
        Some(Value::Object(host))
    }
}

fn current_dir() -> Option<String> {
    std::env::current_dir()
        .ok()
        .map(|path| path.display().to_string())
}

/// Best-effort git metadata for the current working directory. Each field is
/// independent: a repo with no commits yields a branch but no commit, and a
/// directory outside any repository yields an empty map.
fn git_context() -> Map<String, Value> {
    let mut git = Map::new();

    if let Some(branch) = git_output(&["rev-parse", "--abbrev-ref", "HEAD"]) {
        git.insert("branch".to_string(), Value::String(branch));
    }
    if let Some(commit) = git_output(&["rev-parse", "--short", "HEAD"]) {
        git.insert("commit".to_string(), Value::String(commit));
    }
    if let Some(repo) =
        git_output(&["rev-parse", "--show-toplevel"]).and_then(|top| repo_name(&top))
    {
        git.insert("repo".to_string(), Value::String(repo));
    }

    git
}

/// Run a git subcommand, returning its trimmed stdout only on success with
/// non-empty output. Any failure (git missing, not a repository, empty output)
/// degrades quietly to `None`.
fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn repo_name(toplevel: &str) -> Option<String> {
    Path::new(toplevel)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_name_takes_final_path_component() {
        assert_eq!(repo_name("/home/dev/slod").as_deref(), Some("slod"));
        assert_eq!(repo_name("/").as_deref(), None);
    }
}
