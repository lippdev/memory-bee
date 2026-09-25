use memory_bee::{
    bundle, receive,
    workspace::{
        Agent, Event,
        adapter::{Adapter, Demo},
        codex::Protocol,
        store::{State, Store},
    },
};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "memory-bee-workspace-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn until_approval(demo: &mut Demo) -> String {
    for _ in 0..10 {
        if let Some(Event::Approval { id, .. }) = demo.poll() {
            return id;
        }
    }
    panic!("missing approval")
}
#[test]
fn both_demo_adapters_require_decision_and_never_execute() {
    for agent in [Agent::Claude, Agent::Codex] {
        let mut demo = Demo::new(agent);
        demo.send("task").unwrap();
        assert!(demo.send("overlap").is_err());
        let id = until_approval(&mut demo);
        assert!(demo.poll().is_none());
        assert!(demo.decide("wrong", true).is_err());
        demo.decide(&id, false).unwrap();
        assert!(matches!(
            demo.poll(),
            Some(Event::Decision { allow: false, .. })
        ));
        demo.poll();
        assert_eq!(demo.poll(), Some(Event::Completed));
        demo.send("next").unwrap();
    }
}
#[test]
fn interruption_invalidates_permission_and_error_finishes_turn() {
    let mut demo = Demo::new(Agent::Claude);
    demo.send("task").unwrap();
    let id = until_approval(&mut demo);
    demo.interrupt();
    assert_eq!(demo.poll(), Some(Event::Interrupted));
    assert!(demo.decide(&id, true).is_err());
    assert!(demo.poll().is_none());
    demo.send("[erro]").unwrap();
    demo.poll();
    demo.poll();
    assert!(matches!(demo.poll(), Some(Event::Error { .. })));
    demo.send("retry manually").unwrap();
}
#[test]
fn store_reopens_history_and_marks_interrupted_without_replay() {
    let temp = Temp::new();
    let path = temp.0.join("state");
    let (store, mut state) = Store::open(&path, "/synthetic/project", Agent::Claude).unwrap();
    state.current_mut().record(Event::User {
        text: "pending".into(),
    });
    store.save(&state).unwrap();
    assert!(Store::open(&path, "/synthetic/project", Agent::Codex).is_err());
    drop(store);
    let (_store, state) = Store::open(&path, "/synthetic/project", Agent::Codex).unwrap();
    assert_eq!(state.current().agent, Agent::Claude);
    assert!(!state.current().running);
    assert!(state.current().events.contains(&Event::Interrupted));
    assert!(matches!(state.current().events[0], Event::User { .. }));
}
#[test]
fn store_refuses_foreign_project_and_preserves_malformed_data() {
    let temp = Temp::new();
    let path = temp.0.join("state");
    let (store, _) = Store::open(&path, "/one", Agent::Claude).unwrap();
    drop(store);
    assert!(Store::open(&path, "/two", Agent::Codex).is_err());
    assert!(!path.join("workspace.lock").exists());
    fs::write(path.join("workspace.json"), "bad data").unwrap();
    assert!(Store::open(&path, "/one", Agent::Codex).is_err());
    assert_eq!(
        fs::read_to_string(path.join("workspace.json")).unwrap(),
        "bad data"
    );
}
#[cfg(unix)]
#[test]
fn store_uses_private_permissions_and_refuses_symlinks() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = Temp::new();
    let path = temp.0.join("state");
    let (store, _) = Store::open(&path, "/one", Agent::Claude).unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(path.join("workspace.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    drop(store);
    symlink(&path, temp.0.join("alias")).unwrap();
    assert!(Store::open(&temp.0.join("alias"), "/one", Agent::Codex).is_err());
    fs::remove_file(path.join("workspace.json")).unwrap();
    symlink(temp.0.join("target"), path.join("workspace.json")).unwrap();
    fs::write(temp.0.join("target"), "untouched").unwrap();
    assert!(Store::open(&path, "/one", Agent::Codex).is_err());
    assert_eq!(
        fs::read_to_string(temp.0.join("target")).unwrap(),
        "untouched"
    );
}
#[test]
fn state_fork_keeps_source_identity_and_requires_idle_turn() {
    let mut state = State::new("/project".into(), Agent::Claude);
    state.current_mut().record(Event::User {
        text: "original task".into(),
    });
    assert!(
        state
            .fork(Agent::Codex, state.profiles[1].clone(), true)
            .is_err()
    );
    state.current_mut().record(Event::Completed);
    let original = state.current().clone();
    state
        .fork(Agent::Codex, state.profiles[1].clone(), true)
        .unwrap();
    assert_eq!(state.sessions[0], original);
    assert_eq!(state.current().agent, Agent::Codex);
    assert_eq!(state.current().profile, "Trabalho (demo)");
    assert!(
        state
            .current()
            .events
            .iter()
            .any(|e| matches!(e,Event::Notice{message} if message.contains("original task")))
    );
}
#[test]
fn simulation_bundle_has_honest_provenance_and_valid_hashes() {
    let tmp = Temp::new();
    let mut state = State::new("/project".into(), Agent::Claude);
    state.current_mut().record(Event::User {
        text: "Task".into(),
    });
    state.current_mut().record(Event::Text {
        text: "simulated answer".into(),
    });
    let prepared =
        bundle::prepare_workspace_demo(state.current(), &bundle::Options::default()).unwrap();
    assert!(prepared.is_partial());
    assert!(prepared.handoff().contains("SIMULAÇÃO"));
    let manifest = serde_json::to_value(prepared.manifest()).unwrap();
    assert_eq!(manifest["source"]["agent"], "memory-bee-demo");
    assert!(prepared.history().contains("user_input"));
    assert!(prepared.history().contains("simulated"));
    prepared.write(&tmp.0.join("bundle")).unwrap();
    let v = receive::verify(&tmp.0.join("bundle")).unwrap();
    assert!(v.known_source().is_none());
    assert!(!v.has_changes());
}
#[test]
fn simulation_export_blocks_secrets_and_allows_exclusion() {
    let tmp = Temp::new();
    let mut state = State::new("/project".into(), Agent::Codex);
    let token = format!("ghp_{}", "syntheticfixture".repeat(3));
    state.current_mut().record(Event::User { text: token });
    state.current_mut().record(Event::Text {
        text: "safe".into(),
    });
    let p = bundle::prepare_workspace_demo(state.current(), &bundle::Options::default()).unwrap();
    assert!(p.write(&tmp.0.join("blocked")).is_err());
    assert!(!tmp.0.join("blocked").exists());
    let p = bundle::prepare_workspace_demo(
        state.current(),
        &bundle::Options {
            exclude_lines: [1].into(),
            ..Default::default()
        },
    )
    .unwrap();
    p.write(&tmp.0.join("safe")).unwrap();
    assert!(receive::verify(&tmp.0.join("safe")).is_ok());
}
fn ready() -> Protocol {
    let mut p = Protocol::default();
    let init = p.initialize();
    let out = p.receive(json!({"id":init["id"],"result":{"userAgent":"synthetic"}}));
    assert_eq!(out.replies[0]["method"], "initialized");
    let start = p.start(std::path::Path::new("/project"), None).unwrap();
    p.receive(json!({"id":start["id"],"result":{"thread":{"id":"t"}}}));
    let send = p.send("hello").unwrap();
    p.receive(json!({"id":send["id"],"result":{"turn":{"id":"turn"}}}));
    p
}
#[test]
fn protocol_requires_handshake_correlates_requests_and_refuses_overlap() {
    let mut p = Protocol::default();
    assert!(p.send("before init").is_err());
    assert!(p.start(std::path::Path::new("/project"), None).is_err());
    let mut p = ready();
    assert!(p.send("overlap").is_err());
    assert_eq!(p.interrupt().unwrap()["params"]["turnId"], "turn");
    p.receive(json!({"method":"turn/completed","params":{"threadId":"t","turn":{"id":"turn","status":"interrupted"}}}));
    assert!(p.send("manual retry").is_ok());
}
#[test]
fn protocol_scopes_and_invalidates_approvals_and_never_grants_unknown_requests() {
    let mut p = ready();
    let out=p.receive(json!({"id":9,"method":"item/commandExecution/requestApproval","params":{"threadId":"other","turnId":"turn"}}));
    assert!(out.replies[0].get("error").is_some());
    assert!(p.decide("9", true).is_err());
    let out=p.receive(json!({"id":10,"method":"item/commandExecution/requestApproval","params":{"threadId":"t","turnId":"turn","command":"echo synthetic"}}));
    assert!(matches!(out.events[0], Event::Approval { .. }));
    assert_eq!(
        p.decide("10", false).unwrap()["result"]["decision"],
        "decline"
    );
    assert!(p.decide("10", true).is_err());
    let out = p.receive(json!({"id":"unknown","method":"item/tool/requestUserInput","params":{}}));
    assert!(out.replies[0].get("error").is_some());
    p.receive(json!({"id":11,"method":"item/fileChange/requestApproval","params":{"threadId":"t","turnId":"turn"}}));
    p.receive(json!({"method":"turn/completed","params":{"threadId":"t","turn":{"id":"turn","status":"completed"}}}));
    assert!(p.decide("11", true).is_err());
}
#[test]
fn protocol_authentication_values_never_become_conversation_events() {
    let mut p = ready();
    let request = p.account().unwrap();
    let out=p.receive(json!({"id":request["id"],"result":{"account":{"email":"synthetic@example.invalid","token":"private-value"}}}));
    let events = serde_json::to_string(&out.events).unwrap();
    assert!(!events.contains("synthetic@example.invalid"));
    assert!(!events.contains("private-value"));
}
#[test]
fn cli_gate_precedes_state_creation_and_once_is_read_only() {
    let tmp = Temp::new();
    let binary = env!("CARGO_BIN_EXE_memory-bee");
    let out = std::process::Command::new(binary)
        .args(["workspace", "--project"])
        .arg(&tmp.0)
        .arg("--state")
        .arg(tmp.0.join("state"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(64));
    assert!(!tmp.0.join("state").exists());
    for agent in ["claude", "codex"] {
        for (w, h) in [(100, 30), (60, 20), (40, 12), (30, 10)] {
            let out = std::process::Command::new(binary)
                .args(["workspace", "--demo", "--project"])
                .arg(&tmp.0)
                .args([
                    "--once",
                    "--agent",
                    agent,
                    "--width",
                    &w.to_string(),
                    "--height",
                    &h.to_string(),
                    "--no-color",
                ])
                .output()
                .unwrap();
            assert!(out.status.success());
            let text = String::from_utf8(out.stdout).unwrap();
            assert!(text.contains("SIMULAÇÃO"));
            assert!(!text.contains('\x1b'));
            assert_eq!(text.lines().count(), h);
        }
    }
    assert_eq!(fs::read_dir(&tmp.0).unwrap().count(), 0);
}
#[test]
fn malformed_protocol_does_not_start_or_retry_work() {
    let mut p = Protocol::default();
    let r = p.initialize();
    let out = p.receive(json!({"id":r["id"],"error":{"message":"failed"}}));
    assert!(matches!(out.events[0], Event::Error { .. }));
    assert!(p.account().is_err());
    assert!(p.thread.is_none());
    assert!(p.turn.is_none());
}

#[test]
fn stale_turn_events_and_resolved_permissions_cannot_affect_current_work() {
    let mut p = ready();
    for event in [
        json!({"method":"turn/completed","params":{"threadId":"t","turn":{"id":"old","status":"completed"}}}),
        json!({"method":"item/agentMessage/delta","params":{"threadId":"t","turnId":"old","delta":"stale"}}),
        json!({"method":"item/agentMessage/delta","params":{"turnId":"turn","delta":"unscoped"}}),
    ] {
        assert!(p.receive(event).events.is_empty());
        assert_eq!(p.turn.as_deref(), Some("turn"));
    }
    p.receive(json!({"id":12,"method":"item/commandExecution/requestApproval","params":{"threadId":"t","turnId":"turn"}}));
    p.receive(json!({"method":"serverRequest/resolved","params":{"threadId":"t","requestId":12}}));
    assert!(p.decide("12", true).is_err());
    let out = p.receive(json!({"id":13,"method":"item/commandExecution/requestApproval","params":{"threadId":"t","turnId":"turn","availableDecisions":["cancel"]}}));
    assert!(out.replies[0].get("error").is_some());
    assert!(p.decide("13", true).is_err());
}

#[test]
fn interrupted_export_reports_partial_history_in_markdown_and_manifest() {
    let mut state = State::new("/project".into(), Agent::Claude);
    state.current_mut().record(Event::User {
        text: "task".into(),
    });
    state.current_mut().record(Event::Interrupted);
    let p = bundle::prepare_workspace_demo(state.current(), &bundle::Options::default()).unwrap();
    assert!(p.is_partial());
    assert!(p.handoff().contains("Snapshot parcial"));
    assert!(
        serde_json::to_string(p.manifest())
            .unwrap()
            .contains("Snapshot parcial")
    );
}

#[cfg(unix)]
#[test]
fn dangling_state_symlink_is_not_replaced() {
    let temp = Temp::new();
    let path = temp.0.join("state");
    let (store, _) = Store::open(&path, "/one", Agent::Claude).unwrap();
    drop(store);
    fs::remove_file(path.join("workspace.json")).unwrap();
    std::os::unix::fs::symlink(temp.0.join("absent"), path.join("workspace.json")).unwrap();
    assert!(Store::open(&path, "/one", Agent::Claude).is_err());
    assert!(
        fs::symlink_metadata(path.join("workspace.json"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[cfg(unix)]
#[test]
fn transport_uses_explicit_profile_and_bounds_invalid_frames() {
    use memory_bee::workspace::codex::Transport;
    use std::os::unix::fs::PermissionsExt;
    let temp = Temp::new();
    let home = temp.0.join("profile");
    fs::create_dir(&home).unwrap();
    fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).unwrap();
    let fake = temp.0.join("fake-provider");
    fs::write(&fake, "#!/bin/sh\n[ \"$1\" = app-server ] || exit 1\n[ \"$2\" = --listen ] || exit 2\n[ \"$3\" = stdio:// ] || exit 3\n[ \"$HOME\" = \"$CODEX_HOME\" ] || exit 4\n[ -z \"$OPENAI_API_KEY$CODEX_API_KEY$CODEX_ACCESS_TOKEN\" ] || exit 5\nprintf 'private\\n' > \"$CODEX_HOME/marker\"\nread -r request\nprintf '{\"id\":1,\"result\":{}}\\n'\nprintf 'not-json\\n'\n").unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();
    let mut t = Transport::spawn(&fake, &home, &temp.0).unwrap();
    t.send(&json!({"id":1,"method":"initialize","params":{}}))
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut received = false;
    loop {
        assert!(std::time::Instant::now() < deadline, "provider timeout");
        match t.poll() {
            Ok(Some(value)) => {
                assert_eq!(value["id"], 1);
                received = true;
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(5)),
            Err(error) => {
                assert!(error.contains("Malformed"));
                break;
            }
        }
    }
    assert!(received);
    assert_eq!(
        fs::read_to_string(home.join("marker")).unwrap(),
        "private\n"
    );
    assert!(t.send(&json!({"text":"x".repeat(1024 * 1024)})).is_err());
    drop(t);
    fs::set_permissions(&home, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Transport::spawn(&fake, &home, &temp.0).is_err());
}
