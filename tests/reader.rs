use memory_pier::claude::{Limits, ReadState, Report, inspect};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/claude")
        .join(name)
}
fn read(name: &str) -> Report {
    inspect(&fixture(name), Limits::default()).unwrap()
}
fn has(r: &Report, code: &str) -> bool {
    r.diagnostics.iter().any(|d| d.code == code)
}
struct Input(PathBuf);
impl Input {
    fn new(bytes: &[u8]) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "memory-pier-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("session.jsonl");
        fs::write(&path, bytes).unwrap();
        Self(path)
    }
    fn read(&self) -> Report {
        inspect(&self.0, Limits::default()).unwrap()
    }
}
impl Drop for Input {
    fn drop(&mut self) {
        fs::remove_dir_all(self.0.parent().unwrap()).unwrap();
    }
}

#[test]
fn normal_session_preserves_order_origin_and_source_bytes() {
    let p = fixture("basic.jsonl");
    let before = fs::read(&p).unwrap();
    let modified = fs::metadata(&p).unwrap().modified().unwrap();
    let r = inspect(&p, Limits::default()).unwrap();
    assert_eq!(r.state, ReadState::Read);
    assert_eq!(r.events.len(), 2);
    assert_eq!(r.events[0].sequence, 1);
    assert_eq!(r.events[1].source.line, 2);
    assert_eq!(r.events[1].source.block, Some(1));
    assert_eq!(r.events[1].source.parent_id.as_deref(), Some("u1"));
    assert_eq!(
        r.events[0].timestamp.as_deref(),
        Some("2026-09-24T12:00:00Z")
    );
    assert_eq!(r.events[0].provenance, "extracted");
    assert!(!r.requires_branch_selection);
    assert_eq!(fs::read(&p).unwrap(), before);
    assert_eq!(fs::metadata(&p).unwrap().modified().unwrap(), modified);
    assert_eq!(r.compatibility, "unverified");
}
#[test]
fn truncated_tail_preserves_valid_prefix() {
    let r = read("truncated.jsonl");
    assert_eq!(r.events.len(), 2);
    assert_eq!(r.state, ReadState::Partial);
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == "incomplete_final_line" && d.line == Some(3))
    );
}
#[test]
fn empty_and_whitespace_are_explicit() {
    for r in [read("empty.jsonl"), Input::new(b" \r\n\n\t").read()] {
        assert_eq!(r.state, ReadState::Empty);
        assert!(r.events.is_empty());
    }
}
#[test]
fn tools_are_not_human_requests_and_images_are_omitted() {
    let r = read("tools.jsonl");
    assert_eq!(r.events.len(), 5);
    assert_eq!(r.events[2].kind, "tool_call");
    assert_eq!(r.events[2].tool_name.as_deref(), Some("Bash"));
    assert_eq!(r.events[3].role, "tool");
    assert_eq!(r.events[3].tool_is_error, Some(false));
    assert_eq!(r.events[3].tool_id, r.events[2].tool_id);
    assert_eq!(r.events[3].text, "synthetic-only");
    assert_eq!(r.events[4].role, "user");
    assert!(has(&r, "omitted_tool_content"));
    assert!(
        !serde_json::to_string(&r)
            .unwrap()
            .contains("SYNTHETIC_IMAGE_PAYLOAD")
    );
}
#[test]
fn forks_remain_in_physical_order_with_selection_required() {
    let r = read("branches.jsonl");
    assert_eq!(r.events[1].text, "Ramo A");
    assert_eq!(r.events[2].text, "Ramo B");
    assert!(r.requires_branch_selection);
    assert!(has(&r, "branch_selection_required"));
    assert_eq!(
        r.events[2].source.agent_id.as_deref(),
        Some("synthetic-agent")
    );
}
#[test]
fn checkpoints_are_labeled_and_compaction_reports_loss() {
    let r = read("compaction.jsonl");
    assert_eq!(r.state, ReadState::Partial);
    assert_eq!(r.events.len(), 2);
    assert!(
        r.events
            .iter()
            .all(|e| e.kind == "checkpoint" && e.provenance == "checkpoint")
    );
    assert!(has(&r, "compaction"));
}
#[test]
fn unknown_versions_and_sensitive_blocks_are_not_silently_accepted() {
    let r = read("unsupported.jsonl");
    assert!(r.observed_versions.contains("999.0"));
    assert!(has(&r, "unknown_record"));
    assert!(has(&r, "omitted_block"));
    assert_eq!(r.events.len(), 1);
    assert!(!serde_json::to_string(&r).unwrap().contains("PAYLOAD"));
}
#[test]
fn malformed_middle_and_invalid_utf8_recover_following_records() {
    let good = fs::read(fixture("basic.jsonl")).unwrap();
    let mut bytes = b"not json\n\xff\n".to_vec();
    bytes.extend(good);
    let r = Input::new(&bytes).read();
    assert_eq!(r.state, ReadState::Partial);
    assert_eq!(r.events.len(), 2);
    assert_eq!(r.events[0].source.line, 3);
    assert!(has(&r, "invalid_json"));
    assert!(has(&r, "invalid_utf8"));
}
#[test]
fn valid_final_line_needs_no_newline_and_crlf_is_supported() {
    let text = fs::read_to_string(fixture("basic.jsonl")).unwrap();
    let r = Input::new(text.trim_end().replace('\n', "\r\n").as_bytes()).read();
    assert_eq!(r.state, ReadState::Read);
    assert_eq!(r.events.len(), 2);
}
#[test]
fn line_limit_drains_and_recovers_and_file_limit_is_fatal() {
    let mut bytes = vec![b'x'; 2000];
    bytes.push(b'\n');
    bytes.extend(fs::read(fixture("basic.jsonl")).unwrap());
    let input = Input::new(&bytes);
    let r = inspect(
        &input.0,
        Limits {
            file_bytes: 4096,
            line_bytes: 1024,
        },
    )
    .unwrap();
    assert!(has(&r, "line_limit"));
    assert_eq!(r.events.len(), 2);
    assert_eq!(r.events[0].source.line, 2);
    let error = inspect(
        &input.0,
        Limits {
            file_bytes: 10,
            line_bytes: 1024,
        },
    )
    .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(
        inspect(
            &input.0,
            Limits {
                file_bytes: 0,
                line_bytes: 0
            }
        )
        .is_err()
    );
}
#[test]
fn malformed_structures_report_loss_without_dumping_payload() {
    let r = Input::new(
        br#"null
{"type":"user","message":{"role":"assistant","content":"PAYLOAD"}}
{"type":"assistant","message":{"content":12}}
{"type":"assistant","message":{"content":[{"type":"text"},{"type":"tool_use"}]}}
"#,
    )
    .read();
    assert_eq!(r.state, ReadState::Partial);
    assert!(r.events.is_empty());
    for code in [
        "unknown_record",
        "role_mismatch",
        "invalid_content",
        "invalid_block",
    ] {
        assert!(has(&r, code));
    }
}
#[test]
fn missing_metadata_stays_null_and_bad_timestamp_is_diagnosed() {
    let r = Input::new(br#"{"type":"user","timestamp":"yesterday","message":{"content":"test"}}"#)
        .read();
    let e = &r.events[0];
    assert!(e.timestamp.is_none());
    assert!(e.source.id.is_none());
    assert!(e.source.session_id.is_none());
    assert!(has(&r, "invalid_timestamp"));
    assert!(r.requires_branch_selection);
}
#[test]
fn duplicate_missing_and_forward_parents_are_visible() {
    let r = Input::new(
        br#"{"type":"user","uuid":"u","parentUuid":"missing","message":{"content":"one"}}
{"type":"assistant","uuid":"u","parentUuid":"u","message":{"content":"two"}}
"#,
    )
    .read();
    assert!(has(&r, "missing_parent"));
    assert!(has(&r, "duplicate_uuid"));
    assert!(has(&r, "nonpreceding_parent"));
    assert!(r.requires_branch_selection);
}
#[test]
fn timestamps_do_not_reorder_and_mixed_sessions_require_selection() {
    let r = Input::new(br#"{"type":"user","uuid":"u","sessionId":"a","timestamp":"2026-09-24T13:00:00Z","message":{"content":"first"}}
{"type":"assistant","uuid":"a","parentUuid":"u","sessionId":"b","timestamp":"2026-09-24T12:00:00Z","message":{"content":"second"}}
"#).read();
    assert_eq!(r.events[0].text, "first");
    assert_eq!(r.events[1].text, "second");
    assert!(r.requires_branch_selection);
}
#[test]
fn cli_exposes_json_and_exit_codes() {
    let bin = env!("CARGO_BIN_EXE_memory-pier");
    for (name, code, state) in [
        ("basic.jsonl", 0, "read"),
        ("empty.jsonl", 0, "empty"),
        ("truncated.jsonl", 2, "partial"),
    ] {
        let out = Command::new(bin)
            .arg("inspect")
            .arg(fixture(name))
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(code));
        let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(json["state"], state);
        assert!(out.stderr.is_empty());
    }
    let out = Command::new(bin).output().unwrap();
    assert_eq!(out.status.code(), Some(64));
    let out = Command::new(bin)
        .arg("inspect")
        .arg(fixture("not-present.jsonl"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let out = Command::new(bin)
        .arg("inspect")
        .arg(fixture("."))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn malformed_provenance_and_empty_blocks_are_not_clean_reads() {
    let r = Input::new(br#"{"type":"user","uuid":123,"parentUuid":false,"isSidechain":"true","message":{"content":[]}}"#).read();
    assert_eq!(r.state, ReadState::Partial);
    assert!(r.requires_branch_selection);
    assert!(has(&r, "invalid_metadata"));
    assert!(has(&r, "invalid_content"));
}
