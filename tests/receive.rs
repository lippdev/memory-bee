use memory_pier::{
    bundle::{Options, prepare},
    claude::{Limits, inspect},
    receive,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "memory-pier-receive-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("source")).unwrap();
        let work = Self(root);
        work.git("source", &["init", "-b", "synthetic"]);
        work.git("source", &["config", "user.name", "Synthetic"]);
        work.git(
            "source",
            &["config", "user.email", "synthetic@example.invalid"],
        );
        work.git("source", &["config", "commit.gpgsign", "false"]);
        work.git("source", &["config", "core.hooksPath", "/dev/null"]);
        work
    }
    fn git(&self, dir: &str, args: &[&str]) -> String {
        let result = Command::new("git")
            .arg("-C")
            .arg(self.0.join(dir))
            .args(args)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{:?}: {}",
            args,
            String::from_utf8_lossy(&result.stderr)
        );
        String::from_utf8(result.stdout).unwrap().trim().into()
    }
    fn commit(&self) {
        self.git("source", &["add", "."]);
        self.git("source", &["commit", "--allow-empty", "-m", "synthetic"]);
    }
    fn export(&self, paths: &[&str]) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/claude/basic.jsonl");
        prepare(
            inspect(&source, Limits::default()).unwrap(),
            &Options {
                project: Some(self.0.join("source")),
                include_paths: paths.iter().map(|p| (*p).into()).collect(),
                ..Options::default()
            },
        )
        .unwrap()
        .write(&self.0.join("bundle"))
        .unwrap();
    }
    fn fixture() -> Self {
        let work = Self::new();
        fs::write(work.0.join("source/a.txt"), "base\n").unwrap();
        fs::write(work.0.join("source/z.txt"), "delete\n").unwrap();
        work.commit();
        work.git(
            "source",
            &[
                "clone",
                "--no-hardlinks",
                work.0.join("source").to_str().unwrap(),
                work.0.join("target").to_str().unwrap(),
            ],
        );
        fs::write(work.0.join("source/a.txt"), "result\r\nno final newline").unwrap();
        fs::remove_file(work.0.join("source/z.txt")).unwrap();
        fs::create_dir(work.0.join("source/nested")).unwrap();
        fs::write(work.0.join("source/nested/new.txt"), "new text\n").unwrap();
        work.export(&["a.txt", "nested/new.txt", "z.txt"]);
        work
    }
    fn manifest(&self) -> Value {
        serde_json::from_slice(&fs::read(self.0.join("bundle/manifest.json")).unwrap()).unwrap()
    }
    fn save(&self, value: &Value) {
        fs::write(
            self.0.join("bundle/manifest.json"),
            serde_json::to_vec_pretty(value).unwrap(),
        )
        .unwrap();
    }
    fn rehash(&self, filename: &str) {
        let mut manifest = self.manifest();
        for payload in manifest["files"].as_array_mut().unwrap() {
            if payload["path"] == filename {
                payload["sha256"] = format!(
                    "{:x}",
                    Sha256::digest(fs::read(self.0.join("bundle").join(filename)).unwrap())
                )
                .into();
            }
        }
        self.save(&manifest);
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn verify_check_and_explicit_write_roundtrip_preserve_source_and_index() {
    let work = Workspace::fixture();
    let target = work.0.join("target");
    let index = fs::read(target.join(".git/index")).unwrap();
    let verified = receive::verify(&work.0.join("bundle")).unwrap();
    assert_eq!(verified.report().changes, 3);
    let plan = verified.check(&target).unwrap();
    assert!(!plan.report().written);
    assert_eq!(fs::read_to_string(target.join("a.txt")).unwrap(), "base\n");
    assert!(!target.join("nested").exists());
    // Bundle bytes are owned; subsequent tampering cannot change a validated plan.
    fs::write(work.0.join("bundle/changes.patch"), "tampered later").unwrap();
    assert!(plan.write().unwrap().written);
    assert_eq!(
        fs::read(target.join("a.txt")).unwrap(),
        fs::read(work.0.join("source/a.txt")).unwrap()
    );
    assert_eq!(
        fs::read_to_string(target.join("nested/new.txt")).unwrap(),
        "new text\n"
    );
    assert!(!target.join("z.txt").exists());
    assert_eq!(fs::read(target.join(".git/index")).unwrap(), index);
    assert!(verified.check(&target).is_err());
    assert!(!fs::read_dir(&target).unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".memory-pier-apply-")
    }));
}

#[test]
fn hashes_missing_extra_and_duplicate_payloads_are_rejected() {
    let work = Workspace::fixture();
    let original = work.manifest();
    let mut manifest = original.clone();
    manifest["files"][0]["sha256"] = "0".repeat(64).into();
    work.save(&manifest);
    assert!(receive::verify(&work.0.join("bundle")).is_err());
    let mut manifest = original.clone();
    let duplicate = manifest["files"][0].clone();
    manifest["files"].as_array_mut().unwrap().push(duplicate);
    work.save(&manifest);
    assert!(receive::verify(&work.0.join("bundle")).is_err());
    work.save(&original);
    fs::write(work.0.join("bundle/unlisted"), "extra").unwrap();
    assert!(receive::verify(&work.0.join("bundle")).is_err());
    fs::remove_file(work.0.join("bundle/unlisted")).unwrap();
    fs::remove_file(work.0.join("bundle/HANDOFF.md")).unwrap();
    assert!(receive::verify(&work.0.join("bundle")).is_err());
}

