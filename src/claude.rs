use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeSet, HashSet},
    fs::File,
    io::{self, BufRead, BufReader, Read},
    path::Path,
};

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub file_bytes: u64,
    pub line_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            file_bytes: 64 * 1024 * 1024,
            line_bytes: 1024 * 1024,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReadState {
    Read,
    Empty,
    Partial,
}

#[derive(Debug, Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub line: Option<usize>,
    pub block: Option<usize>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Source {
    pub line: usize,
    pub block: Option<usize>,
    pub id: Option<String>,
    pub parent_id: Option<String>,
    pub session_id: Option<String>,
    pub agent_id: Option<String>,
    pub is_sidechain: Option<bool>,
}
#[derive(Debug, Serialize)]
pub struct Event {
    pub sequence: usize,
    pub role: &'static str,
    pub kind: &'static str,
    pub text: String,
    pub timestamp: Option<String>,
    pub source: Source,
    pub provenance: &'static str,
    pub tool_id: Option<String>,
    pub tool_name: Option<String>,
    pub tool_is_error: Option<bool>,
}
#[derive(Debug, Serialize)]
pub struct Record {
    pub source: Source,
    pub parent_known: bool,
    pub valid_metadata: bool,
}
#[derive(Debug, Serialize)]
pub struct Selection {
    pub leaf_uuid: String,
    pub excluded_records: usize,
    pub excluded_events: usize,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub inspection_version: u8,
    pub state: ReadState,
    pub snapshot_bytes: u64,
    pub lines: usize,
    pub compatibility: &'static str,
    pub observed_versions: BTreeSet<String>,
    pub requires_branch_selection: bool,
    pub events: Vec<Event>,
    pub records: Vec<Record>,
    pub project_paths: BTreeSet<String>,
    pub branch_tips: Vec<String>,
    pub selection: Option<Selection>,
    pub diagnostics: Vec<Diagnostic>,
}
impl Report {
    fn warn(&mut self, code: &'static str, line: Option<usize>, block: Option<usize>, loss: bool) {
        self.diagnostics.push(Diagnostic { code, line, block });
        if loss {
            self.state = ReadState::Partial;
        }
    }
    fn emit(
        &mut self,
        source: Source,
        timestamp: Option<String>,
        role: &'static str,
        kind: &'static str,
        text: String,
        tool: Option<&Value>,
    ) {
        self.events.push(Event {
            sequence: self.events.len() + 1,
            role,
            kind,
            text,
            timestamp,
            source,
            provenance: if kind == "checkpoint" {
                "checkpoint"
            } else {
                "extracted"
            },
            tool_id: tool.and_then(|v| {
                string(
                    v,
                    if kind == "tool_call" {
                        "id"
                    } else {
                        "tool_use_id"
                    },
                )
            }),
            tool_name: tool.and_then(|v| string(v, "name")),
            tool_is_error: tool
                .and_then(|v| v.get("is_error"))
                .and_then(Value::as_bool),
        });
    }
}
fn string(v: &Value, key: &str) -> Option<String> {
    v.get(key)?.as_str().map(str::to_owned)
}

/// Reads at most the size observed on opening. No network, subprocesses or writes.
/// File limits are fatal; oversized lines are skipped with a located diagnostic.
pub fn inspect(path: &Path, limits: Limits) -> io::Result<Report> {
    if limits.file_bytes == 0 || limits.line_bytes == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "limits must be positive",
        ));
    }
    let file = File::open(path)?;
    let before = file.metadata()?;
    if !before.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected a regular file",
        ));
    }
    if before.len() > limits.file_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file exceeds configured byte limit",
        ));
    }
    let mut reader = BufReader::new((&file).take(before.len()));
    let mut report = Report {
        inspection_version: 1,
        state: ReadState::Empty,
        snapshot_bytes: before.len(),
        lines: 0,
        compatibility: "unverified",
        observed_versions: BTreeSet::new(),
        requires_branch_selection: false,
        events: vec![],
        records: vec![],
        project_paths: BTreeSet::new(),
        branch_tips: vec![],
        selection: None,
        diagnostics: vec![],
    };
    report.warn("unverified_compatibility", None, None, false);
    let mut graph = Graph::default();
    let mut consumed = 0_u64;
    loop {
        let (bytes, count, oversized, terminated) = bounded_line(&mut reader, limits.line_bytes)?;
        if count == 0 {
            break;
        }
        consumed += count as u64;
        report.lines += 1;
        let line = report.lines;
        if oversized {
            report.warn("line_limit", Some(line), None, true);
            continue;
        }
        if bytes.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let text = match std::str::from_utf8(&bytes) {
            Ok(text) => text,
            Err(_) => {
                report.warn("invalid_utf8", Some(line), None, true);
                continue;
            }
        };
        let value: Value = match serde_json::from_str(text) {
            Ok(value) => value,
            Err(e) => {
                report.warn(
                    if !terminated && e.is_eof() {
                        "incomplete_final_line"
                    } else {
                        "invalid_json"
                    },
                    Some(line),
                    None,
                    true,
                );
                continue;
            }
        };
        parse_record(&value, line, &mut report, &mut graph);
    }
    graph.finish(&mut report);
    let after = file.metadata()?;
    if consumed != before.len()
        || before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
    {
        report.warn("source_changed", None, None, true);
    }
    if report.state != ReadState::Partial && !report.events.is_empty() {
        report.state = ReadState::Read;
    }
    Ok(report)
}

