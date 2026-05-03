use std::{fmt, str::FromStr};

use anyhow::{Result, bail};
use serde_json::{Map, Value};

use crate::trace::EventKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookSource {
    Codex,
    Omx,
    Generic,
}

impl HookSource {
    pub fn as_str(self) -> &'static str {
        match self {
            HookSource::Codex => "codex",
            HookSource::Omx => "omx",
            HookSource::Generic => "generic",
        }
    }
}

impl fmt::Display for HookSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for HookSource {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim().to_ascii_lowercase().as_str() {
            "codex" => Ok(HookSource::Codex),
            "omx" | "oh-my-codex" => Ok(HookSource::Omx),
            "generic" | "host" => Ok(HookSource::Generic),
            other => bail!("unknown hook source: {other}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HookEvent {
    pub kind: EventKind,
    pub payload: Value,
}

pub fn map_hook_payload(source: HookSource, payload: &Value) -> Result<Vec<HookEvent>> {
    let Some(object) = payload.as_object() else {
        bail!("hook payload must be a JSON object");
    };
    if object.is_empty() {
        bail!("hook payload must not be empty");
    }

    let hook_event = string_field(
        payload,
        &["hook_event_name", "event_name", "event", "type", "name"],
    )
    .unwrap_or_else(|| "hook.event".to_string());
    let normalized = hook_event.to_ascii_lowercase();

    if let Some(event) = map_permission_decision(source, &hook_event, payload) {
        return Ok(vec![event]);
    }

    if is_tool_result_hook(&normalized, payload) {
        return Ok(vec![map_tool_result(source, &hook_event, payload)]);
    }

    if is_error_hook(&normalized, payload) {
        return Ok(vec![map_error(source, &hook_event, payload)]);
    }

    if is_tool_call_hook(&normalized, payload) {
        return Ok(vec![map_tool_call(source, &hook_event, payload)]);
    }

    bail!(
        "unsupported hook payload: expected tool call, tool result, permission decision, or error"
    )
}

fn map_permission_decision(
    source: HookSource,
    hook_event: &str,
    payload: &Value,
) -> Option<HookEvent> {
    let decision = string_field(
        payload,
        &[
            "decision",
            "permission_decision",
            "permissionDecision",
            "permission.decision",
            "tool_decision",
        ],
    )?;

    let mut out = base_payload(source, hook_event);
    insert_string(
        &mut out,
        "capability",
        string_field(payload, &["capability", "permission.capability"])
            .or_else(|| infer_capability(payload)),
    );
    insert_string(&mut out, "decision", Some(decision));
    insert_string(
        &mut out,
        "reason",
        string_field(payload, &["reason", "permission.reason"]),
    );
    insert_string(&mut out, "tool", infer_tool(payload));
    insert_string(&mut out, "command", infer_command(payload));
    insert_host_context(&mut out, payload);

    Some(HookEvent {
        kind: EventKind::PermissionDecision,
        payload: Value::Object(out),
    })
}

fn map_tool_call(source: HookSource, hook_event: &str, payload: &Value) -> HookEvent {
    let mut out = base_payload(source, hook_event);
    insert_string(&mut out, "tool", infer_tool(payload));
    insert_string(&mut out, "command", infer_command(payload));
    insert_value(
        &mut out,
        "input",
        value_field(
            payload,
            &[
                "tool_input",
                "toolInput",
                "input",
                "arguments",
                "args",
                "parameters",
                "tool_call.input",
            ],
        ),
    );
    insert_host_context(&mut out, payload);

    HookEvent {
        kind: EventKind::ToolCall,
        payload: Value::Object(out),
    }
}

fn map_tool_result(source: HookSource, hook_event: &str, payload: &Value) -> HookEvent {
    let exit_code = value_field(
        payload,
        &[
            "exit_code",
            "exitCode",
            "tool_response.exit_code",
            "result.exit_code",
        ],
    );
    let success = bool_field(
        payload,
        &[
            "success",
            "tool_response.success",
            "result.success",
            "ok",
            "status.success",
        ],
    )
    .or_else(|| exit_code.as_ref().and_then(value_exit_success))
    .unwrap_or_else(|| {
        string_field(payload, &["error", "tool_response.error", "result.error"]).is_none()
    });

    let mut out = base_payload(source, hook_event);
    insert_string(&mut out, "tool", infer_tool(payload));
    insert_string(&mut out, "command", infer_command(payload));
    out.insert("success".to_string(), Value::Bool(success));
    insert_value(&mut out, "exit_code", exit_code);
    insert_value(
        &mut out,
        "duration_ms",
        value_field(
            payload,
            &[
                "duration_ms",
                "durationMs",
                "tool_response.duration_ms",
                "result.duration_ms",
            ],
        ),
    );
    insert_value(
        &mut out,
        "output",
        value_field(
            payload,
            &[
                "output",
                "stdout",
                "tool_response.output",
                "tool_response.stdout",
                "result.output",
                "result.stdout",
            ],
        ),
    );
    insert_string(
        &mut out,
        "error",
        string_field(payload, &["error", "tool_response.error", "result.error"]),
    );
    insert_host_context(&mut out, payload);

    HookEvent {
        kind: EventKind::ToolResult,
        payload: Value::Object(out),
    }
}

fn map_error(source: HookSource, hook_event: &str, payload: &Value) -> HookEvent {
    let mut out = base_payload(source, hook_event);
    insert_string(
        &mut out,
        "message",
        string_field(payload, &["message", "error", "error.message"])
            .or_else(|| Some("host hook reported an error".to_string())),
    );
    insert_string(&mut out, "tool", infer_tool(payload));
    insert_string(&mut out, "command", infer_command(payload));
    insert_host_context(&mut out, payload);

    HookEvent {
        kind: EventKind::Error,
        payload: Value::Object(out),
    }
}

fn is_tool_call_hook(normalized: &str, payload: &Value) -> bool {
    normalized.contains("pretooluse")
        || normalized.contains("tool.call")
        || normalized.contains("tool_call")
        || normalized.contains("toolcall")
        || value_field(
            payload,
            &[
                "tool_input",
                "toolInput",
                "tool_name",
                "toolName",
                "tool_call",
                "request.tool_input",
            ],
        )
        .is_some()
}

fn is_tool_result_hook(normalized: &str, payload: &Value) -> bool {
    normalized.contains("posttooluse")
        || normalized.contains("tool.result")
        || normalized.contains("tool_result")
        || normalized.contains("toolresult")
        || value_field(
            payload,
            &[
                "tool_response",
                "toolResponse",
                "result",
                "success",
                "exit_code",
                "exitCode",
            ],
        )
        .is_some()
}

fn is_error_hook(normalized: &str, payload: &Value) -> bool {
    normalized.contains("error")
        || value_field(payload, &["error", "error.message", "failure"]).is_some()
}

fn base_payload(source: HookSource, hook_event: &str) -> Map<String, Value> {
    let mut out = Map::new();
    out.insert("source".to_string(), Value::String(source.to_string()));
    out.insert(
        "hook_event".to_string(),
        Value::String(hook_event.to_string()),
    );
    out
}

fn insert_host_context(out: &mut Map<String, Value>, payload: &Value) {
    let mut host = Map::new();
    for key in [
        "session_id",
        "transcript_path",
        "cwd",
        "workspace",
        "model",
        "hook_event_name",
    ] {
        insert_value(&mut host, key, value_field(payload, &[key]));
    }
    if !host.is_empty() {
        out.insert("host".to_string(), Value::Object(host));
    }
}

fn infer_capability(payload: &Value) -> Option<String> {
    let tool = infer_tool(payload)?;
    let command = infer_command(payload);
    Some(match command {
        Some(command) => format!("tool:{tool}:{command}"),
        None => format!("tool:{tool}"),
    })
}

fn infer_tool(payload: &Value) -> Option<String> {
    string_field(
        payload,
        &[
            "tool_name",
            "toolName",
            "tool",
            "tool.name",
            "tool_call.tool",
            "tool_call.name",
            "request.tool_name",
            "tool_response.tool",
            "result.tool",
        ],
    )
}

fn infer_command(payload: &Value) -> Option<String> {
    string_field(
        payload,
        &[
            "command",
            "tool_input.command",
            "toolInput.command",
            "input.command",
            "arguments.command",
            "args.command",
            "parameters.command",
            "tool_call.command",
            "tool_response.command",
            "result.command",
        ],
    )
}

fn insert_string(out: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        out.insert(key.to_string(), Value::String(value));
    }
}

fn insert_value(out: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        out.insert(key.to_string(), value);
    }
}

