use memory_pier::{
    claude::Limits,
    codex_discovery::{Discovery, discover},
    discovery::DiscoveryLimits,
};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

const PROJECT: &str = "/synthetic/project";
struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "memory-pier-codex-discovery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, rows: &[Value]) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            rows.iter().map(|r| format!("{r}\n")).collect::<String>(),
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
fn row(kind: &str, payload: Value) -> Value {
    json!({"timestamp":"2026-09-24T12:00:00Z","type":kind,"payload":payload})
}
fn meta(id: &str, cwd: &str) -> Value {
    row(
        "session_meta",
        json!({"id":id,"cwd":cwd,"cli_version":"synthetic"}),
    )
}
fn message() -> Value {
    row(
        "response_item",
        json!({"type":"message","role":"user","content":[{"type":"input_text","text":"PRIVATE SYNTHETIC MESSAGE"}]}),
    )
}
fn scan(tree: &Tree) -> Discovery {
    discover(&tree.0, Path::new(PROJECT), DiscoveryLimits::default()).unwrap()
}
fn has(report: &Discovery, code: &str) -> bool {
    report.diagnostics.iter().any(|d| d.code == code)
}
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/codex-sessions")
}
fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_memory-pier"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn discovers_physical_files_without_content_or_inferred_relationships() {
    let tree = Tree::new();
    let first = tree.write(
        "2026/09/24/session.jsonl",
        &[meta("same-id", PROJECT), message()],
    );
    tree.write("copy.jsonl", &[meta("same-id", PROJECT), message()]);
    tree.write(
        "workers/worker.jsonl",
        &[meta("other-id", PROJECT), message()],
    );
    tree.write(
        "project/session.jsonl",
        &[meta("unrelated", "/elsewhere/project"), message()],
    );
    let before = fs::read(&first).unwrap();
    let modified = fs::metadata(&first).unwrap().modified().unwrap();
    let report = scan(&tree);
    assert_eq!(report.agent, "codex");
    assert_eq!(report.compatibility, "unverified");
    assert!(!report.partial);
    assert_eq!(report.files_inspected, 4);
    assert_eq!(report.sessions.len(), 3);
    assert_eq!(
        report
            .sessions
            .iter()
            .filter(|s| s.session_id == "same-id")
            .count(),
        2
    );
    assert!(report.sessions.windows(2).all(|s| s[0].path < s[1].path));
    assert!(
        report
            .sessions
            .iter()
            .all(|s| s.event_count == 1 && s.record_count == 2)
    );
    let serialized = serde_json::to_string(&report).unwrap();
    for omitted in [
        "PRIVATE SYNTHETIC MESSAGE",
        "branch_tips",
        "requires_branch_selection",
        "is_subagent",
    ] {
        assert!(!serialized.contains(omitted));
    }
    assert_eq!(fs::read(&first).unwrap(), before);
    assert_eq!(fs::metadata(first).unwrap().modified().unwrap(), modified);
}

#[test]
fn project_matching_is_lexical_and_never_uses_message_text_or_filenames() {
    let tree = Tree::new();
    tree.write(
        "strange/arbitrary.jsonl",
        &[meta("match", "/synthetic/parent/../project/./"), message()],
    );
    tree.write("synthetic/project/session.jsonl", &[meta("wrong","/different/project"),row("response_item",json!({"type":"message","role":"user","content":[{"type":"input_text","text":PROJECT}]}))]);
    tree.write(
        "subdirectory.jsonl",
        &[meta("sub", "/synthetic/project/sub")],
    );
    tree.write(
        "worktree.jsonl",
        &[meta("worktree", "/synthetic/project-worktree")],
    );
    let report = scan(&tree);
    assert!(!report.partial);
    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.sessions[0].session_id, "match");
    let relative = std::env::current_dir()
        .unwrap()
        .join("synthetic-missing-project");
    tree.write(
        "relative-query.jsonl",
        &[meta("relative", relative.to_str().unwrap())],
    );
    let report = discover(
        &tree.0,
        Path::new("./synthetic-missing-project"),
        DiscoveryLimits::default(),
    )
    .unwrap();
    assert_eq!(report.project, relative);
    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.sessions[0].session_id, "relative");
}

