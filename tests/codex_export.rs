use memory_pier::{
    bundle::{Options, Prepared, prepare_codex},
    claude::Limits,
    codex::inspect,
    receive::verify,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "memory-pier-codex-export-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/codex")
        .join(name)
}
fn prepare(name: &str, options: &Options) -> Result<Prepared, String> {
    prepare_codex(inspect(&fixture(name), Limits::default()).unwrap(), options)
}
fn history(bundle: &Prepared) -> Vec<Value> {
    bundle
        .history()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_memory-pier"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn native_events_preview_write_verify_and_origin_preserved() {
    let path = fixture("basic.jsonl");
    let before = fs::read(&path).unwrap();
    let mtime = fs::metadata(&path).unwrap().modified().unwrap();
    let report = inspect(&path, Limits::default()).unwrap();
    let expected: Vec<_> = report
        .events
        .iter()
        .map(|e| serde_json::to_value(e).unwrap())
        .collect();
    let bundle = prepare_codex(report, &Options::default()).unwrap();
    assert_eq!(history(&bundle), expected);
    let manifest = serde_json::to_value(bundle.manifest()).unwrap();
    assert_eq!(
        manifest["source"],
        json!({"agent":"codex","version":"synthetic","session_id":"synthetic-session"})
    );
    assert_eq!(manifest["format_version"], 1);
    assert_eq!(manifest["code_state"], "unknown");
    assert!(bundle.handoff().contains("Registro físico"));
    assert!(!bundle.handoff().contains("UUID"));
    assert!(!bundle.handoff().contains("Claude"));
    assert!(bundle.handoff().contains("unverified"));
    assert!(!bundle.history().contains("/synthetic/project"));
    let root = Temp::new();
    let out = root.0.join("bundle");
    bundle.write(&out).unwrap();
    verify(&out).unwrap();
    assert_eq!(
        fs::read_to_string(out.join("history.jsonl")).unwrap(),
        bundle.history()
    );
    assert_eq!(
        fs::read_to_string(out.join("HANDOFF.md")).unwrap(),
        bundle.handoff()
    );
    assert!(bundle.write(&out).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), mtime);
}

