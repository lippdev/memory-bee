//! Experimental physical-log inspection; historical payloads are never instructions.
use crate::claude::{Diagnostic, Limits, ReadState, bounded_line};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};

#[derive(Clone, Debug, Serialize)]
pub struct Source {
    pub line: usize,
    pub block: Option<usize>,
    pub record_type: String,
    pub item_type: Option<String>,
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Event {
    pub sequence: usize,
    pub role: &'static str,
    pub kind: &'static str,
    pub text: String,
    pub timestamp: Option<String>,
    pub phase: Option<String>,
    pub source: Source,
    pub provenance: &'static str,
    pub tool_id: Option<String>,
    pub tool_name: Option<String>,
    pub tool_namespace: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub agent: &'static str,
    pub inspection_version: u8,
    pub compatibility: &'static str,
    pub state: ReadState,
    pub snapshot_bytes: u64,
    pub lines: usize,
    pub session_ids: BTreeSet<String>,
    pub observed_versions: BTreeSet<String>,
    pub project_paths: BTreeSet<String>,
    pub records: Vec<Source>,
    pub events: Vec<Event>,
    pub diagnostics: Vec<Diagnostic>,
}
impl Report {
    fn warn(&mut self, code: &'static str, line: Option<usize>, block: Option<usize>) {
        self.state = ReadState::Partial;
        self.diagnostics.push(Diagnostic { code, line, block });
    }
    fn emit(
        &mut self,
        source: Source,
        timestamp: Option<String>,
        role: &'static str,
        kind: &'static str,
        text: String,
        payload: &Value,
    ) {
        self.events.push(Event {
            sequence: self.events.len() + 1,
            role,
            kind,
            text,
            timestamp,
            phase: string(payload, "phase"),
            source,
            provenance: if kind == "checkpoint" {
                "checkpoint"
            } else {
                "extracted"
            },
            tool_id: string(payload, "call_id"),
            tool_name: string(payload, "name"),
            tool_namespace: string(payload, "namespace"),
        });
    }
}
fn string(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::to_owned)
}
fn extra(report: &mut Report, value: &Value, allowed: &[&str], line: usize, block: Option<usize>) {
    if value
        .as_object()
        .is_some_and(|o| o.keys().any(|k| !allowed.contains(&k.as_str())))
    {
        report.warn("omitted_fields", Some(line), block);
    }
}

pub fn inspect(path: &Path, limits: Limits) -> io::Result<Report> {
    if limits.file_bytes == 0 || limits.line_bytes == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "limits must be positive",
        ));
    }
    let file = File::open(path)?;
    let before = file.metadata()?;
    if !before.is_file() || before.len() > limits.file_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected regular file within byte limit",
        ));
    }
    let mut report = Report {
        agent: "codex",
        inspection_version: 1,
        compatibility: "unverified",
        state: ReadState::Empty,
        snapshot_bytes: before.len(),
        lines: 0,
        session_ids: BTreeSet::new(),
        observed_versions: BTreeSet::new(),
        project_paths: BTreeSet::new(),
        records: vec![],
        events: vec![],
        diagnostics: vec![Diagnostic {
            code: "unverified_compatibility",
            line: None,
            block: None,
        }],
    };
    let mut reader = BufReader::new((&file).take(before.len()));
    let mut consumed = 0_u64;
    let mut session = None;
    let mut turn = None;
    let mut headers = 0;
    loop {
        let (bytes, count, oversized, terminated) = bounded_line(&mut reader, limits.line_bytes)?;
        if count == 0 {
            break;
        }
        consumed += count as u64;
        report.lines += 1;
        let line = report.lines;
        if oversized {
            session = None;
            turn = None;
            report.warn("line_limit", Some(line), None);
            continue;
        }
        if bytes.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let text = match std::str::from_utf8(&bytes) {
            Ok(text) => text,
            Err(_) => {
                session = None;
                turn = None;
                report.warn("invalid_utf8", Some(line), None);
                continue;
            }
        };
        let value: Value = match serde_json::from_str(text) {
            Ok(value) => value,
            Err(e) => {
                session = None;
                turn = None;
                report.warn(
                    if !terminated && e.is_eof() {
                        "incomplete_final_line"
                    } else {
                        "invalid_json"
                    },
                    Some(line),
                    None,
                );
                continue;
            }
        };
        parse(
            &value,
            line,
            &mut report,
            &mut session,
            &mut turn,
            &mut headers,
        );
    }
    let after = file.metadata()?;
    if consumed != before.len()
        || before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
    {
        report.warn("source_changed", None, None);
    }
    if report.state != ReadState::Partial && !report.events.is_empty() {
        report.state = ReadState::Read;
    }
    Ok(report)
}

