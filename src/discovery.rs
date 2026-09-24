//! Bounded discovery under explicit roots. Reader-specific layouts and metadata stay separate.
use crate::claude::{self, Diagnostic, Limits, ReadState};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    fs, io,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Copy)]
pub struct DiscoveryLimits {
    pub reader: Limits,
    pub entries: usize,
    pub files: usize,
    pub total_bytes: u64,
}
impl Default for DiscoveryLimits {
    fn default() -> Self {
        Self {
            reader: Limits::default(),
            entries: 10_000,
            files: 1_000,
            total_bytes: 256 * 1024 * 1024,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct DiscoveryDiagnostic {
    pub path: PathBuf,
    pub code: &'static str,
}
#[derive(Debug, Serialize)]
pub struct Session {
    pub path: PathBuf,
    pub session_ids: BTreeSet<String>,
    pub agent_ids: BTreeSet<String>,
    pub is_subagent: bool,
    pub state: ReadState,
    pub event_count: usize,
    pub branch_tips: Vec<String>,
    pub requires_branch_selection: bool,
    pub diagnostics: Vec<Diagnostic>,
}
#[derive(Debug, Serialize)]
pub struct Discovery<S = Session> {
    pub agent: &'static str,
    pub discovery_version: u8,
    pub root: PathBuf,
    pub project: PathBuf,
    pub partial: bool,
    pub files_inspected: usize,
    pub compatibility: &'static str,
    pub sessions: Vec<S>,
    pub diagnostics: Vec<DiscoveryDiagnostic>,
}
pub(crate) struct Inspection<S> {
    pub snapshot_bytes: u64,
    pub partial: bool,
    pub project_paths: BTreeSet<String>,
    pub session: Result<S, &'static str>,
}
#[derive(Clone, Copy)]
pub(crate) enum Layout {
    Claude,
    Codex,
}
// Sorting uses the file path and never assumes globally unique session IDs.
pub(crate) trait SessionPath {
    fn path(&self) -> &Path;
}
impl SessionPath for Session {
    fn path(&self) -> &Path {
        &self.path
    }
}
struct Scan<S> {
    report: Discovery<S>,
    layout: Layout,
    inspect: fn(&Path, Limits, bool) -> io::Result<Inspection<S>>,
    limits: DiscoveryLimits,
    entries: usize,
    bytes: u64,
    stopped: bool,
}

/// Lexical absolute normalization only: never resolve recorded paths on disk.
fn normalize(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let mut normalized = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::CurDir => (),
            _ => normalized.push(part.as_os_str()),
        }
    }
    Some(normalized)
}

pub fn discover(root: &Path, project: &Path, limits: DiscoveryLimits) -> io::Result<Discovery> {
    discover_with(root, project, limits, Layout::Claude, inspect_claude)
}

pub(crate) fn discover_with<S: SessionPath>(
    root: &Path,
    project: &Path,
    limits: DiscoveryLimits,
    layout: Layout,
    inspect: fn(&Path, Limits, bool) -> io::Result<Inspection<S>>,
) -> io::Result<Discovery<S>> {
    if limits.entries == 0
        || limits.files == 0
        || limits.total_bytes == 0
        || limits.reader.file_bytes == 0
        || limits.reader.line_bytes == 0
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "limits must be positive",
        ));
    }
    let root = root.canonicalize()?;
    if !root.is_dir() || root.to_str().is_none() || project.to_str().is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "root must be a directory; paths must be UTF-8",
        ));
    }
    // Root access failure is fatal. Failures below it are visible partial results.
    let entries = fs::read_dir(&root)?;
    let project = normalize(&std::env::current_dir()?.join(project)).unwrap();
    let mut scan = Scan {
        layout,
        inspect,
        report: Discovery {
            agent: match layout {
                Layout::Claude => "claude-code",
                Layout::Codex => "codex",
            },
            discovery_version: 1,
            root: root.clone(),
            project,
            partial: false,
            files_inspected: 0,
            compatibility: "unverified",
            sessions: vec![],
            diagnostics: vec![],
        },
        limits,
        entries: 0,
        bytes: 0,
        stopped: false,
    };
    scan.walk(&root, entries, 0);
    scan.report.sessions.sort_by(|a, b| a.path().cmp(b.path()));
    scan.report
        .diagnostics
        .sort_by(|a, b| a.path.cmp(&b.path).then(a.code.cmp(b.code)));
    Ok(scan.report)
}
impl<S> Scan<S> {
    fn warn(&mut self, path: &Path, code: &'static str) {
        self.report.partial = true;
        self.report.diagnostics.push(DiscoveryDiagnostic {
            path: path.to_path_buf(),
            code,
        });
    }
    fn descend(&mut self, path: &Path, level: usize) {
        match fs::read_dir(path) {
            Ok(entries) => self.walk(path, entries, level),
            Err(_) => self.warn(path, "directory_unreadable"),
        }
    }
    // Claude layout is unchanged; Codex accepts .jsonl at depths 0 through 3.
    fn walk(&mut self, dir: &Path, entries: fs::ReadDir, level: usize) {
        for entry in entries {
            if self.stopped {
                break;
            }
            if self.entries >= self.limits.entries {
                self.warn(dir, "entry_limit");
                self.stopped = true;
                break;
            }
            self.entries += 1;
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    self.warn(dir, "entry_unreadable");
                    continue;
                }
            };
            let path = entry.path();
            if path.to_str().is_none() {
                self.warn(dir, "non_utf8_path");
                continue;
            }
            let kind = match entry.file_type() {
                Ok(kind) => kind,
                Err(_) => {
                    self.warn(&path, "entry_unreadable");
                    continue;
                }
            };
            if kind.is_symlink() {
                self.warn(&path, "symlink_skipped");
                continue;
            }
            if kind.is_dir() {
                let descend = match self.layout {
                    Layout::Claude => level < 2 || (level == 2 && entry.file_name() == "subagents"),
                    Layout::Codex => level < 3,
                };
                if descend {
                    self.descend(&path, level + 1);
                } else if matches!(self.layout, Layout::Codex) {
                    self.warn(&path, "depth_limit");
                }
            } else if (matches!(self.layout, Layout::Codex) || level == 1 || level == 3)
                && path.extension().is_some_and(|ext| ext == "jsonl")
            {
                if kind.is_file() {
                    self.file(&path, level == 3);
                } else {
                    self.warn(&path, "non_regular_file");
                }
            }
        }
    }
    fn file(&mut self, path: &Path, nested: bool) {
        if self.report.files_inspected >= self.limits.files {
            self.warn(path, "file_count_limit");
            self.stopped = true;
            return;
        }
        let size = match fs::symlink_metadata(path) {
            Ok(meta) if meta.is_file() => meta.len(),
            Ok(_) => {
                self.warn(path, "non_regular_file");
                return;
            }
            Err(_) => {
                self.warn(path, "file_unreadable");
                return;
            }
        };
        if size > self.limits.reader.file_bytes {
            self.warn(path, "file_size_limit");
            return;
        }
        let reservation = size.max(1);
        if reservation > self.limits.total_bytes.saturating_sub(self.bytes) {
            self.warn(path, "total_byte_limit");
            self.stopped = true;
            return;
        }
        self.bytes += reservation;
        self.report.files_inspected += 1;
        let mut reader_limits = self.limits.reader;
        // Refuse growth between stat and open rather than reading beyond the reservation.
        reader_limits.file_bytes = reader_limits.file_bytes.min(reservation);
        let report = match (self.inspect)(path, reader_limits, nested) {
            Ok(report) => report,
            Err(_) => {
                self.warn(path, "file_unreadable");
                return;
            }
        };
        self.bytes = self.bytes - reservation + report.snapshot_bytes;
        if report.partial {
            self.warn(path, "partial_file");
        }
        let projects: BTreeSet<_> = report
            .project_paths
            .iter()
            .filter_map(|p| normalize(Path::new(p)))
            .collect();
        if report
            .project_paths
            .iter()
            .any(|p| !Path::new(p).is_absolute())
        {
            self.warn(path, "invalid_project_metadata");
            return;
        }
        if projects.is_empty() {
            self.warn(path, "missing_project_metadata");
            return;
        }
        if projects.len() > 1 {
            self.warn(path, "conflicting_project_metadata");
            return;
        }
        let session = match report.session {
            Ok(session) => session,
            Err(code) => {
                self.warn(path, code);
                return;
            }
        };
        if projects.contains(&self.report.project) {
            self.report.sessions.push(session);
        }
    }
}

fn inspect_claude(path: &Path, limits: Limits, nested: bool) -> io::Result<Inspection<Session>> {
    let report = claude::inspect(path, limits)?;
    let session_ids = report
        .records
        .iter()
        .filter_map(|r| r.source.session_id.clone())
        .collect();
    let agent_ids = report
        .records
        .iter()
        .filter_map(|r| r.source.agent_id.clone())
        .collect();
    let is_subagent = nested
        || report
            .records
            .iter()
            .any(|r| r.source.is_sidechain == Some(true) || r.source.agent_id.is_some());
    let session = Session {
        path: path.to_path_buf(),
        session_ids,
        agent_ids,
        is_subagent,
        state: report.state,
        event_count: report.events.len(),
        branch_tips: report.branch_tips,
        requires_branch_selection: report.requires_branch_selection,
        diagnostics: report.diagnostics,
    };
    Ok(Inspection {
        snapshot_bytes: report.snapshot_bytes,
        partial: session.state == ReadState::Partial,
        project_paths: report.project_paths,
        session: Ok(session),
    })
}