fn string_field(value: &Value, paths: &[&str]) -> Option<String> {
    for path in paths {
        let Some(value) = value_at(value, path) else {
            continue;
        };
        match value {
            Value::String(text) if !text.trim().is_empty() => return Some(text.to_string()),
            Value::Number(number) => return Some(number.to_string()),
            Value::Bool(flag) => return Some(flag.to_string()),
            Value::Object(object) => {
                for nested in ["name", "tool", "command", "message"] {
                    if let Some(Value::String(text)) = object.get(nested)
                        && !text.trim().is_empty()
                    {
                        return Some(text.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn bool_field(value: &Value, paths: &[&str]) -> Option<bool> {
    for path in paths {
        let Some(value) = value_at(value, path) else {
            continue;
        };
        match value {
            Value::Bool(flag) => return Some(*flag),
            Value::Number(number) => {
                if let Some(code) = number.as_i64() {
                    return Some(code == 0);
                }
            }
            Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
                "true" | "ok" | "success" | "allow" | "allowed" => return Some(true),
                "false" | "failed" | "error" | "deny" | "denied" => return Some(false),
                _ => {}
            },
            _ => {}
        }
    }
    None
}

fn value_exit_success(value: &Value) -> Option<bool> {
    match value {
        Value::Number(number) => number.as_i64().map(|code| code == 0),
        Value::String(text) => text.parse::<i64>().ok().map(|code| code == 0),
        _ => None,
    }
}

fn value_field(value: &Value, paths: &[&str]) -> Option<Value> {
    paths
        .iter()
        .find_map(|path| value_at(value, path).cloned())
        .filter(|value| !value.is_null())
}

fn value_at<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_supported_sources() {
        assert_eq!("codex".parse::<HookSource>().unwrap(), HookSource::Codex);
        assert_eq!(
            "oh-my-codex".parse::<HookSource>().unwrap(),
            HookSource::Omx
        );
        assert_eq!("host".parse::<HookSource>().unwrap(), HookSource::Generic);
        assert!("unknown".parse::<HookSource>().is_err());
    }

    #[test]
    fn maps_codex_pre_tool_use_to_tool_call() {
        let events = map_hook_payload(
            HookSource::Codex,
            &json!({
                "hook_event_name": "PreToolUse",
                "tool_name": "Bash",
                "tool_input": {"command": "cargo test"},
                "session_id": "session-demo"
            }),
        )
        .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::ToolCall);
        assert_eq!(events[0].payload["source"], "codex");
        assert_eq!(events[0].payload["tool"], "Bash");
        assert_eq!(events[0].payload["command"], "cargo test");
        assert_eq!(events[0].payload["host"]["session_id"], "session-demo");
    }

    #[test]
    fn maps_codex_post_tool_use_to_tool_result() {
        let events = map_hook_payload(
            HookSource::Codex,
            &json!({
                "hook_event_name": "PostToolUse",
                "tool_name": "Bash",
                "tool_response": {
                    "success": true,
                    "exit_code": 0,
                    "stdout": "ok"
                }
            }),
        )
        .unwrap();

        assert_eq!(events[0].kind, EventKind::ToolResult);
        assert_eq!(events[0].payload["success"], true);
        assert_eq!(events[0].payload["exit_code"], 0);
        assert_eq!(events[0].payload["output"], "ok");
    }

    #[test]
    fn maps_permission_decision() {
        let events = map_hook_payload(
            HookSource::Omx,
            &json!({
                "hook_event_name": "PreToolUse",
                "tool_name": "Write",
                "tool_input": {"command": "edit README.md"},
                "decision": "allow"
            }),
        )
        .unwrap();

        assert_eq!(events[0].kind, EventKind::PermissionDecision);
        assert_eq!(events[0].payload["decision"], "allow");
        assert_eq!(events[0].payload["capability"], "tool:Write:edit README.md");
    }

    #[test]
    fn maps_error_payload() {
        let events = map_hook_payload(
            HookSource::Generic,
            &json!({
                "hook_event_name": "HookError",
                "error": {"message": "permission hook failed"},
                "tool_name": "Bash"
            }),
        )
        .unwrap();

        assert_eq!(events[0].kind, EventKind::Error);
        assert_eq!(events[0].payload["message"], "permission hook failed");
    }

    #[test]
    fn rejects_empty_or_unsupported_payloads() {
        assert!(map_hook_payload(HookSource::Codex, &json!({})).is_err());
        assert!(map_hook_payload(HookSource::Codex, &json!("nope")).is_err());
        assert!(map_hook_payload(HookSource::Codex, &json!({"event":"UserPromptSubmit"})).is_err());
    }
}