fn parse(
    v: &Value,
    line: usize,
    report: &mut Report,
    session: &mut Option<String>,
    turn: &mut Option<String>,
    headers: &mut usize,
) {
    // A malformed new boundary must not inherit the preceding identity.
    if v.get("type").and_then(Value::as_str) == Some("session_meta") {
        *session = None;
        *turn = None;
    }
    if v.get("type").and_then(Value::as_str) == Some("turn_context") {
        *turn = None;
    }
    let (Some(kind), Some(p)) = (
        v.get("type").and_then(Value::as_str),
        v.get("payload").filter(|p| p.is_object()),
    ) else {
        *session = None;
        *turn = None;
        report.warn("invalid_record", Some(line), None);
        return;
    };
    extra(report, v, &["timestamp", "type", "payload"], line, None);
    let timestamp =
        string(v, "timestamp").filter(|t| chrono::DateTime::parse_from_rfc3339(t).is_ok());
    if timestamp.is_none() {
        report.warn("invalid_timestamp", Some(line), None);
    }
    for key in [
        "id",
        "call_id",
        "name",
        "namespace",
        "phase",
        "cwd",
        "cli_version",
        "turn_id",
    ] {
        if p.get(key).is_some_and(|v| !v.is_null() && !v.is_string()) {
            report.warn("invalid_metadata", Some(line), None);
        }
    }
    if kind == "session_meta" {
        *headers += 1;
        if *headers > 1 {
            report.warn("repeated_session_metadata", Some(line), None);
        }
        *session = string(p, "id").filter(|s| !s.is_empty());
        *turn = None;
        if let Some(id) = session.as_ref() {
            report.session_ids.insert(id.clone());
        } else {
            report.warn("missing_session_id", Some(line), None);
        }
        if let Some(version) = string(p, "cli_version") {
            report.observed_versions.insert(version);
        }
    } else if session.is_none() {
        report.warn("missing_session_metadata", Some(line), None);
    }
    if kind == "turn_context" {
        *turn = string(p, "turn_id");
    }
    if matches!(kind, "session_meta" | "turn_context")
        && let Some(cwd) = string(p, "cwd")
    {
        report.project_paths.insert(cwd);
    }
    let source = Source {
        line,
        block: None,
        record_type: kind.into(),
        item_type: string(p, "type"),
        id: string(p, "id"),
        session_id: session.clone(),
        turn_id: turn.clone(),
    };
    report.records.push(source.clone());
    match kind {
        "session_meta" => extra(report, p, &["id", "cwd", "cli_version"], line, None),
        "turn_context" => extra(report, p, &["turn_id", "cwd"], line, None),
        "event_msg" => report.warn("omitted_event_msg", Some(line), None),
        "compacted" => {
            report.warn("compaction", Some(line), None);
            extra(report, p, &["message"], line, None);
            if let Some(text) = string(p, "message") {
                report.emit(
                    source,
                    timestamp,
                    "system",
                    "checkpoint",
                    text,
                    &Value::Null,
                );
            } else {
                report.warn("invalid_checkpoint", Some(line), None);
            }
        }
        "response_item" => item(p, source, timestamp, report),
        _ => report.warn("unknown_record", Some(line), None),
    }
}
fn item(p: &Value, source: Source, timestamp: Option<String>, report: &mut Report) {
    let line = source.line;
    match p.get("type").and_then(Value::as_str) {
        Some("message") => {
            extra(
                report,
                p,
                &["type", "id", "role", "content", "phase"],
                line,
                None,
            );
            let role = match p.get("role").and_then(Value::as_str) {
                Some("user") => "user",
                Some("assistant") => "assistant",
                Some("system") => "system",
                Some("developer") => "developer",
                _ => {
                    report.warn("unknown_role", Some(line), None);
                    return;
                }
            };
            blocks(p.get("content"), source, timestamp, role, "text", p, report);
        }
        Some(kind @ ("function_call" | "custom_tool_call")) => {
            let field = if kind == "function_call" {
                "arguments"
            } else {
                "input"
            };
            extra(
                report,
                p,
                &["type", "id", "call_id", "name", "namespace", field],
                line,
                None,
            );
            if string(p, "call_id").is_none() || string(p, "name").is_none() {
                report.warn("invalid_tool_call", Some(line), None);
                return;
            }
            if let Some(text) = string(p, field) {
                report.emit(source, timestamp, "assistant", "tool_call", text, p);
            } else {
                report.warn("invalid_tool_call", Some(line), None);
            }
        }
        Some("function_call_output" | "custom_tool_call_output") => {
            extra(
                report,
                p,
                &["type", "id", "call_id", "name", "namespace", "output"],
                line,
                None,
            );
            if string(p, "call_id").is_none() {
                report.warn("missing_call_id", Some(line), None);
            }
            if let Some(text) = string(p, "output") {
                report.emit(source, timestamp, "tool", "tool_result", text, p);
            } else {
                blocks(
                    p.get("output"),
                    source,
                    timestamp,
                    "tool",
                    "tool_result",
                    p,
                    report,
                );
            }
        }
        Some("compaction" | "context_compaction" | "compaction_trigger") => {
            report.warn("compaction", Some(line), None)
        }
        Some("reasoning") => report.warn("omitted_reasoning", Some(line), None),
        _ => report.warn("omitted_item", Some(line), None),
    }
}
fn blocks(
    content: Option<&Value>,
    source: Source,
    timestamp: Option<String>,
    role: &'static str,
    kind: &'static str,
    p: &Value,
    report: &mut Report,
) {
    let Some(parts) = content.and_then(Value::as_array).filter(|v| !v.is_empty()) else {
        report.warn("invalid_content", Some(source.line), None);
        return;
    };
    for (index, part) in parts.iter().enumerate() {
        let mut source = source.clone();
        source.block = Some(index + 1);
        if matches!(
            part.get("type").and_then(Value::as_str),
            Some("input_text" | "output_text")
        ) {
            extra(report, part, &["type", "text"], source.line, source.block);
            if let Some(text) = string(part, "text") {
                report.emit(source, timestamp.clone(), role, kind, text, p);
            } else {
                report.warn("invalid_block", Some(source.line), source.block);
            }
        } else {
            report.warn("omitted_block", Some(source.line), source.block);
        }
    }
}