#[test]
fn malformed_fields_versions_paths_and_change_mappings_fail() {
    let work = Workspace::fixture();
    let original = work.manifest();
    for (pointer, value) in [
        ("/format_version", Value::from(99)),
        ("/project/base_commit", Value::Null),
        ("/changes/0/path", Value::from("../escape")),
        ("/changes/0/payload", Value::from("files/0001.txt")),
        ("/changes/0/result_sha256", Value::from("0".repeat(64))),
        ("/changes/0/result_mode", Value::from("120000")),
        ("/created_at", Value::from("invalid")),
        ("/redaction", Value::from("trusted")),
        ("/files/0/path", Value::from("/outside")),
        ("/code_state", Value::from("unknown")),
        ("/selected_paths/1", Value::from("a.txt")),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        work.save(&changed);
        assert!(
            receive::verify(&work.0.join("bundle")).is_err(),
            "{pointer}"
        );
    }
    let mut changed = original.clone();
    changed["changes"][0]
        .as_object_mut()
        .unwrap()
        .remove("base_mode");
    work.save(&changed);
    assert!(receive::verify(&work.0.join("bundle")).is_err());
    let mut changed = original.clone();
    changed["source"].as_object_mut().unwrap().remove("version");
    work.save(&changed);
    assert!(receive::verify(&work.0.join("bundle")).is_err());
    let mut changed = original;
    changed["unexpected"] = true.into();
    work.save(&changed);
    assert!(receive::verify(&work.0.join("bundle")).is_err());
}

#[test]
fn altered_patch_with_recomputed_payload_hash_is_rejected() {
    let work = Workspace::fixture();
    let patch = fs::read_to_string(work.0.join("bundle/changes.patch")).unwrap();
    for altered in [
        patch.replace("a/a.txt", "a/../escape"),
        patch.replace("@@ -1,1", "@@ -1,99"),
        format!("{patch}diff --git \"a/unmapped\" \"b/unmapped\"\n"),
        patch.replace("+result", "+different"),
    ] {
        fs::write(work.0.join("bundle/changes.patch"), altered).unwrap();
        work.rehash("changes.patch");
        assert!(receive::verify(&work.0.join("bundle")).is_err());
    }
}

#[test]
fn dirty_wrong_base_and_changed_after_check_destinations_are_preserved() {
    let work = Workspace::fixture();
    let target = work.0.join("target");
    let verified = receive::verify(&work.0.join("bundle")).unwrap();
    let plan = verified.check(&target).unwrap();
    fs::write(target.join("a.txt"), "local edit\n").unwrap();
    assert!(plan.write().is_err());
    assert!(verified.check(&target).is_err());
    assert_eq!(
        fs::read_to_string(target.join("a.txt")).unwrap(),
        "local edit\n"
    );
    work.git(
        "target",
        &[
            "-c",
            "user.name=Synthetic",
            "-c",
            "user.email=synthetic@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "-am",
            "different base",
        ],
    );
    assert!(verified.check(&target).is_err());
}

#[test]
fn ignored_addition_collision_is_not_overwritten() {
    let work = Workspace::fixture();
    let target = work.0.join("target");
    fs::write(target.join(".git/info/exclude"), "nested/\n").unwrap();
    fs::create_dir(target.join("nested")).unwrap();
    fs::write(target.join("nested/new.txt"), "private existing\n").unwrap();
    assert!(
        receive::verify(&work.0.join("bundle"))
            .unwrap()
            .check(&target)
            .is_err()
    );
    assert_eq!(
        fs::read_to_string(target.join("nested/new.txt")).unwrap(),
        "private existing\n"
    );
}

#[cfg(unix)]
#[test]
fn symlink_payloads_roots_and_target_parents_are_rejected() {
    use std::os::unix::fs::symlink;
    let work = Workspace::fixture();
    let bundle = work.0.join("bundle");
    symlink(&bundle, work.0.join("link")).unwrap();
    assert!(receive::verify(&work.0.join("link")).is_err());
    let verified = receive::verify(&bundle).unwrap();
    fs::rename(bundle.join("HANDOFF.md"), work.0.join("outside")).unwrap();
    symlink(work.0.join("outside"), bundle.join("HANDOFF.md")).unwrap();
    assert!(receive::verify(&bundle).is_err());
    let target = work.0.join("target");
    fs::write(target.join(".git/info/exclude"), "nested\n").unwrap();
    symlink(work.0.join("source/nested"), target.join("nested")).unwrap();
    assert!(verified.check(&target).is_err());
}

