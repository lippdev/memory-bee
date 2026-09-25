use memory_bee::{
    claude::{Limits, ReadState, inspect},
    discovery::{DiscoveryLimits, discover},
    selection::select,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "memory-bee-navigation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, records: &[Value]) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            records.iter().map(|v| format!("{v}\n")).collect::<String>(),
        )
        .unwrap();
        path
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn record(id: &str, parent: Option<&str>, cwd: &str) -> Value {
    json!({"type":"user", "uuid":id, "parentUuid":parent, "sessionId":"synthetic-session", "cwd":cwd, "message":{"role":"user","content":format!("synthetic {id}")}})
}
fn scan(tree: &Tree) -> memory_bee::discovery::Discovery {
    discover(
        &tree.0,
        Path::new("/synthetic/project"),
        DiscoveryLimits::default(),
    )
    .unwrap()
}
fn parsed(path: &Path) -> memory_bee::claude::Report {
    inspect(path, Limits::default()).unwrap()
}

#[test]
fn discovery_uses_metadata_not_encoded_names_and_does_not_return_content() {
    let tree = Tree::new();
    let matching = tree.write(
        "arbitrary-name/good.jsonl",
        &[record("u", None, "/synthetic/project/./")],
    );
    tree.write(
        "-synthetic-project/impostor.jsonl",
        &[record("u", None, "/different/project")],
    );
    let before = fs::read(&matching).unwrap();
    let report = scan(&tree);
    assert!(!report.partial);
    assert_eq!(report.files_inspected, 2);
    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.sessions[0].path, matching.canonicalize().unwrap());
    assert_eq!(report.sessions[0].branch_tips, vec!["u"]);
    assert!(report.sessions[0].session_ids.contains("synthetic-session"));
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("synthetic u")
    );
    assert_eq!(fs::read(&matching).unwrap(), before);
}
#[test]
fn empty_root_is_distinct_from_missing_root_or_non_directory() {
    let tree = Tree::new();
    assert!(!scan(&tree).partial);
    assert!(scan(&tree).sessions.is_empty());
    let file = tree.write("file", &[]);
    for path in [tree.0.join("missing"), file] {
        assert!(
            discover(
                &path,
                Path::new("/synthetic/project"),
                DiscoveryLimits::default()
            )
            .is_err()
        );
    }
}
#[test]
fn unclassifiable_and_conflicting_files_are_diagnosed() {
    let tree = Tree::new();
    tree.write("p/empty.jsonl", &[]);
    let mut missing = record("u", None, "/synthetic/project");
    missing.as_object_mut().unwrap().remove("cwd");
    tree.write("p/missing.jsonl", &[missing]);
    tree.write(
        "p/conflict.jsonl",
        &[
            record("u", None, "/synthetic/project"),
            record("v", Some("u"), "/elsewhere"),
        ],
    );
    tree.write("p/relative.jsonl", &[record("u", None, "relative")]);
    let report = scan(&tree);
    assert!(report.partial);
    assert!(report.sessions.is_empty());
    for code in [
        "missing_project_metadata",
        "conflicting_project_metadata",
        "invalid_project_metadata",
    ] {
        assert!(report.diagnostics.iter().any(|d| d.code == code));
    }
}
#[test]
fn subagents_stay_in_separate_files_and_results_are_sorted() {
    let tree = Tree::new();
    let mut sub = record("agent-root", None, "/synthetic/project");
    sub["agentId"] = json!("worker");
    sub["isSidechain"] = json!(true);
    tree.write("p/session/subagents/agent-worker.jsonl", &[sub]);
    tree.write(
        "p/session.jsonl",
        &[record("root", None, "/synthetic/project")],
    );
    tree.write(
        "p/memory/irrelevant.jsonl",
        &[record("noise", None, "/synthetic/project")],
    );
    let report = scan(&tree);
    assert_eq!(report.sessions.len(), 2);
    let main = report.sessions.iter().find(|s| !s.is_subagent).unwrap();
    let sub = report.sessions.iter().find(|s| s.is_subagent).unwrap();
    assert!(sub.agent_ids.contains("worker"));
    assert_eq!(main.event_count, 1);
    assert!(report.sessions.windows(2).all(|w| w[0].path <= w[1].path));
}
#[test]
fn partial_matching_session_retains_diagnostics() {
    let tree = Tree::new();
    let path = tree.write("p/broken.jsonl", &[record("u", None, "/synthetic/project")]);
    use std::io::Write;
    fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(b"{broken")
        .unwrap();
    let report = scan(&tree);
    assert!(report.partial);
    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.sessions[0].state, ReadState::Partial);
    assert!(
        report.sessions[0]
            .diagnostics
            .iter()
            .any(|d| d.code == "invalid_json")
    );
}
#[test]
fn all_discovery_budgets_are_visible() {
    let tree = Tree::new();
    tree.write("p/a.jsonl", &[record("u", None, "/synthetic/project")]);
    tree.write("p/b.jsonl", &[record("v", None, "/synthetic/project")]);
    let defaults = DiscoveryLimits::default();
    for (limits, code) in [
        (
            DiscoveryLimits {
                entries: 1,
                ..defaults
            },
            "entry_limit",
        ),
        (
            DiscoveryLimits {
                files: 1,
                ..defaults
            },
            "file_count_limit",
        ),
        (
            DiscoveryLimits {
                total_bytes: 1,
                ..defaults
            },
            "total_byte_limit",
        ),
        (
            DiscoveryLimits {
                reader: Limits {
                    file_bytes: 1,
                    ..Limits::default()
                },
                ..defaults
            },
            "file_size_limit",
        ),
    ] {
        let report = discover(&tree.0, Path::new("/synthetic/project"), limits).unwrap();
        assert!(report.partial);
        assert!(report.diagnostics.iter().any(|d| d.code == code));
    }
    assert!(
        discover(
            &tree.0,
            Path::new("/synthetic/project"),
            DiscoveryLimits {
                entries: 0,
                ..defaults
            }
        )
        .is_err()
    );
}
#[cfg(unix)]
#[test]
fn symlinks_are_skipped_without_reading_targets() {
    use std::os::unix::fs::symlink;
    let tree = Tree::new();
    let outside = Tree::new();
    let file = outside.write("secret.jsonl", &[record("u", None, "/synthetic/project")]);
    fs::create_dir(tree.0.join("p")).unwrap();
    symlink(file, tree.0.join("p/link.jsonl")).unwrap();
    symlink(&outside.0, tree.0.join("linked-project")).unwrap();
    let report = scan(&tree);
    assert!(report.partial);
    assert_eq!(report.files_inspected, 0);
    assert!(report.sessions.is_empty());
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|d| d.code == "symlink_skipped")
            .count(),
        2
    );
}
#[cfg(unix)]
#[test]
fn permission_failures_are_partial_and_do_not_hide_readable_sessions() {
    use std::os::unix::fs::PermissionsExt;
    let tree = Tree::new();
    let denied = tree.write(
        "denied/private.jsonl",
        &[record("u", None, "/synthetic/project")],
    );
    tree.write(
        "good/public.jsonl",
        &[record("v", None, "/synthetic/project")],
    );
    fs::set_permissions(&denied, fs::Permissions::from_mode(0o0)).unwrap();
    let inaccessible = fs::File::open(&denied).is_err();
    let report = scan(&tree);
    fs::set_permissions(&denied, fs::Permissions::from_mode(0o600)).unwrap();
    if inaccessible {
        assert!(report.partial);
        assert_eq!(report.sessions.len(), 1);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "file_unreadable")
        );
    } else {
        eprintln!("permission test: process can read mode-000 files; assertions not applicable");
    }
}
#[test]
fn selecting_a_fork_keeps_all_ancestor_blocks_and_reports_exclusions() {
    let tree = Tree::new();
    let mut a = record("a", Some("u"), "/synthetic/project");
    a["message"]["content"] =
        json!([{"type":"text","text":"first"},{"type":"text","text":"second"}]);
    let path = tree.write(
        "fork.jsonl",
        &[
            record("u", None, "/synthetic/project"),
            a,
            record("b", Some("u"), "/synthetic/project"),
        ],
    );
    let r = parsed(&path);
    assert_eq!(r.branch_tips, vec!["a", "b"]);
    assert!(r.requires_branch_selection);
    let r = select(r, "a").unwrap();
    assert_eq!(r.events.len(), 3);
    assert_eq!(r.events[2].source.block, Some(2));
    assert_eq!(r.events[2].sequence, 3);
    assert_eq!(r.events[2].source.line, 2);
    assert!(!r.requires_branch_selection);
    assert_eq!(r.selection.as_ref().unwrap().excluded_records, 1);
    assert_eq!(r.selection.as_ref().unwrap().excluded_events, 1);
    assert!(r.diagnostics.iter().any(|d| d.code == "branch_excluded"));
    assert_eq!(r.compatibility, "unverified");
    assert!(select(parsed(&path), "u").is_err());
    assert!(select(r, "b").is_err());
}
#[test]
fn selecting_missing_duplicate_forward_or_unidentified_ancestry_fails() {
    let tree = Tree::new();
    let mut missing_parent = record("u", None, "/synthetic/project");
    missing_parent.as_object_mut().unwrap().remove("parentUuid");
    let mut missing_session = record("u", None, "/synthetic/project");
    missing_session.as_object_mut().unwrap().remove("sessionId");
    let cases = vec![
        vec![record("u", Some("missing"), "/synthetic/project")],
        vec![
            record("u", None, "/synthetic/project"),
            record("u", None, "/synthetic/project"),
        ],
        vec![
            record("u", Some("later"), "/synthetic/project"),
            record("later", None, "/synthetic/project"),
        ],
        vec![missing_parent],
        vec![missing_session],
        vec![record("u", Some("u"), "/synthetic/project")],
    ];
    for records in cases {
        let path = tree.write("bad.jsonl", &records);
        assert!(select(parsed(&path), "u").is_err());
    }
}
#[test]
fn selection_rejects_cross_session_and_cross_agent_but_accepts_standalone_subagent() {
    let tree = Tree::new();
    for key in ["sessionId", "agentId", "isSidechain"] {
        let mut child = record("a", Some("u"), "/synthetic/project");
        child[key] = if key == "isSidechain" {
            json!(true)
        } else {
            json!("another")
        };
        let path = tree.write(
            "bad.jsonl",
            &[record("u", None, "/synthetic/project"), child],
        );
        assert!(select(parsed(&path), "a").is_err());
    }
    let mut sub = record("s", None, "/synthetic/project");
    sub["agentId"] = json!("worker");
    sub["isSidechain"] = json!(true);
    let path = tree.write("sub.jsonl", &[sub]);
    assert!(select(parsed(&path), "s").is_ok());
}
#[test]
fn selection_preserves_loss_diagnostics_and_does_not_invent_clean_history() {
    let tree = Tree::new();
    let path = tree.write(
        "partial.jsonl",
        &[
            record("u", None, "/synthetic/project"),
            json!({"type":"future"}),
            record("a", Some("u"), "/synthetic/project"),
        ],
    );
    let report = select(parsed(&path), "a").unwrap();
    assert_eq!(report.state, ReadState::Partial);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "unknown_record")
    );
}
#[test]
fn cli_discovers_and_selects_with_explicit_arguments() {
    let tree = Tree::new();
    let path = tree.write(
        "folder with spaces/session.jsonl",
        &[record("u", None, "/synthetic/project")],
    );
    let bin = env!("CARGO_BIN_EXE_memory-bee");
    let result = Command::new(bin)
        .args(["sessions", "--project", "/synthetic/project", "--root"])
        .arg(&tree.0)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["sessions"].as_array().unwrap().len(), 1);
    let result = Command::new(bin)
        .arg("inspect")
        .arg(&path)
        .args(["--leaf", "u"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["selection"]["leaf_uuid"], "u");
    let result = Command::new(bin)
        .arg("inspect")
        .arg(&path)
        .args(["--leaf", "missing"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    let result = Command::new(bin)
        .args(["sessions", "--root"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(64));
    tree.write("p/empty.jsonl", &[]);
    let result = Command::new(bin)
        .args(["sessions", "--root"])
        .arg(&tree.0)
        .args(["--project", "/synthetic/project"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
}

#[cfg(unix)]
#[test]
fn unreadable_directory_is_reported_and_permissions_are_restored() {
    use std::os::unix::fs::PermissionsExt;
    let tree = Tree::new();
    let path = tree.0.join("denied");
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o0)).unwrap();
    let inaccessible = fs::read_dir(&path).is_err();
    let report = scan(&tree);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    if inaccessible {
        assert!(report.partial);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "directory_unreadable")
        );
    } else {
        eprintln!("directory permission assertions not applicable for privileged process");
    }
}

#[test]
fn normalized_project_paths_and_missing_project_directories_are_supported() {
    let tree = Tree::new();
    tree.write(
        "p/a.jsonl",
        &[
            record("u", None, "/synthetic/old/../project/"),
            record("a", Some("u"), "/synthetic/project"),
        ],
    );
    let report = scan(&tree);
    assert_eq!(report.sessions.len(), 1);
    assert!(!report.partial);
}

#[test]
fn partial_unrelated_files_do_not_make_discovery_look_complete() {
    let tree = Tree::new();
    tree.write(
        "p/other.jsonl",
        &[
            record("u", None, "/other/project"),
            json!({"type":"future"}),
        ],
    );
    let report = scan(&tree);
    assert!(report.partial);
    assert!(report.sessions.is_empty());
    assert!(report.diagnostics.iter().any(|d| d.code == "partial_file"));
}