#[test]
fn empty_root_and_other_project_are_distinct_from_invalid_root() {
    let tree = Tree::new();
    assert!(!scan(&tree).partial);
    assert!(scan(&tree).sessions.is_empty());
    tree.write("other.jsonl", &[meta("other", "/other")]);
    assert!(!scan(&tree).partial);
    assert!(scan(&tree).sessions.is_empty());
    for root in [tree.0.join("missing"), tree.0.join("other.jsonl")] {
        assert!(discover(&root, Path::new(PROJECT), DiscoveryLimits::default()).is_err());
    }
}

#[test]
fn unclassifiable_and_mixed_metadata_are_excluded_with_diagnostics() {
    let tree = Tree::new();
    tree.write("empty.jsonl", &[]);
    tree.write(
        "no-project.jsonl",
        &[row("session_meta", json!({"id":"missing"})), message()],
    );
    tree.write("relative.jsonl", &[meta("relative", "relative/project")]);
    tree.write(
        "conflicting-projects.jsonl",
        &[
            meta("mixed", PROJECT),
            row("turn_context", json!({"cwd":"/other","turn_id":"turn"})),
        ],
    );
    tree.write(
        "conflicting-sessions.jsonl",
        &[
            meta("one", PROJECT),
            message(),
            meta("two", PROJECT),
            message(),
        ],
    );
    tree.write(
        "no-session.jsonl",
        &[
            row("turn_context", json!({"cwd":PROJECT,"turn_id":"turn"})),
            message(),
        ],
    );
    tree.write("empty-id.jsonl", &[meta("", PROJECT), message()]);
    // A Claude log is not inferred to be Codex from its extension or cwd.
    tree.write("claude.jsonl",&[json!({"type":"user","uuid":"claude","sessionId":"claude-session","cwd":PROJECT,"message":{"role":"user","content":"PRIVATE CLAUDE CONTENT"}})]);
    let report = scan(&tree);
    assert!(report.partial);
    assert!(report.sessions.is_empty());
    for code in [
        "missing_project_metadata",
        "invalid_project_metadata",
        "conflicting_project_metadata",
        "missing_session_metadata",
        "conflicting_session_metadata",
        "partial_file",
    ] {
        assert!(has(&report, code), "{code}");
    }
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("PRIVATE CLAUDE CONTENT"));
    assert!(!serialized.contains("PRIVATE SYNTHETIC MESSAGE"));
}

#[test]
fn turn_cwd_and_metadata_only_sessions_are_supported_without_claiming_exportability() {
    let tree = Tree::new();
    tree.write("header-only.jsonl", &[meta("header-only", PROJECT)]);
    tree.write(
        "turn-cwd.jsonl",
        &[
            row("session_meta", json!({"id":"turn-project"})),
            row("turn_context", json!({"cwd":PROJECT,"turn_id":"turn"})),
            message(),
        ],
    );
    let report = scan(&tree);
    assert!(!report.partial);
    assert_eq!(report.sessions.len(), 2);
    let empty = report
        .sessions
        .iter()
        .find(|s| s.session_id == "header-only")
        .unwrap();
    assert_eq!(empty.event_count, 0);
    assert_eq!(empty.state, memory_pier::claude::ReadState::Empty);
}

#[test]
fn partial_files_keep_diagnostics_even_when_the_project_does_not_match() {
    let tree = Tree::new();
    let matched = tree.write(
        "partial.jsonl",
        &[
            meta("partial", PROJECT),
            message(),
            row(
                "event_msg",
                json!({"type":"thread_rolled_back","num_turns":1}),
            ),
        ],
    );
    fs::OpenOptions::new()
        .append(true)
        .open(&matched)
        .unwrap()
        .write_all(b"{\"type\":")
        .unwrap();
    let other = tree.write("other.jsonl", &[meta("other", "/other"), message()]);
    fs::OpenOptions::new()
        .append(true)
        .open(&other)
        .unwrap()
        .write_all(b"{broken\n")
        .unwrap();
    let report = scan(&tree);
    assert!(report.partial);
    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.sessions[0].event_count, 1);
    for code in [
        "omitted_event_msg",
        "incomplete_final_line",
        "unverified_compatibility",
    ] {
        assert!(
            report.sessions[0]
                .diagnostics
                .iter()
                .any(|d| d.code == code)
        );
    }
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|d| d.code == "partial_file")
            .count(),
        2
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.path == other.canonicalize().unwrap() && d.code == "partial_file")
    );
}

