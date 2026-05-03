mod render;
mod trace;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde_json::Value;
use trace::{EventKind, Trace};

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
            println!("created trace: {}", file.display());
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
            println!("recorded {} seq={}", event.kind, event.seq);
        }
        Command::Verify { file } => {
            let trace = Trace::read(&file)?;
            trace.verify()?;
            println!("valid trace: {}", file.display());
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
            println!("rendered html: {}", html.display());
        }
    }

    Ok(())
}

fn parse_payload(payload: &str) -> Result<Value> {
    serde_json::from_str(payload).with_context(|| format!("invalid JSON payload: {payload}"))
}