#[cfg(unix)]
#[test]
fn mode_only_empty_deletion_and_empty_addition_roundtrip() {
    use std::os::unix::fs::PermissionsExt;
    let work = Workspace::new();
    fs::write(work.0.join("source/mode"), "unchanged\n").unwrap();
    fs::write(work.0.join("source/empty"), "").unwrap();
    work.commit();
    work.git(
        "source",
        &[
            "clone",
            work.0.join("source").to_str().unwrap(),
            work.0.join("target").to_str().unwrap(),
        ],
    );
    fs::set_permissions(
        work.0.join("source/mode"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    fs::remove_file(work.0.join("source/empty")).unwrap();
    fs::write(work.0.join("source/new"), "").unwrap();
    work.export(&["mode", "empty", "new"]);
    receive::verify(&work.0.join("bundle"))
        .unwrap()
        .check(&work.0.join("target"))
        .unwrap()
        .write()
        .unwrap();
    assert_eq!(
        fs::read_to_string(work.0.join("target/mode")).unwrap(),
        "unchanged\n"
    );
    assert_ne!(
        fs::metadata(work.0.join("target/mode"))
            .unwrap()
            .permissions()
            .mode()
            & 0o111,
        0
    );
    assert!(!work.0.join("target/empty").exists());
    assert_eq!(fs::metadata(work.0.join("target/new")).unwrap().len(), 0);
}

#[test]
fn cli_verifies_v1_and_checks_v2_without_implicit_writes() {
    let work = Workspace::fixture();
    let binary = env!("CARGO_BIN_EXE_memory-pier");
    let verified = Command::new(binary)
        .arg("verify")
        .arg(work.0.join("bundle"))
        .output()
        .unwrap();
    assert!(verified.status.success());
    let check = Command::new(binary)
        .arg("apply")
        .arg(work.0.join("bundle"))
        .arg("--project")
        .arg(work.0.join("target"))
        .arg("--check")
        .output()
        .unwrap();
    assert!(check.status.success());
    assert!(String::from_utf8_lossy(&check.stdout).contains("\"written\": false"));
    let missing = Command::new(binary)
        .arg("apply")
        .arg(work.0.join("bundle"))
        .arg("--project")
        .arg(work.0.join("target"))
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(64));
    let v1 = Workspace::new();
    v1.commit();
    v1.export(&[]);
    let verified = receive::verify(&v1.0.join("bundle")).unwrap();
    assert_eq!(verified.report().format_version, 1);
    assert!(verified.check(&v1.0.join("source")).is_err());
    let write = Command::new(binary)
        .arg("apply")
        .arg(work.0.join("bundle"))
        .arg("--project")
        .arg(work.0.join("target"))
        .arg("--write")
        .output()
        .unwrap();
    assert!(
        write.status.success(),
        "{}",
        String::from_utf8_lossy(&write.stderr)
    );
}

#[test]
fn resource_limits_case_collisions_and_duplicate_json_fields_fail() {
    let work = Workspace::fixture();
    let original = work.manifest();
    let mut changed = original.clone();
    changed["selected_paths"][1] = "A.TXT".into();
    work.save(&changed);
    assert!(receive::verify(&work.0.join("bundle")).is_err());
    let mut changed = original.clone();
    changed["selected_paths"][1] = "a.txt/child".into();
    work.save(&changed);
    assert!(receive::verify(&work.0.join("bundle")).is_err());
    let encoded = serde_json::to_string(&original).unwrap().replacen(
        "\"format_version\":2",
        "\"format_version\":2,\"format_version\":2",
        1,
    );
    fs::write(work.0.join("bundle/manifest.json"), encoded).unwrap();
    assert!(receive::verify(&work.0.join("bundle")).is_err());
    fs::File::create(work.0.join("bundle/manifest.json"))
        .unwrap()
        .set_len(1024 * 1024 + 1)
        .unwrap();
    assert!(receive::verify(&work.0.join("bundle")).is_err());
    work.save(&original);
    fs::File::create(work.0.join("bundle/changes.patch"))
        .unwrap()
        .set_len(17 * 1024 * 1024 + 1)
        .unwrap();
    assert!(receive::verify(&work.0.join("bundle")).is_err());
}

#[cfg(unix)]
#[test]
fn application_does_not_execute_configured_hooks_or_filters() {
    let work = Workspace::fixture();
    let target = work.0.join("target");
    fs::write(
        target.join(".git/info/attributes"),
        "a.txt filter=synthetic\n",
    )
    .unwrap();
    work.git(
        "target",
        &["config", "filter.synthetic.clean", "touch EXECUTED; cat"],
    );
    work.git(
        "target",
        &[
            "config",
            "filter.synthetic.process",
            "touch EXECUTED; exit 1",
        ],
    );
    work.git("target", &["config", "core.fsmonitor", "touch EXECUTED"]);
    work.git("target", &["config", "core.hooksPath", "invalid-hooks"]);
    receive::verify(&work.0.join("bundle"))
        .unwrap()
        .check(&target)
        .unwrap()
        .write()
        .unwrap();
    assert!(!target.join("EXECUTED").exists());
}