#[test]
fn depth_limit_is_visible_and_does_not_read_deeper_files() {
    let tree = Tree::new();
    for (name, id) in [
        ("root.jsonl", "zero"),
        ("a/one.jsonl", "one"),
        ("a/b/two.jsonl", "two"),
        ("a/b/c/three.jsonl", "three"),
        ("a/b/c/d/four.jsonl", "four"),
    ] {
        tree.write(name, &[meta(id, PROJECT), message()]);
    }
    tree.write("ignored.txt", &[meta("not-jsonl", PROJECT)]);
    let report = scan(&tree);
    assert!(report.partial);
    assert!(has(&report, "depth_limit"));
    assert_eq!(report.files_inspected, 4);
    assert_eq!(report.sessions.len(), 4);
    assert!(!report.sessions.iter().any(|s| s.session_id == "four"));
}

#[test]
fn entry_file_byte_and_line_limits_remain_observable() {
    let tree = Tree::new();
    let path = tree.write("one.jsonl", &[meta("one", PROJECT), message()]);
    tree.write("two.jsonl", &[meta("two", PROJECT), message()]);
    let size = fs::metadata(&path).unwrap().len();
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
                total_bytes: size,
                ..defaults
            },
            "total_byte_limit",
        ),
        (
            DiscoveryLimits {
                reader: Limits {
                    file_bytes: 1,
                    ..defaults.reader
                },
                ..defaults
            },
            "file_size_limit",
        ),
    ] {
        let report = discover(&tree.0, Path::new(PROJECT), limits).unwrap();
        assert!(report.partial);
        assert!(has(&report, code), "{code}");
        assert!(report.files_inspected <= 1);
    }
    let tree = Tree::new();
    let path = tree.write("lines.jsonl", &[meta("line-budget", PROJECT), message()]);
    let long = row("event_msg", json!({"message":"x".repeat(1024)}));
    fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(format!("{long}\n").as_bytes())
        .unwrap();
    let report = discover(
        &tree.0,
        Path::new(PROJECT),
        DiscoveryLimits {
            reader: Limits {
                line_bytes: 512,
                ..defaults.reader
            },
            ..defaults
        },
    )
    .unwrap();
    assert_eq!(report.sessions.len(), 1);
    assert!(
        report.sessions[0]
            .diagnostics
            .iter()
            .any(|d| d.code == "line_limit")
    );
    assert!(has(&report, "partial_file"));
    for limits in [
        DiscoveryLimits {
            entries: 0,
            ..defaults
        },
        DiscoveryLimits {
            files: 0,
            ..defaults
        },
        DiscoveryLimits {
            total_bytes: 0,
            ..defaults
        },
        DiscoveryLimits {
            reader: Limits {
                file_bytes: 0,
                ..defaults.reader
            },
            ..defaults
        },
        DiscoveryLimits {
            reader: Limits {
                line_bytes: 0,
                ..defaults.reader
            },
            ..defaults
        },
    ] {
        assert!(discover(&tree.0, Path::new(PROJECT), limits).is_err());
    }
}

#[cfg(unix)]
#[test]
fn symlinks_and_special_files_are_skipped_while_explicit_root_alias_is_usable() {
    use std::os::unix::fs::symlink;
    let tree = Tree::new();
    let outside = Tree::new();
    let target = outside.write("private.jsonl", &[meta("outside", PROJECT), message()]);
    symlink(&target, tree.0.join("file.jsonl")).unwrap();
    symlink(&outside.0, tree.0.join("directory")).unwrap();
    // A Unix socket must not be opened by the JSONL reader.
    let socket = std::os::unix::net::UnixListener::bind(tree.0.join("socket.jsonl")).unwrap();
    let root_alias = outside.0.join("root-alias");
    symlink(&tree.0, &root_alias).unwrap();
    let report = discover(&root_alias, Path::new(PROJECT), DiscoveryLimits::default()).unwrap();
    assert_eq!(report.root, tree.0.canonicalize().unwrap());
    assert!(report.sessions.is_empty());
    assert_eq!(report.files_inspected, 0);
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|d| d.code == "symlink_skipped")
            .count(),
        2
    );
    assert!(has(&report, "non_regular_file"));
    drop(socket);
}

#[cfg(unix)]
#[test]
fn non_utf8_root_and_project_arguments_are_errors() {
    use std::os::unix::ffi::OsStringExt;
    let tree = Tree::new();
    let name = std::ffi::OsString::from_vec(b"invalid-\xff".to_vec());
    assert!(discover(&tree.0, Path::new(&name), DiscoveryLimits::default()).is_err());
    assert!(
        discover(
            &tree.0.join(&name),
            Path::new(PROJECT),
            DiscoveryLimits::default()
        )
        .is_err()
    );
}