// Drain an oversized line without allocating the whole line. Count includes delimiters.
pub(crate) fn bounded_line(
    reader: &mut impl BufRead,
    limit: usize,
) -> io::Result<(Vec<u8>, usize, bool, bool)> {
    let mut bytes = Vec::new();
    let mut count = 0;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok((bytes, count, count > limit, false));
        }
        let end = available.iter().position(|b| *b == b'\n').map(|n| n + 1);
        let n = end.unwrap_or(available.len());
        let keep = n.min(limit.saturating_sub(count));
        bytes.extend_from_slice(&available[..keep]);
        count += n;
        reader.consume(n);
        if end.is_some() {
            return Ok((bytes, count, count > limit, true));
        }
    }
}

#[derive(Default)]
struct Graph {
    ids: HashSet<String>,
    parents: HashSet<Option<String>>,
    references: Vec<(String, usize)>,
    sessions: HashSet<String>,
}
impl Graph {
    fn record(&mut self, source: &Source, report: &mut Report) {
        if let Some(session) = &source.session_id {
            self.sessions.insert(session.clone());
        }
        if let Some(id) = &source.id {
            if !self.ids.insert(id.clone()) {
                report.warn("duplicate_uuid", Some(source.line), None, true);
                report.requires_branch_selection = true;
            }
            if !self.parents.insert(source.parent_id.clone()) {
                report.requires_branch_selection = true;
            }
            if let Some(parent) = &source.parent_id {
                // A parent must precede its child: forward references and cycles are ambiguous.
                if parent == id || !self.ids.contains(parent) {
                    report.warn("nonpreceding_parent", Some(source.line), None, false);
                    report.requires_branch_selection = true;
                }
                self.references.push((parent.clone(), source.line));
            }
        } else {
            report.warn("missing_uuid", Some(source.line), None, false);
            report.requires_branch_selection = true;
        }
        if source.is_sidechain == Some(true) || source.agent_id.is_some() {
            report.requires_branch_selection = true;
        }
    }
    fn finish(self, report: &mut Report) {
        report.branch_tips = self
            .ids
            .iter()
            .filter(|id| !self.parents.contains(&Some((*id).clone())))
            .cloned()
            .collect();
        report.branch_tips.sort();
        for (parent, line) in self.references {
            if !self.ids.contains(&parent) {
                report.warn("missing_parent", Some(line), None, false);
            }
        }
        if self.sessions.len() > 1 {
            report.requires_branch_selection = true;
        }
        if report.requires_branch_selection {
            report.warn("branch_selection_required", None, None, false);
        }
    }
}