#[test]
fn exclusions_renumber_without_changing_native_provenance_or_losses() {
    let mut report = inspect(&fixture("basic.jsonl"), Limits::default()).unwrap();
    report.events[0].phase = Some("analysis".into());
    report.events[0].role = "developer";
    // Multiple blocks of one physical line must disappear together.
    report.events[1].source.line = 3;
    let expected: Vec<_> = report
        .events
        .iter()
        .skip(2)
        .enumerate()
        .map(|(i, e)| {
            let mut value = serde_json::to_value(e).unwrap();
            value["sequence"] = json!(i + 1);
            value
        })
        .collect();
    let bundle = prepare_codex(
        report,
        &Options {
            exclude_lines: [3].into(),
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(history(&bundle), expected);
    assert!(bundle.handoff().contains("Nenhum pedido humano"));
    assert!(bundle.handoff().contains("Linha 3 excluída"));
    let partial = prepare(
        "losses.jsonl",
        &Options {
            exclude_lines: [2].into(),
            ..Options::default()
        },
    )
    .unwrap();
    assert!(partial.is_partial());
    assert!(partial.handoff().contains("event_msg"));
    assert!(!partial.history().contains("NOT-REPLAYED"));
}

#[test]
fn checkpoint_and_developer_are_not_fabricated_human_requests() {
    let mut report = inspect(&fixture("basic.jsonl"), Limits::default()).unwrap();
    report.events.truncate(1);
    report.events[0].role = "developer";
    report.events[0].phase = Some("analysis".into());
    report.events[0].tool_namespace = Some("functions".into());
    let bundle = prepare_codex(report, &Options::default()).unwrap();
    assert!(bundle.handoff().contains("Nenhum pedido humano"));
    assert_eq!(history(&bundle)[0]["role"], "developer");
    assert_eq!(history(&bundle)[0]["phase"], "analysis");
    assert_eq!(history(&bundle)[0]["tool_namespace"], "functions");
    let partial = prepare("losses.jsonl", &Options::default()).unwrap();
    assert_eq!(history(&partial)[1]["provenance"], "checkpoint");
    assert!(!partial.history().contains("synthetic-opaque"));
}

#[test]
fn empty_all_excluded_unknown_line_leaf_and_multiple_sessions_are_refused() {
    assert!(prepare("empty.jsonl", &Options::default()).is_err());
    for options in [
        Options {
            exclude_lines: [3, 4, 5, 6, 7].into(),
            ..Options::default()
        },
        Options {
            exclude_lines: [1].into(),
            ..Options::default()
        },
        Options {
            leaf: Some("synthetic-turn".into()),
            ..Options::default()
        },
        Options {
            include_paths: ["file".into()].into(),
            ..Options::default()
        },
    ] {
        assert!(prepare("basic.jsonl", &options).is_err());
    }
    let mut report = inspect(&fixture("basic.jsonl"), Limits::default()).unwrap();
    report.events[1].source.session_id = Some("second-session".into());
    assert!(
        prepare_codex(report, &Options::default())
            .unwrap_err()
            .contains("multiple sessions")
    );
}

#[test]
fn unknown_identity_and_multiple_versions_are_not_guessed() {
    let mut report = inspect(&fixture("basic.jsonl"), Limits::default()).unwrap();
    for e in &mut report.events {
        e.source.session_id = None;
        e.source.turn_id = None;
    }
    report.observed_versions.insert("other-version".into());
    let bundle = prepare_codex(report, &Options::default()).unwrap();
    let manifest = serde_json::to_value(bundle.manifest()).unwrap();
    assert!(manifest["source"]["session_id"].is_null());
    assert!(manifest["source"]["version"].is_null());
    assert!(history(&bundle)[0]["source"]["turn_id"].is_null());
    assert!(bundle.handoff().contains("Múltiplas versões"));
}

#[test]
fn secrets_in_codex_metadata_and_text_block_writing_after_selection() {
    let secret = "sk-synthetic0123456789abcdef";
    let root = Temp::new();
    for field in [
        "text",
        "phase",
        "tool_namespace",
        "tool_id",
        "tool_name",
        "source.id",
        "source.session_id",
        "source.turn_id",
    ] {
        let mut report = inspect(&fixture("basic.jsonl"), Limits::default()).unwrap();
        report.events.truncate(1);
        let event = &mut report.events[0];
        match field {
            "text" => event.text = secret.into(),
            "phase" => event.phase = Some(secret.into()),
            "tool_namespace" => event.tool_namespace = Some(secret.into()),
            "tool_id" => event.tool_id = Some(secret.into()),
            "tool_name" => event.tool_name = Some(secret.into()),
            "source.id" => event.source.id = Some(secret.into()),
            "source.session_id" => event.source.session_id = Some(secret.into()),
            "source.turn_id" => event.source.turn_id = Some(secret.into()),
            _ => unreachable!(),
        }
        let bundle = prepare_codex(report, &Options::default()).unwrap();
        assert!(
            bundle
                .findings()
                .iter()
                .any(|f| f.field == field && f.line == Some(3)),
            "{field}"
        );
        let findings = serde_json::to_string(bundle.findings()).unwrap();
        assert!(!findings.contains(secret));
        let out = root.0.join("blocked");
        assert!(bundle.write(&out).is_err());
        assert!(!out.exists());
    }
    let mut report = inspect(&fixture("basic.jsonl"), Limits::default()).unwrap();
    report.events[0].text = secret.into();
    let bundle = prepare_codex(
        report,
        &Options {
            exclude_lines: [3].into(),
            ..Options::default()
        },
    )
    .unwrap();
    assert!(bundle.findings().is_empty());
    assert!(!bundle.history().contains(secret));
    bundle.write(&root.0.join("filtered")).unwrap();
}

#[test]
fn inherited_sensitive_turn_survives_message_exclusion_and_version_is_scanned() {
    let secret = "sk-synthetic0123456789abcdef";
    let mut report = inspect(&fixture("basic.jsonl"), Limits::default()).unwrap();
    for e in &mut report.events {
        e.source.turn_id = Some(secret.into());
    }
    let bundle = prepare_codex(
        report,
        &Options {
            exclude_lines: [3].into(),
            ..Options::default()
        },
    )
    .unwrap();
    assert!(
        bundle
            .findings()
            .iter()
            .any(|f| f.field == "source.turn_id")
    );
    let mut report = inspect(&fixture("basic.jsonl"), Limits::default()).unwrap();
    report.observed_versions = [secret.into()].into();
    let bundle = prepare_codex(report, &Options::default()).unwrap();
    assert!(
        bundle
            .findings()
            .iter()
            .any(|f| f.field == "source.version")
    );
}

#[test]
fn cli_preview_partial_write_usage_and_secret_exit_codes() {
    let root = Temp::new();
    let basic = fixture("basic.jsonl");
    let basic = basic.to_str().unwrap();
    let preview = cli(&["export-codex", basic, "--preview"]);
    assert_eq!(preview.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(value["manifest"]["source"]["agent"], "codex");
    for tail in [
        vec![],
        vec!["--preview", "--leaf", "x"],
        vec!["--preview", "--output", "unused"],
        vec!["--preview", "--exclude-line", "0"],
        vec!["--preview", "--include-path", "file"],
    ] {
        let mut args = vec!["export-codex", basic];
        args.extend(tail);
        assert_eq!(cli(&args).status.code(), Some(64));
    }
    let out = root.0.join("partial");
    let result = cli(&[
        "export-codex",
        fixture("truncated.jsonl").to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert_eq!(result.status.code(), Some(2));
    verify(&out).unwrap();
    assert_eq!(
        cli(&[
            "export-codex",
            fixture("empty.jsonl").to_str().unwrap(),
            "--preview"
        ])
        .status
        .code(),
        Some(1)
    );
    let input = root.0.join("sensitive.jsonl");
    let secret = "sk-synthetic0123456789abcdef";
    let row = json!({"type":"response_item", "payload":{"type":"message","role":"user","content":[{"type":"input_text","text":secret}]}});
    fs::write(&input, format!("{row}\n")).unwrap();
    let out = root.0.join("secret-output");
    let result = cli(&[
        "export-codex",
        input.to_str().unwrap(),
        "--output",
        out.to_str().unwrap(),
    ]);
    assert_eq!(result.status.code(), Some(3));
    assert!(!String::from_utf8(result.stdout).unwrap().contains(secret));
    assert!(!out.exists());
}
