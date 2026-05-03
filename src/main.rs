use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::{self, Command as ProcessCommand},
    time::Instant,
};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use traceframe::{
    render,
    trace::{EventKind, Trace},
};

const OUTPUT_PREVIEW_CHARS: usize = 4_000;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a new trace file with a run.started event.
    Init {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Append one structured event to a trace.
    Record {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        payload: String,
    },
    /// Close a trace with a run.finished event.
    Finish {
        #[arg(long)]
        file: PathBuf,
        #[arg(long, default_value = "success")]
        status: String,
        #[arg(long)]
        summary: Option<String>,
    },
    /// Run a command and append tool.call/tool.result events.
    Exec {
        #[arg(long)]
        file: PathBuf,
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Validate schema and ordering invariants.
    Verify {
        #[arg(long)]
        file: PathBuf,
    },
    /// Print ordered events for terminal inspection.
    Inspect {
        #[arg(long)]
        file: PathBuf,
    },
    /// Print a compact run summary.
    Summary {
        #[arg(long)]
        file: PathBuf,
    },
    /// Render a standalone HTML report.
    Render {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        html: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init {
            file,
            run_id,
            force,
        } => {
            Trace::init(&file, &run_id, force)?;
            print_action(
                "init",
                &[
                    ("file", file.display().to_string()),
                    ("run_id", run_id),
                    ("status", "started".to_string()),
                ],
            );
        }
        Command::Record {
            file,
            kind,
            payload,
        } => {
            let kind = kind.parse::<EventKind>()?;
            if kind == EventKind::RunStarted {
                bail!("run.started is created by traceframe init");
            }
            let payload = parse_payload(&payload)?;
            let event = Trace::append(&file, kind, payload)?;
            print_action(
                "record",
                &[
                    ("file", file.display().to_string()),
                    ("kind", event.kind.to_string()),
                    ("seq", event.seq.to_string()),
                ],
            );
        }
        Command::Finish {
            file,
            status,
            summary,
        } => {
            let mut payload = json!({ "status": status });
            if let Some(summary) = summary {
                payload["summary"] = Value::String(summary);
            }
            let event = Trace::append(&file, EventKind::RunFinished, payload)?;
            print_action(
                "finish",
                &[
                    ("file", file.display().to_string()),
                    (
                        "status",
                        event
                            .payload
                            .get("status")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown")
                            .to_string(),
                    ),
                    ("seq", event.seq.to_string()),
                ],
            );
        }
        Command::Exec { file, command } => {
            let exit_code = exec_command(&file, &command)?;
            if exit_code != 0 {
                process::exit(exit_code);
            }
        }
        Command::Verify { file } => {
            let trace = Trace::read(&file)?;
            trace.verify()?;
            print_action(
                "verify",
                &[
                    ("file", file.display().to_string()),
                    ("result", "valid trace".to_string()),
                ],
            );
        }
        Command::Inspect { file } => {
            let trace = Trace::read(&file)?;
            trace.verify()?;
            print!("{}", trace.inspect());
        }
        Command::Summary { file } => {
            let trace = Trace::read(&file)?;
            trace.verify()?;
            print!("{}", trace.summary().render_text());
        }
        Command::Render { file, html } => {
            let trace = Trace::read(&file)?;
            trace.verify()?;
            render::write_html(&trace, &html)?;
            print_action(
                "render",
                &[
                    ("file", file.display().to_string()),
                    ("html", html.display().to_string()),
                ],
            );
        }
    }

    Ok(())
}

fn parse_payload(payload: &str) -> Result<Value> {
    serde_json::from_str(payload).with_context(|| format!("invalid JSON payload: {payload}"))
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

fn print_action(action: &str, fields: &[(&str, String)]) {
    println!("{}", format_action(action, fields));
}

fn eprint_action(action: &str, fields: &[(&str, String)]) {
    eprintln!("{}", format_action(action, fields));
}

fn format_action(action: &str, fields: &[(&str, String)]) -> String {
    let mut output = format!("traceframe {action}\n");
    for (label, value) in fields {
        output.push_str(&format!("  {label:<11} {value}\n"));
    }
    output
}
