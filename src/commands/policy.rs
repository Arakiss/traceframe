//! The policy-check command: audit a trace against capability/permission rules.

use std::{path::Path, process};

use anyhow::Result;
use slod::{policy, trace::Trace};

use super::{eprint_action, print_action};

pub(crate) fn policy_check(file: &Path, allow_open: bool) -> Result<()> {
    let trace = Trace::read(file)?;
    if allow_open {
        trace.verify_open()?;
    } else {
        trace.verify()?;
    }

    let violations = policy::check(&trace);
    if violations.is_empty() {
        print_action(
            "policy-check",
            &[
                ("file", file.display().to_string()),
                ("result", "clean".to_string()),
            ],
        );
        return Ok(());
    }

    eprint_action(
        "policy-check",
        &[
            ("file", file.display().to_string()),
            ("result", format!("{} violation(s)", violations.len())),
        ],
    );
    for violation in &violations {
        eprintln!("  - {violation}");
    }
    process::exit(1);
}