fn parse_record(v: &Value, line: usize, report: &mut Report, graph: &mut Graph) {
    if let Some(version) = string(v, "version") {
        report.observed_versions.insert(version);
    }
    let mut valid_metadata = true;
    for key in ["uuid", "parentUuid", "sessionId", "agentId", "version"] {
        if v.get(key)
            .is_some_and(|value| !value.is_null() && !value.is_string())
        {
            valid_metadata = false;
            report.warn("invalid_metadata", Some(line), None, true);
            report.requires_branch_selection = true;
        }
    }
    for key in ["isSidechain", "isCompactSummary"] {
        if v.get(key)
            .is_some_and(|value| !value.is_null() && !value.is_boolean())
        {
            valid_metadata = false;
            report.warn("invalid_metadata", Some(line), None, true);
            report.requires_branch_selection = true;
        }
    }
    let source = Source {
        line,
        block: None,
        id: string(v, "uuid"),
        parent_id: string(v, "parentUuid"),
        session_id: string(v, "sessionId"),
        agent_id: string(v, "agentId"),
        is_sidechain: v.get("isSidechain").and_then(Value::as_bool),
    };
    let timestamp =
        string(v, "timestamp").filter(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok());
    if v.get("timestamp").is_some_and(|t| !t.is_null()) && timestamp.is_none() {
        report.warn("invalid_timestamp", Some(line), None, true);
    }
    let kind = v.get("type").and_then(Value::as_str).unwrap_or("");
    if kind == "summary"
        || (kind == "system"
            && v.get("subtype").and_then(Value::as_str) == Some("compact_boundary"))
    {
        report.warn("compaction", Some(line), None, true);
        if let Some(summary) = string(v, "summary") {
            report.emit(source, timestamp, "system", "checkpoint", summary, None);
        }
        return;
    }
    let role = match kind {
        "user" => "user",
        "assistant" => "assistant",
        _ => {
            report.warn("unknown_record", Some(line), None, true);
            return;
        }
    };
    if let Some(cwd) = string(v, "cwd") {
        report.project_paths.insert(cwd);
    } else if v.get("cwd").is_some_and(|cwd| !cwd.is_null()) {
        report.warn("invalid_project_metadata", Some(line), None, true);
    }
    report.records.push(Record {
        source: source.clone(),
        parent_known: v
            .get("parentUuid")
            .is_some_and(|p| p.is_null() || p.is_string()),
        valid_metadata,
    });
    graph.record(&source, report);
    let checkpoint = v.get("isCompactSummary").and_then(Value::as_bool) == Some(true);
    if checkpoint {
        report.warn("compaction", Some(line), None, true);
    }
    if v.pointer("/message/role")
        .is_some_and(|r| r.as_str() != Some(role))
    {
        report.warn("role_mismatch", Some(line), None, true);
        return;
    }
    let text_kind = if checkpoint { "checkpoint" } else { "text" };
    match v.pointer("/message/content") {
        Some(Value::String(text)) => {
            report.emit(source, timestamp, role, text_kind, text.clone(), None)
        }
        Some(Value::Array(blocks)) => {
            if blocks.is_empty() {
                report.warn("invalid_content", Some(line), None, true);
            }
            for (index, block) in blocks.iter().enumerate() {
                let mut source = source.clone();
                source.block = Some(index + 1);
                match block.get("type").and_then(Value::as_str) {
                    Some("text") => {
                        if let Some(text) = string(block, "text") {
                            report.emit(source, timestamp.clone(), role, text_kind, text, None);
                        } else {
                            report.warn("invalid_block", Some(line), source.block, true);
                        }
                    }
                    Some("tool_use") if role == "assistant" => {
                        if string(block, "id").is_none()
                            || string(block, "name").is_none()
                            || block.get("input").is_none()
                        {
                            report.warn("invalid_block", Some(line), source.block, true);
                        } else {
                            report.emit(
                                source,
                                timestamp.clone(),
                                "assistant",
                                "tool_call",
                                block["input"].to_string(),
                                Some(block),
                            );
                        }
                    }
                    Some("tool_result") if role == "user" => {
                        if string(block, "tool_use_id").is_none() {
                            report.warn("invalid_block", Some(line), source.block, true);
                            continue;
                        }
                        let text = tool_text(block.get("content"), line, source.block, report);
                        report.emit(
                            source,
                            timestamp.clone(),
                            "tool",
                            "tool_result",
                            text,
                            Some(block),
                        );
                    }
                    _ => report.warn("omitted_block", Some(line), source.block, true),
                }
            }
        }
        _ => report.warn("invalid_content", Some(line), None, true),
    }
}

fn tool_text(
    content: Option<&Value>,
    line: usize,
    block: Option<usize>,
    report: &mut Report,
) -> String {
    match content {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(parts)) => {
            let mut texts = vec![];
            for part in parts {
                if part.get("type").and_then(Value::as_str) == Some("text")
                    && let Some(text) = string(part, "text")
                {
                    texts.push(text);
                    continue;
                }
                report.warn("omitted_tool_content", Some(line), block, true);
            }
            texts.join("\n")
        }
        _ => {
            report.warn("invalid_tool_content", Some(line), block, true);
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn snapshot_cap_excludes_appended_bytes() {
        let initial = b"one\ntwo";
        let mut bytes = initial.to_vec();
        bytes.extend_from_slice(b" appended\nthree\n");
        let mut reader = BufReader::with_capacity(2, Cursor::new(bytes).take(initial.len() as u64));
        assert_eq!(
            bounded_line(&mut reader, 10).unwrap(),
            (b"one\n".to_vec(), 4, false, true)
        );
        assert_eq!(
            bounded_line(&mut reader, 10).unwrap(),
            (b"two".to_vec(), 3, false, false)
        );
        assert_eq!(bounded_line(&mut reader, 10).unwrap().1, 0);
    }

    #[test]
    fn exact_line_limit_and_oversized_final_line() {
        let mut reader = BufReader::with_capacity(2, Cursor::new(b"abc\n12345"));
        assert_eq!(
            bounded_line(&mut reader, 4).unwrap(),
            (b"abc\n".to_vec(), 4, false, true)
        );
        assert_eq!(
            bounded_line(&mut reader, 4).unwrap(),
            (b"1234".to_vec(), 5, true, false)
        );
    }
}
