//! Select a physical branch without executing or interpreting its content.
use crate::claude::{Diagnostic, Report, Selection};
use std::collections::{HashMap, HashSet};

/// Selection refuses ambiguous identity, missing ancestry and agent/session changes.
/// All original diagnostics remain (except the now-resolved selection prompt).
pub fn select(mut report: Report, leaf: &str) -> Result<Report, &'static str> {
    if report.selection.is_some() {
        return Err("report already has a selection");
    }
    if !report.branch_tips.iter().any(|id| id == leaf) {
        return Err("UUID is not a branch tip");
    }
    let mut by_id: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, record) in report.records.iter().enumerate() {
        if let Some(id) = record.source.id.as_deref() {
            by_id.entry(id).or_default().push(index);
        }
    }
    let mut current = leaf;
    let mut selected = HashSet::new();
    let mut identity = None;
    let mut child_line = usize::MAX;
    loop {
        let indexes = by_id.get(current).ok_or("missing ancestor")?;
        if indexes.len() != 1 {
            return Err("duplicate UUID in selected ancestry");
        }
        let record = &report.records[indexes[0]];
        let source = &record.source;
        if current.is_empty() || !record.valid_metadata || !record.parent_known {
            return Err("incomplete or invalid branch metadata");
        }
        if !selected.insert(source.line) || source.line >= child_line {
            return Err("cyclic or forward ancestry");
        }
        child_line = source.line;
        let session = source
            .session_id
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or("missing session identity")?;
        let this_identity = (
            session,
            source.agent_id.as_deref(),
            source.is_sidechain.unwrap_or(false),
        );
        if identity.is_some_and(|identity| identity != this_identity) {
            return Err("ancestry crosses session or agent boundaries");
        }
        identity = Some(this_identity);
        match source.parent_id.as_deref() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    let excluded_records = report.records.len() - selected.len();
    let before = report.events.len();
    report
        .events
        .retain(|event| selected.contains(&event.source.line));
    report
        .records
        .retain(|record| selected.contains(&record.source.line));
    for (index, event) in report.events.iter_mut().enumerate() {
        event.sequence = index + 1;
    }
    report.selection = Some(Selection {
        leaf_uuid: leaf.to_owned(),
        excluded_records,
        excluded_events: before - report.events.len(),
    });
    report.requires_branch_selection = false;
    report
        .diagnostics
        .retain(|d| d.code != "branch_selection_required");
    if excluded_records > 0 || before != report.events.len() {
        report.diagnostics.push(Diagnostic {
            code: "branch_excluded",
            line: None,
            block: None,
        });
    }
    // State, project metadata, original tips and diagnostics describe the full input.
    Ok(report)
}
