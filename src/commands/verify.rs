//! The verify command: schema and ordering validation.

use std::path::Path;

use anyhow::Result;
use slod::trace::Trace;

use super::print_action;

pub(crate) fn verify(file: &Path, allow_open: bool) -> Result<()> {
    let trace = Trace::read(file)?;
    if allow_open {
        trace.verify_open()?;
    } else {
        trace.verify()?;
    }
    print_action(
        "verify",
        &[
            ("file", file.display().to_string()),
            (
                "result",
                if allow_open {
                    "valid open trace".to_string()
                } else {
                    "valid trace".to_string()
                },
            ),
        ],
    );
    Ok(())
}
