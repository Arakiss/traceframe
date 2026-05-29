//! Read-only reporting commands: inspect, summary, render.

use std::path::Path;

use anyhow::Result;
use traceframe::{render, trace::Trace};

use super::print_action;

pub(crate) fn inspect(file: &Path) -> Result<()> {
    let trace = Trace::read(file)?;
    trace.verify_open()?;
    print!("{}", trace.inspect());
    Ok(())
}

pub(crate) fn summary(file: &Path) -> Result<()> {
    let trace = Trace::read(file)?;
    trace.verify_open()?;
    print!("{}", trace.summary().render_text());
    Ok(())
}

pub(crate) fn render(file: &Path, html: &Path) -> Result<()> {
    let trace = Trace::read(file)?;
    trace.verify_open()?;
    render::write_html(&trace, html)?;
    print_action(
        "render",
        &[
            ("file", file.display().to_string()),
            ("html", html.display().to_string()),
        ],
    );
    Ok(())
}