// Linux permits byte names that the macOS filesystem rejects at creation.
#[cfg(target_os = "linux")]
#[test]
fn non_utf8_entries_are_reported_without_lossy_replacement() {
    use std::os::unix::ffi::OsStringExt;
    let tree = Tree::new();
    let name = std::ffi::OsString::from_vec(b"invalid-\xff.jsonl".to_vec());
    fs::write(
        tree.0.join(&name),
        format!("{}\n", meta("invalid-path", PROJECT)),
    )
    .unwrap();
    let report = scan(&tree);
    assert!(has(&report, "non_utf8_path"));
    assert_eq!(report.files_inspected, 0);
    assert!(report.sessions.is_empty());
}

#[cfg(unix)]
#[test]
fn permission_failures_do_not_hide_readable_sessions() {
    use std::os::unix::fs::PermissionsExt;
    let tree = Tree::new();
    let file = tree.write("denied.jsonl", &[meta("denied", PROJECT)]);
    let nested = tree.write("locked/session.jsonl", &[meta("locked", PROJECT)]);
    tree.write("good.jsonl", &[meta("good", PROJECT)]);
    fs::set_permissions(&file, fs::Permissions::from_mode(0o0)).unwrap();
    fs::set_permissions(nested.parent().unwrap(), fs::Permissions::from_mode(0o0)).unwrap();
    let file_denied = fs::File::open(&file).is_err();
    let directory_denied = fs::read_dir(nested.parent().unwrap()).is_err();
    let report = scan(&tree);
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(nested.parent().unwrap(), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(report.sessions.iter().any(|s| s.session_id == "good"));
    if file_denied {
        assert!(has(&report, "file_unreadable"));
    } else {
        eprintln!("mode-000 file readable by this process; denial assertion not applicable");
    }
    if directory_denied {
        assert!(has(&report, "directory_unreadable"));
    } else {
        eprintln!("mode-000 directory readable by this process; denial assertion not applicable");
    }
}

#[test]
fn cli_fixture_selection_roundtrips_through_inspect_export_and_verify() {
    let root = fixture();
    let root = root.to_str().unwrap();
    let result = cli(&["sessions-codex", "--root", root, "--project", PROJECT]);
    assert_eq!(result.status.code(), Some(0));
    let discovery: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(discovery["files_inspected"], 4);
    assert_eq!(discovery["sessions"].as_array().unwrap().len(), 3);
    let reversed = cli(&["sessions-codex", "--project", PROJECT, "--root", root]);
    assert_eq!(result.stdout, reversed.stdout);
    let source = discovery["sessions"][0]["path"].as_str().unwrap();
    let inspected = cli(&["inspect-codex", source]);
    assert_eq!(inspected.status.code(), Some(0));
    let events: Value = serde_json::from_slice(&inspected.stdout).unwrap();
    assert_eq!(events["events"].as_array().unwrap().len(), 1);
    let temp = Tree::new();
    let output = temp.0.join("bundle");
    let exported = cli(&["export-codex", source, "--output", output.to_str().unwrap()]);
    assert_eq!(exported.status.code(), Some(0));
    assert_eq!(
        cli(&["verify", output.to_str().unwrap()]).status.code(),
        Some(0)
    );
    assert_eq!(
        fs::read_to_string(output.join("history.jsonl"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[test]
fn cli_has_no_implicit_roots_or_claude_selection_and_reports_partial() {
    let tree = Tree::new();
    let root = tree.0.to_str().unwrap();
    for args in [
        vec!["sessions-codex"],
        vec!["sessions-codex", "--root", root],
        vec!["sessions-codex", "--project", PROJECT],
        vec!["sessions-codex", "--root", root, "--root", root],
        vec![
            "sessions-codex",
            "--root",
            root,
            "--project",
            PROJECT,
            "--leaf",
            "id",
        ],
    ] {
        assert_eq!(cli(&args).status.code(), Some(64));
    }
    let missing = tree.0.join("missing");
    assert_eq!(
        cli(&[
            "sessions-codex",
            "--root",
            missing.to_str().unwrap(),
            "--project",
            PROJECT
        ])
        .status
        .code(),
        Some(1)
    );
    tree.write("empty.jsonl", &[]);
    let result = cli(&["sessions-codex", "--root", root, "--project", PROJECT]);
    assert_eq!(result.status.code(), Some(2));
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["partial"], true);
    assert!(report["sessions"].as_array().unwrap().is_empty());
}
