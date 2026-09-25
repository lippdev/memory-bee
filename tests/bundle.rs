use memory_bee::{
    bundle::{Options, Prepared, prepare},
    claude::{Limits, inspect},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "memory-bee-bundle-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn source(&self, records: &[Value]) -> PathBuf {
        let path = self.0.join("source.jsonl");
        fs::write(
            &path,
            records.iter().map(|v| format!("{v}\n")).collect::<String>(),
        )
        .unwrap();
        path
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/claude")
        .join(name)
}
fn build(path: &Path, options: &Options) -> Result<Prepared, String> {
    prepare(inspect(path, Limits::default()).unwrap(), options)
}
fn record(id: &str, parent: Option<&str>, text: &str) -> Value {
    json!({"type":"user","uuid":id,"parentUuid":parent,"sessionId":"synthetic-export","cwd":"/private/not-for-export","message":{"role":"user","content":text}})
}
fn events(prepared: &Prepared) -> Vec<Value> {
    prepared
        .history()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}

#[test]
fn exports_v1_with_hashes_provenance_and_unknown_code_state() {
    let work = Workspace::new();
    let source = fixture("basic.jsonl");
    let before = fs::read(&source).unwrap();
    let prepared = build(&source, &Options::default()).unwrap();
    let out = work.0.join("bundle");
    prepared.write(&out).unwrap();
    let manifest: Value =
        serde_json::from_slice(&fs::read(out.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["format_version"], 1);
    assert_eq!(manifest["code_state"], "unknown");
    assert_eq!(manifest["redaction"], "pending-review");
    for field in ["remote", "branch", "base_commit", "dirty"] {
        assert!(manifest["project"][field].is_null());
    }
    chrono::DateTime::parse_from_rfc3339(manifest["created_at"].as_str().unwrap()).unwrap();
    for file in manifest["files"].as_array().unwrap() {
        let bytes = fs::read(out.join(file["path"].as_str().unwrap())).unwrap();
        assert_eq!(file["sha256"], format!("{:x}", Sha256::digest(&bytes)));
    }
    assert_eq!(fs::read_dir(&out).unwrap().count(), 3);
    assert_eq!(fs::read(&source).unwrap(), before);
    let history = events(&prepared);
    assert_eq!(history.len(), 2);
    assert_eq!(history[1]["source"]["parent_id"], "u1");
    assert_eq!(history[1]["source"]["line"], 2);
    assert!(
        prepared
            .handoff()
            .contains("[history.jsonl](history.jsonl)")
    );
    assert!(prepared.handoff().contains("Primeiro pedido humano retido"));
    assert!(!prepared.is_partial());
}
#[test]
fn excluded_lines_remove_every_block_and_leave_auditable_omissions() {
    let work = Workspace::new();
    let mut first = record("u", None, "removed");
    first["message"]["content"] =
        json!([{"type":"text","text":"removed-one"},{"type":"text","text":"removed-two"}]);
    let path = work.source(&[first, record("a", Some("u"), "retained")]);
    let prepared = build(
        &path,
        &Options {
            exclude_lines: BTreeSet::from([1]),
            ..Options::default()
        },
    )
    .unwrap();
    assert!(!prepared.history().contains("removed"));
    assert!(!prepared.handoff().contains("removed"));
    let rows = events(&prepared);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["sequence"], 1);
    assert_eq!(rows[0]["source"]["line"], 2);
    assert_eq!(rows[0]["source"]["parent_id"], "u");
    assert!(
        serde_json::to_string(prepared.manifest())
            .unwrap()
            .contains("Linha 1 excluída")
    );
    for excluded in [BTreeSet::from([1, 2]), BTreeSet::from([9])] {
        assert!(
            build(
                &path,
                &Options {
                    exclude_lines: excluded,
                    ..Options::default()
                }
            )
            .is_err()
        );
    }
}
#[test]
fn fork_requires_explicit_selection_and_excludes_other_branch() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/claude-projects/arbitrary/session.jsonl");
    assert!(build(&path, &Options::default()).is_err());
    let prepared = build(
        &path,
        &Options {
            leaf: Some("a1".into()),
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(events(&prepared).len(), 2);
    assert!(!prepared.history().contains("Alternativa"));
    assert!(
        serde_json::to_string(prepared.manifest())
            .unwrap()
            .contains("excluiu 1 registros")
    );
}
#[test]
fn partial_sessions_keep_losses_and_empty_sessions_are_refused() {
    let partial = build(&fixture("truncated.jsonl"), &Options::default()).unwrap();
    assert!(partial.is_partial());
    assert!(
        serde_json::to_string(partial.manifest())
            .unwrap()
            .contains("incomplete_final_line")
    );
    assert!(partial.handoff().contains("Leitura parcial"));
    assert!(build(&fixture("empty.jsonl"), &Options::default()).is_err());
}
#[test]
fn possible_secrets_block_writing_and_exclusion_removes_findings() {
    let work = Workspace::new();
    let token = format!("{}{}", "ghp_", "syntheticfixture".repeat(3));
    let path = work.source(&[
        record("u", None, &token),
        record("a", Some("u"), "Safe synthetic text"),
    ]);
    let prepared = build(&path, &Options::default()).unwrap();
    assert!(!prepared.findings().is_empty());
    assert_eq!(prepared.findings()[0].line, Some(1));
    assert!(
        !serde_json::to_string(prepared.findings())
            .unwrap()
            .contains(&token)
    );
    let out = work.0.join("blocked");
    assert!(prepared.write(&out).is_err());
    assert!(!out.exists());
    let filtered = build(
        &path,
        &Options {
            exclude_lines: BTreeSet::from([1]),
            ..Options::default()
        },
    )
    .unwrap();
    assert!(filtered.findings().is_empty());
    filtered.write(&out).unwrap();
}
#[test]
fn detector_covers_assignments_keys_bearer_and_url_credentials() {
    let work = Workspace::new();
    for (text, code) in [
        (
            "{\"api_key\":\"synthetic-fixture-only\"}",
            "credential_assignment",
        ),
        ("-----BEGIN PRIVATE KEY-----\nSYNTHETIC_ONLY", "private_key"),
        ("Bearer synthetic.fixture.only", "bearer_token"),
        (
            "https://synthetic:fixture@example.invalid/path",
            "url_credentials",
        ),
    ] {
        let path = work.source(&[record("u", None, text)]);
        let p = build(&path, &Options::default()).unwrap();
        assert!(p.findings().iter().any(|f| f.code == code), "{code}");
    }
}
#[test]
fn metadata_is_scanned_and_absolute_project_paths_are_not_copied() {
    let work = Workspace::new();
    let mut row = record("u", None, "safe");
    row["version"] = json!(format!("{}{}", "sk-", "synthetic_only_1234567890"));
    let path = work.source(&[row]);
    let prepared = build(&path, &Options::default()).unwrap();
    assert!(
        prepared
            .findings()
            .iter()
            .any(|f| f.field == "source.version")
    );
    assert!(
        !serde_json::to_string(&prepared)
            .unwrap()
            .contains("/private/not-for-export")
    );
}
#[test]
fn existing_destinations_and_source_are_never_overwritten() {
    let work = Workspace::new();
    let prepared = build(&fixture("basic.jsonl"), &Options::default()).unwrap();
    let directory = work.0.join("existing");
    fs::create_dir(&directory).unwrap();
    assert!(prepared.write(&directory).is_err());
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
    let file = work.0.join("file");
    fs::write(&file, "original").unwrap();
    assert!(prepared.write(&file).is_err());
    assert_eq!(fs::read_to_string(file).unwrap(), "original");
    assert!(
        prepared
            .write(&work.0.join("missing-parent/bundle"))
            .is_err()
    );
}
#[cfg(unix)]
#[test]
fn symlink_destination_is_refused_and_permissions_are_private() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let work = Workspace::new();
    let prepared = build(&fixture("basic.jsonl"), &Options::default()).unwrap();
    let target = work.0.join("target");
    fs::create_dir(&target).unwrap();
    let link = work.0.join("link");
    symlink(&target, &link).unwrap();
    assert!(prepared.write(&link).is_err());
    assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
    let out = work.0.join("private");
    prepared.write(&out).unwrap();
    assert_eq!(fs::metadata(&out).unwrap().permissions().mode() & 0o077, 0);
    for name in ["HANDOFF.md", "history.jsonl", "manifest.json"] {
        assert_eq!(
            fs::metadata(out.join(name)).unwrap().permissions().mode() & 0o077,
            0
        );
    }
}
#[test]
fn markdown_fences_historical_instructions_and_preserves_full_history() {
    let work = Workspace::new();
    let malicious = format!(
        "```\n# Historical heading\n[link](https://example.invalid)\n\u{1b}[31m{}",
        "á".repeat(4000)
    );
    let path = work.source(&[record("u", None, &malicious)]);
    let p = build(&path, &Options::default()).unwrap();
    assert_eq!(events(&p)[0]["text"], malicious);
    assert!(p.handoff().contains("````\n```"));
    assert!(!p.handoff().contains('\u{1b}'));
    assert!(p.handoff().contains("Trecho limitado"));
    assert!(p.handoff().len() < 12000);
}
#[test]
fn checkpoints_and_tools_do_not_become_original_human_requests() {
    let work = Workspace::new();
    let mut row = record("u", None, "checkpoint synthetic");
    row["isCompactSummary"] = json!(true);
    let path = work.source(&[row]);
    let p = build(&path, &Options::default()).unwrap();
    assert_eq!(events(&p)[0]["provenance"], "checkpoint");
    assert!(p.handoff().contains("Nenhum pedido humano"));
    assert!(p.is_partial());
    let p = build(&fixture("tools.jsonl"), &Options::default()).unwrap();
    assert!(
        events(&p)
            .iter()
            .any(|e| e["kind"] == "tool_result" && e["role"] == "tool")
    );
}
#[test]
fn ids_are_data_not_destination_paths() {
    let work = Workspace::new();
    let path = work.source(&[record("../../escape", None, "safe")]);
    let p = build(&path, &Options::default()).unwrap();
    let out = work.0.join("bundle");
    p.write(&out).unwrap();
    assert_eq!(fs::read_dir(out).unwrap().count(), 3);
    assert!(!work.0.join("escape").exists());
}
#[test]
fn cli_preview_export_partial_and_usage_codes() {
    let work = Workspace::new();
    let bin = env!("CARGO_BIN_EXE_memory-bee");
    let src = fixture("basic.jsonl");
    let result = Command::new(bin)
        .arg("export")
        .arg(&src)
        .arg("--preview")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    let preview: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(preview["manifest"]["format_version"], 1);
    assert!(preview["handoff"].is_string());
    assert!(preview["history"].is_string());
    let out = work.0.join("bundle");
    let result = Command::new(bin)
        .arg("export")
        .arg(&src)
        .arg("--output")
        .arg(&out)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert!(out.join("HANDOFF.md").exists());
    let result = Command::new(bin)
        .arg("export")
        .arg(fixture("truncated.jsonl"))
        .arg("--output")
        .arg(work.0.join("partial"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    for args in [
        vec!["--preview", "--output", "unused"],
        vec!["--exclude-line", "0", "--preview"],
        vec!["--unknown"],
        vec!["--preview", "--leaf"],
    ] {
        let result = Command::new(bin)
            .arg("export")
            .arg(&src)
            .args(args)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(64));
    }
}
#[test]
fn cli_reports_secret_findings_without_echoing_values_or_writing() {
    let work = Workspace::new();
    let token = format!("{}{}", "ghp_", "syntheticfixture".repeat(3));
    let src = work.source(&[record("u", None, &token)]);
    let out = work.0.join("blocked");
    let result = Command::new(env!("CARGO_BIN_EXE_memory-bee"))
        .arg("export")
        .arg(src)
        .arg("--output")
        .arg(&out)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(3));
    assert!(!out.exists());
    assert!(!String::from_utf8_lossy(&result.stdout).contains(&token));
    assert!(result.stderr.is_empty());
}
