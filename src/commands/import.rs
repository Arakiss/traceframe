//! Import command: backfill traces from harness-native transcripts.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use slod::import::{parse_claude_code, parse_codex, write_trace};

use super::print_action;

#[allow(clippy::too_many_arguments)]
pub(crate) fn import(
    format: &str,
    input: &Path,
    file: Option<&Path>,
    dir: Option<&Path>,
    source: Option<&str>,
    run_id: Option<&str>,
    force: bool,
) -> Result<()> {
    if file.is_some() && dir.is_some() {
        bail!("pass at most one of --file or --dir");
    }

    let source = source.unwrap_or(format);
    let imported = match format {
        "claude-code" => parse_claude_code(input, run_id, source)?,
        "codex" => parse_codex(input, run_id, source)?,
        _ => bail!("unsupported --format {format}; supported formats: claude-code, codex"),
    };

    let target: PathBuf = match file {
        Some(file) => file.to_path_buf(),
        None => dir
            .unwrap_or(Path::new(".slod/runs"))
            .join(format!("{}.slod", imported.run_id)),
    };
    write_trace(&target, &imported.events, force)?;

    let stats = &imported.stats;
    print_action(
        "import",
        &[
            ("file", target.display().to_string()),
            ("run_id", imported.run_id.clone()),
            ("format", format.to_string()),
            ("events", imported.events.len().to_string()),
            ("model_calls", stats.model_calls.to_string()),
            ("tool_calls", stats.tool_calls.to_string()),
            ("tool_failures", stats.tool_failures.to_string()),
            ("skipped", stats.skipped_lines.to_string()),
        ],
    );
    Ok(())
}
