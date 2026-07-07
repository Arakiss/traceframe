//! Ledger commands: rebuild, list, show.

use std::path::Path;

use anyhow::{Context, Result};
use slod::ledger;

use super::print_action;

pub(crate) fn rebuild(dir: &Path, out: &Path) -> Result<()> {
    let entries = ledger::rebuild(dir, out)?;
    print_action(
        "ledger rebuild",
        &[
            ("dir", dir.display().to_string()),
            ("out", out.display().to_string()),
            ("entries", entries.len().to_string()),
        ],
    );
    Ok(())
}

pub(crate) fn list(file: &Path, status: Option<&str>) -> Result<()> {
    let entries = ledger::read(file)?;
    let entries = ledger::filter_by_status(&entries, status);
    print!("{}", ledger::render_list(&entries));
    Ok(())
}

pub(crate) fn show(file: &Path, run_id: &str) -> Result<()> {
    let entries = ledger::read(file)?;
    let entry = ledger::find_by_run_id(&entries, run_id)
        .with_context(|| format!("run not found in ledger: {run_id}"))?;
    print!("{}", ledger::render_entry(entry));
    Ok(())
}
