use memory_bee::{
    claude::{Limits, ReadState},
    codex::{Report, inspect},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/codex")
        .join(name)
}
fn has(r: &Report, code: &str) -> bool {
    r.diagnostics.iter().any(|d| d.code == code)
}
fn row(kind: &str, payload: Value) -> Value {
    json!({"timestamp":"2026-09-24T12:00:00Z","type":kind,"payload":payload})
}
fn meta(id: &str) -> Value {
    row("session_meta", json!({"id":id}))
}
fn msg(text: &str) -> Value {
    row(
        "response_item",
        json!({"type":"message","role":"user","content":[{"type":"input_text","text":text}]}),
    )
}
struct Input(PathBuf);
impl Input {
    fn bytes(bytes: &[u8]) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "memory-bee-codex-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("session.jsonl");
        fs::write(&path, bytes).unwrap();
        Self(path)
    }
    fn rows(rows: &[Value]) -> Self {
        Self::bytes(
            rows.iter()
                .map(|r| format!("{r}\n"))
                .collect::<String>()
                .as_bytes(),
        )
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
fn basic_order_tools_and_read_only_origin() {
    let path = fixture("basic.jsonl");
    let before = fs::read(&path).unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let r = inspect(&path, Limits::default()).unwrap();
    assert_eq!(r.state, ReadState::Read);
    assert_eq!(r.agent, "codex");
    assert_eq!(r.compatibility, "unverified");
    assert_eq!(r.events.len(), 5);
    assert_eq!(r.records.len(), 7);
    assert_eq!(r.events[0].source.line, 3);
    assert_eq!(r.events[0].source.block, Some(1));
    assert_eq!(
        r.events[0].source.session_id.as_deref(),
        Some("synthetic-session")
    );
    assert_eq!(
        r.events[0].source.turn_id.as_deref(),
        Some("synthetic-turn")
    );
    assert_eq!(r.events[2].tool_id.as_deref(), Some("call-1"));
    assert_eq!(r.events[2].tool_namespace.as_deref(), Some("functions"));
    assert_eq!(r.events[3].kind, "tool_result");
    assert_eq!(r.events[3].text, "synthetic\n");
    assert_eq!(r.events[4].sequence, 5);
    assert_eq!(r.events[4].provenance, "extracted");
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), modified);
}
#[test]
fn losses_do_not_duplicate_messages_or_replay_checkpoint_history() {
    let r = inspect(&fixture("losses.jsonl"), Limits::default()).unwrap();
    assert_eq!(r.state, ReadState::Partial);
    assert_eq!(r.events.len(), 3);
    assert_eq!(r.events[1].kind, "checkpoint");
    assert_eq!(r.events[1].provenance, "checkpoint");
    assert!(
        has(&r, "omitted_event_msg")
            && has(&r, "omitted_reasoning")
            && has(&r, "compaction")
            && has(&r, "omitted_fields")
    );
    let serialized = serde_json::to_string(&r).unwrap();
    for omitted in ["NOT-REPLAYED", "synthetic-opaque", "synthetic-image"] {
        assert!(!serialized.contains(omitted));
    }
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == "omitted_block" && d.line == Some(6) && d.block == Some(2))
    );
}
#[test]
fn empty_and_truncated() {
    assert_eq!(
        inspect(&fixture("empty.jsonl"), Limits::default())
            .unwrap()
            .state,
        ReadState::Empty
    );
    let r = inspect(&fixture("truncated.jsonl"), Limits::default()).unwrap();
    assert_eq!(r.events.len(), 5);
    assert!(has(&r, "incomplete_final_line"));
    assert_eq!(r.lines, 8);
}
#[test]
fn malformed_lines_recover_and_utf8_is_not_lossily_decoded() {
    let mut bytes = format!("{}\n", meta("s")).into_bytes();
    bytes.extend_from_slice(b"invalid\n\xff\nnull\n");
    bytes.extend_from_slice(format!("{}", msg("ok")).as_bytes());
    let r = Input::bytes(&bytes).read();
    assert_eq!(r.events.len(), 1);
    assert_eq!(r.events[0].source.line, 5);
    for code in ["invalid_json", "invalid_utf8", "invalid_record"] {
        assert!(has(&r, code));
    }
}
#[test]
fn limits_drain_lines_and_reject_files() {
    let input = Input::rows(&[meta("s"), msg(&"a".repeat(1000)), msg("ok")]);
    let r = inspect(
        &input.0,
        Limits {
            file_bytes: 5000,
            line_bytes: 300,
        },
    )
    .unwrap();
    assert!(has(&r, "line_limit"));
    assert_eq!(r.events.len(), 1);
    assert_eq!(r.events[0].source.line, 3);
    assert!(
        inspect(
            &input.0,
            Limits {
                file_bytes: 10,
                line_bytes: 300
            }
        )
        .is_err()
    );
    assert!(
        inspect(
            &input.0,
            Limits {
                file_bytes: 0,
                line_bytes: 300
            }
        )
        .is_err()
    );
    assert!(
        inspect(
            &input.0,
            Limits {
                file_bytes: 5000,
                line_bytes: 0
            }
        )
        .is_err()
    );
}
#[test]
fn identities_never_retroactively_assigned_or_inherited_across_bad_boundaries() {
    let r = Input::rows(&[
        msg("before"),
        meta("a"),
        row("turn_context", json!({"turn_id":"t"})),
        msg("a"),
        row("turn_context", Value::Null),
        msg("without turn"),
        row("session_meta", Value::Null),
        msg("without session"),
        meta("b"),
        msg("b"),
    ])
    .read();
    assert!(r.events[0].source.session_id.is_none());
    assert_eq!(r.events[1].source.turn_id.as_deref(), Some("t"));
    assert!(r.events[2].source.turn_id.is_none());
    assert!(r.events[3].source.session_id.is_none());
    assert_eq!(r.events[4].source.session_id.as_deref(), Some("b"));
    assert!(has(&r, "repeated_session_metadata"));
}
#[test]
fn unknown_roles_items_envelopes_and_fields_are_explicit_losses() {
    let r=Input::rows(&[meta("s"),row("future",json!({})),row("response_item",json!({"type":"future"})),
        row("response_item",json!({"type":"message","role":"future","content":[]})),
        row("response_item",json!({"type":"message","role":"user","extra":"OMITTED","content":[{"type":"input_text","text":"ok","new_field":"OMITTED"}]}))]).read();
    for code in [
        "unknown_record",
        "omitted_item",
        "unknown_role",
        "omitted_fields",
    ] {
        assert!(has(&r, code));
    }
    assert_eq!(r.events.len(), 1);
    assert!(!serde_json::to_string(&r).unwrap().contains("OMITTED"));
}
#[test]
fn custom_tools_and_mixed_results_keep_links_and_blocks() {
    let r=Input::rows(&[meta("s"),row("response_item",json!({"type":"custom_tool_call","id":"i","call_id":"c","name":"apply_patch","input":"*** synthetic ***"})),
        row("response_item",json!({"type":"custom_tool_call_output","call_id":"c","output":[{"type":"input_text","text":"one"},{"type":"input_image"},{"type":"input_text","text":"two"}]}))]).read();
    assert_eq!(r.events.len(), 3);
    assert_eq!(r.events[0].text, "*** synthetic ***");
    assert_eq!(r.events[0].source.id.as_deref(), Some("i"));
    assert_eq!(r.events[2].source.block, Some(3));
    assert!(r.events.iter().all(|e| e.tool_id.as_deref() == Some("c")));
    assert!(has(&r, "omitted_block"));
}
#[test]
fn metadata_timestamp_and_missing_links_are_diagnosed() {
    let mut message = msg("kept");
    message["timestamp"] = json!("invalid");
    message["payload"]["id"] = json!(2);
    let r = Input::rows(&[
        meta("s"),
        message,
        row(
            "response_item",
            json!({"type":"function_call_output","output":"unlinked"}),
        ),
        row(
            "response_item",
            json!({"type":"function_call","arguments":"{}"}),
        ),
    ])
    .read();
    for code in [
        "invalid_metadata",
        "invalid_timestamp",
        "missing_call_id",
        "invalid_tool_call",
    ] {
        assert!(has(&r, code));
    }
    assert!(r.events[0].timestamp.is_none());
    assert_eq!(r.events[1].text, "unlinked");
}
#[test]
fn all_roles_phase_and_identical_messages_preserved() {
    let mut rows = vec![meta("s")];
    for role in ["user", "assistant", "system", "developer", "user"] {
        rows.push(row("response_item",json!({"type":"message","role":role,"phase":"final_answer","content":[{"type":"output_text","text":"same"}]})));
    }
    let r = Input::rows(&rows).read();
    assert_eq!(r.events.len(), 5);
    assert_eq!(r.events[3].role, "developer");
    assert_eq!(r.events[1].phase.as_deref(), Some("final_answer"));
    assert_eq!(r.state, ReadState::Read);
}
#[test]
fn auxiliary_only_is_partial_and_not_fabricated_chat() {
    let r = Input::rows(&[
        meta("s"),
        row(
            "event_msg",
            json!({"type":"agent_message","message":"not imported"}),
        ),
    ])
    .read();
    assert_eq!(r.state, ReadState::Partial);
    assert!(r.events.is_empty());
    assert_eq!(r.records.len(), 2);
}
#[test]
fn encrypted_compaction_markers_and_invalid_content_remain_visible() {
    let mut rows = vec![meta("s"), row("compacted", json!({"message":3}))];
    for kind in ["compaction", "context_compaction", "compaction_trigger"] {
        rows.push(row(
            "response_item",
            json!({"type":kind,"encrypted_content":"OPAQUE"}),
        ));
    }
    rows.push(row(
        "response_item",
        json!({"type":"message","role":"user","content":[{"type":"input_text","text":3}]}),
    ));
    rows.push(row(
        "response_item",
        json!({"type":"message","role":"user","content":[]}),
    ));
    let r = Input::rows(&rows).read();
    assert_eq!(
        r.diagnostics
            .iter()
            .filter(|d| d.code == "compaction")
            .count(),
        4
    );
    for code in ["invalid_checkpoint", "invalid_block", "invalid_content"] {
        assert!(has(&r, code));
    }
    assert!(!serde_json::to_string(&r).unwrap().contains("OPAQUE"));
}
#[test]
fn cli_exit_codes_and_no_implicit_claude_flags() {
    for (name, code) in [
        ("basic.jsonl", 0),
        ("losses.jsonl", 2),
        ("truncated.jsonl", 2),
        ("empty.jsonl", 0),
        ("absent.jsonl", 1),
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_memory-bee"))
            .arg("inspect-codex")
            .arg(fixture(name))
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(code));
        if code != 1 {
            let _: Value = serde_json::from_slice(&out.stdout).unwrap();
        }
    }
    let out = Command::new(env!("CARGO_BIN_EXE_memory-bee"))
        .args(["inspect-codex", "x", "--leaf", "x"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(64));
}

#[test]
fn unreadable_records_break_identity_until_fresh_metadata() {
    let bytes = format!(
        "{}\n{}\n{}\nBROKEN\n{}\n{}\n{}\n",
        meta("before"),
        row("turn_context", json!({"turn_id":"old"})),
        msg("first"),
        msg("unknown"),
        meta("after"),
        msg("last")
    );
    let r = Input::bytes(bytes.as_bytes()).read();
    assert_eq!(r.events[0].source.session_id.as_deref(), Some("before"));
    assert!(r.events[1].source.session_id.is_none());
    assert!(r.events[1].source.turn_id.is_none());
    assert_eq!(r.events[2].source.session_id.as_deref(), Some("after"));
}
