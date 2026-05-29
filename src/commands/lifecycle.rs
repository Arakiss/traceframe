//! Lifecycle commands: init, record, finish, exec, run.

use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::{self, Command as ProcessCommand},
    time::Instant,
};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use time::{OffsetDateTime, macros::format_description};
use traceframe::trace::{EventKind, Trace};

use super::{OUTPUT_PREVIEW_CHARS, eprint_action, parse_payload, print_action};

pub(crate) fn init(file: &Path, run_id: &str, force: bool) -> Result<()> {
    Trace::init(file, run_id, force)?;
    print_action(
        "init",
        &[
            ("file", file.display().to_string()),
            ("run_id", run_id.to_string()),
            ("status", "started".to_string()),
        ],
    );
    Ok(())
}

pub(crate) fn record(file: &Path, kind: &str, payload: &str) -> Result<()> {
    let kind = kind.parse::<EventKind>()?;
    if kind == EventKind::RunStarted {
        bail!("run.started is created by traceframe init");
    }
    let payload = parse_payload(payload)?;
    let event = Trace::append(file, kind, payload)?;
    print_action(
        "record",
        &[
            ("file", file.display().to_string()),
            ("kind", event.kind.to_string()),
            ("seq", event.seq.to_string()),
        ],
    );
    Ok(())
}

pub(crate) fn finish(file: &Path, status: &str, summary: Option<&str>) -> Result<()> {
    finish_trace(file, status, summary)
}

pub(crate) fn exec(file: &Path, command: &[String]) -> Result<()> {
    let exit_code = exec_command(file, command)?;
    if exit_code != 0 {
        process::exit(exit_code);
    }
    Ok(())
}

pub(crate) fn run(
    file: Option<PathBuf>,
    run_id: Option<String>,
    force: bool,
    command: &[String],
) -> Result<()> {
    let run_id = run_id.unwrap_or_else(|| default_run_id(command));
    let file = file.unwrap_or_else(|| default_trace_path(&run_id));
    Trace::init(&file, &run_id, force)?;
    print_action(
        "run",
        &[
            ("file", file.display().to_string()),
            ("run_id", run_id.clone()),
            ("command", command.join(" ")),
        ],
    );

    let exit_code = match exec_command(&file, command) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            let _ = finish_trace(&file, "failed", Some("command execution failed"));
            return Err(error);
        }
    };

    let status = if exit_code == 0 { "success" } else { "failed" };
    finish_trace(&file, status, Some("traceframe run completed"))?;
    if exit_code != 0 {
        process::exit(exit_code);
    }
    Ok(())
}

fn finish_trace(file: &Path, status: &str, summary: Option<&str>) -> Result<()> {
    let mut payload = json!({ "status": status });
    if let Some(summary) = summary {
        payload["summary"] = Value::String(summary.to_string());
    }
    let event = Trace::append(file, EventKind::RunFinished, payload)?;
    print_action(
        "finish",
        &[
            ("file", file.display().to_string()),
            ("status", status.to_string()),
            ("seq", event.seq.to_string()),
        ],
    );
    Ok(())
}

fn exec_command(file: &Path, command: &[String]) -> Result<i32> {
    let Some(program) = command.first() else {
        bail!("missing command after --");
    };
    let command_text = command.join(" ");

    let call_event = Trace::append(
        file,
        EventKind::ToolCall,
        json!({
            "tool": "shell",
            "command": &command_text,
            "argv": command,
        }),
    )?;

    let started = Instant::now();
    let output = match ProcessCommand::new(program).args(&command[1..]).output() {
        Ok(output) => output,
        Err(error) => {
            let duration_ms = elapsed_ms(started);
            Trace::append(
                file,
                EventKind::ToolResult,
                json!({
                    "tool": "shell",
                    "command": &command_text,
                    "argv": command,
                    "success": false,
                    "exit_code": null,
                    "duration_ms": duration_ms,
                    "error": error.to_string(),
                }),
            )?;
            Trace::append(
                file,
                EventKind::Error,
                json!({
                    "message": "failed to execute command",
                    "command": &command_text,
                    "error": error.to_string(),
                }),
            )?;
            bail!("failed to execute command: {command_text}: {error}");
        }
    };
    let duration_ms = elapsed_ms(started);

    io::stdout()
        .write_all(&output.stdout)
        .context("failed to forward command stdout")?;
    io::stderr()
        .write_all(&output.stderr)
        .context("failed to forward command stderr")?;

    let exit_code = output.status.code().unwrap_or(1);
    let event = Trace::append(
        file,
        EventKind::ToolResult,
        json!({
            "tool": "shell",
            "command": &command_text,
            "argv": command,
            "success": output.status.success(),
            "exit_code": output.status.code(),
            "duration_ms": duration_ms,
            "stdout_bytes": output.stdout.len(),
            "stderr_bytes": output.stderr.len(),
            "stdout_preview": preview_output(&output.stdout),
            "stderr_preview": preview_output(&output.stderr),
        }),
    )?;

    eprint_action(
        "exec",
        &[
            ("file", file.display().to_string()),
            ("command", command_text),
            (
                "result",
                if output.status.success() {
                    "success".to_string()
                } else {
                    "failed".to_string()
                },
            ),
            ("exit_code", exit_code.to_string()),
            ("duration_ms", duration_ms.to_string()),
            (
                "events",
                format!("tool.call#{} -> tool.result#{}", call_event.seq, event.seq),
            ),
        ],
    );
    Ok(exit_code)
}

fn preview_output(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut preview = String::new();
    let mut chars = text.chars();

    for _ in 0..OUTPUT_PREVIEW_CHARS {
        let Some(ch) = chars.next() else {
            return preview;
        };
        preview.push(ch);
    }

    if chars.next().is_some() {
        preview.push_str("\n[truncated]");
    }

    preview
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn default_trace_path(run_id: &str) -> PathBuf {
    PathBuf::from(".traceframe")
        .join("runs")
        .join(format!("{run_id}.traceframe"))
}

fn default_run_id(command: &[String]) -> String {
    let timestamp = OffsetDateTime::now_utc()
        .format(format_description!(
            "[year][month][day]T[hour][minute][second]Z"
        ))
        .unwrap_or_else(|_| OffsetDateTime::now_utc().unix_timestamp().to_string());
    let command_slug = command
        .first()
        .map(|program| slugify(program))
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| "command".to_string());
    format!("{timestamp}-{command_slug}")
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}
