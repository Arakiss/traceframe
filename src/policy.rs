//! Capability/permission policy audit over a recorded trace.
//!
//! The check walks events in trace order and reports two classes of violation:
//!
//! 1. A `permission.decision` whose decision is `deny`/`denied`/`block` with no
//!    later `permission.decision` `allow` that resolves the same capability or
//!    command. These are unresolved denies the run proceeded past or left open.
//! 2. A `tool.call` that maps to a sensitive public capability (`git push` /
//!    `git.push` in the command or capability) with no prior or simultaneous
//!    `permission.decision` `allow` for that same capability/command earlier in
//!    the trace. These are sensitive actions taken without a recorded approval.
//!
//! The reading is done by the caller via `Trace::read`; this module only
//! inspects the already-parsed events, so it does not reinvent trace parsing.

use serde_json::Value;

use crate::trace::{EventKind, Trace};

/// A single human-readable policy finding.
pub type Violation = String;

/// Audit a trace and return every policy violation found, in trace order.
pub fn check(trace: &Trace) -> Vec<Violation> {
    let mut violations = Vec::new();
    check_unresolved_denies(trace, &mut violations);
    check_sensitive_without_allow(trace, &mut violations);
    violations
}

/// Rule (a): a deny with no later allow resolving the same capability/command.
fn check_unresolved_denies(trace: &Trace, violations: &mut Vec<Violation>) {
    for (index, event) in trace.events.iter().enumerate() {
        if event.kind != EventKind::PermissionDecision {
            continue;
        }
        if !is_denial(decision_of(event)) {
            continue;
        }

        let key = capability_key(event);
        let resolved = trace.events[index + 1..].iter().any(|later| {
            later.kind == EventKind::PermissionDecision
                && is_allow(decision_of(later))
                && capability_key(later) == key
        });

        if !resolved {
            violations.push(format!(
                "unresolved deny at seq {}: {} was denied with no later allow",
                event.seq,
                describe_capability(event),
            ));
        }
    }
}

/// Rule (b): a sensitive `git push` tool.call with no prior/simultaneous allow.
fn check_sensitive_without_allow(trace: &Trace, violations: &mut Vec<Violation>) {
    for (index, event) in trace.events.iter().enumerate() {
        if event.kind != EventKind::ToolCall {
            continue;
        }
        if !is_sensitive(event) {
            continue;
        }

        // A prior or simultaneous (same seq position or earlier) allow for the
        // same sensitive capability/command authorizes the call.
        let authorized = trace.events[..=index].iter().any(|prior| {
            prior.kind == EventKind::PermissionDecision
                && is_allow(decision_of(prior))
                && is_sensitive(prior)
        });

        if !authorized {
            violations.push(format!(
                "sensitive tool.call at seq {}: {} ran with no prior permission.decision allow",
                event.seq,
                describe_capability(event),
            ));
        }
    }
}

/// Read the decision string from a permission.decision payload.
fn decision_of(event: &crate::trace::Event) -> Option<&str> {
    event.payload.get("decision").and_then(Value::as_str)
}

fn is_denial(decision: Option<&str>) -> bool {
    matches!(
        decision.map(normalize),
        Some(ref d) if d == "deny" || d == "denied" || d == "block" || d == "blocked"
    )
}

fn is_allow(decision: Option<&str>) -> bool {
    matches!(
        decision.map(normalize),
        Some(ref d) if d == "allow" || d == "allowed"
    )
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

/// True when the event's command or capability touches the sensitive public
/// `git push` capability (`git push` or `git.push`).
fn is_sensitive(event: &crate::trace::Event) -> bool {
    let command = string_field(event, "command");
    let capability = string_field(event, "capability");
    [command, capability]
        .into_iter()
        .flatten()
        .any(|text| matches_git_push(&text))
}

fn matches_git_push(text: &str) -> bool {
    let normalized = normalize(text);
    normalized.contains("git push") || normalized.contains("git.push")
}

/// Build a stable key used to match a deny with a later resolving allow. Prefer
/// capability, then command, so equivalent decisions line up across events.
fn capability_key(event: &crate::trace::Event) -> String {
    string_field(event, "capability")
        .or_else(|| string_field(event, "command"))
        .unwrap_or_default()
}

/// Human-readable identifier for a finding line.
fn describe_capability(event: &crate::trace::Event) -> String {
    if let Some(capability) = string_field(event, "capability") {
        return format!("capability {capability}");
    }
    if let Some(command) = string_field(event, "command") {
        return format!("command {command}");
    }
    "unnamed capability".to_string()
}

fn string_field(event: &crate::trace::Event, key: &str) -> Option<String> {
    event
        .payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::trace::{Event, TRACEFRAME_VERSION};

    fn event(seq: u64, kind: EventKind, payload: Value) -> Event {
        Event {
            version: TRACEFRAME_VERSION,
            run_id: "run-policy".into(),
            event_id: format!("e{seq}"),
            kind,
            ts_ms: 100 + seq as i128,
            seq,
            payload,
        }
    }

    fn trace(events: Vec<Event>) -> Trace {
        Trace { events }
    }

    #[test]
    fn clean_trace_has_no_violations() {
        let trace = trace(vec![
            event(0, EventKind::RunStarted, json!({"status":"started"})),
            event(
                1,
                EventKind::PermissionDecision,
                json!({"capability":"git.push","decision":"allow"}),
            ),
            event(
                2,
                EventKind::ToolCall,
                json!({"tool":"shell","command":"git push origin main"}),
            ),
            event(3, EventKind::RunFinished, json!({"status":"success"})),
        ]);
        assert!(check(&trace).is_empty());
    }

    #[test]
    fn unresolved_deny_is_reported() {
        let trace = trace(vec![
            event(0, EventKind::RunStarted, json!({"status":"started"})),
            event(
                1,
                EventKind::PermissionDecision,
                json!({"capability":"fs.write:secrets","decision":"deny"}),
            ),
            event(2, EventKind::RunFinished, json!({"status":"failed"})),
        ]);
        let violations = check(&trace);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("unresolved deny"));
        assert!(violations[0].contains("fs.write:secrets"));
    }

    #[test]
    fn deny_followed_by_allow_resolves() {
        let trace = trace(vec![
            event(0, EventKind::RunStarted, json!({"status":"started"})),
            event(
                1,
                EventKind::PermissionDecision,
                json!({"capability":"fs.write:README.md","decision":"denied"}),
            ),
            event(
                2,
                EventKind::PermissionDecision,
                json!({"capability":"fs.write:README.md","decision":"allow"}),
            ),
            event(3, EventKind::RunFinished, json!({"status":"success"})),
        ]);
        assert!(check(&trace).is_empty());
    }

    #[test]
    fn git_push_without_allow_is_reported() {
        let trace = trace(vec![
            event(0, EventKind::RunStarted, json!({"status":"started"})),
            event(
                1,
                EventKind::ToolCall,
                json!({"tool":"shell","command":"git push --force"}),
            ),
            event(2, EventKind::RunFinished, json!({"status":"success"})),
        ]);
        let violations = check(&trace);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("sensitive tool.call"));
    }

    #[test]
    fn git_push_capability_with_allow_is_clean() {
        let trace = trace(vec![
            event(0, EventKind::RunStarted, json!({"status":"started"})),
            event(
                1,
                EventKind::PermissionDecision,
                json!({"capability":"git.push","decision":"allow"}),
            ),
            event(
                2,
                EventKind::ToolCall,
                json!({"tool":"git","capability":"git.push"}),
            ),
            event(3, EventKind::RunFinished, json!({"status":"success"})),
        ]);
        assert!(check(&trace).is_empty());
    }
}
