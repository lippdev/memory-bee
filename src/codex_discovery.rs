//! Experimental Codex discovery: explicit root, physical files, metadata-only output.
use crate::{
    claude::{Diagnostic, Limits, ReadState},
    codex,
    discovery::{DiscoveryLimits, Inspection, Layout, SessionPath, discover_with},
};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize)]
pub struct Session {
    pub path: PathBuf,
    pub session_id: String,
    pub observed_versions: BTreeSet<String>,
    pub state: ReadState,
    pub record_count: usize,
    pub event_count: usize,
    pub diagnostics: Vec<Diagnostic>,
}
impl SessionPath for Session {
    fn path(&self) -> &Path {
        &self.path
    }
}
pub type Discovery = crate::discovery::Discovery<Session>;

pub fn discover(root: &Path, project: &Path, limits: DiscoveryLimits) -> io::Result<Discovery> {
    discover_with(root, project, limits, Layout::Codex, inspect)
}
fn inspect(path: &Path, limits: Limits, _nested: bool) -> io::Result<Inspection<Session>> {
    let report = codex::inspect(path, limits)?;
    let partial = report.state == ReadState::Partial;
    let session = if report.session_ids.is_empty() {
        Err("missing_session_metadata")
    } else if report.session_ids.len() > 1 {
        Err("conflicting_session_metadata")
    } else {
        Ok(Session {
            path: path.to_path_buf(),
            session_id: report.session_ids.into_iter().next().expect("one session"),
            observed_versions: report.observed_versions,
            record_count: report.records.len(),
            event_count: report.events.len(),
            state: report.state,
            diagnostics: report.diagnostics,
        })
    };
    Ok(Inspection {
        snapshot_bytes: report.snapshot_bytes,
        // Partial files remain visible even if classification later excludes them.
        partial,
        project_paths: report.project_paths,
        session,
    })
}
