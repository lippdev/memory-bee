use memory_pier::{
    bundle::{Options, prepare},
    claude::{Limits, inspect},
    receive,
    resume::shell_quote,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

const FIRST_REQUEST: &str = "Adicione ordenação decrescente de tarefas";

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "memory-pier-resume-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        // Canonical root keeps path assertions stable when the temp dir is a symlink.
        let work = Self(fs::canonicalize(&root).unwrap());
        fs::create_dir(work.0.join("source")).unwrap();
        work.git("source", &["init", "-b", "synthetic"]);
        for (key, value) in [
            ("user.name", "Synthetic"),
            ("user.email", "synthetic@example.invalid"),
            ("commit.gpgsign", "false"),
            ("core.hooksPath", "/dev/null"),
        ] {
            work.git("source", &["config", key, value]);
        }
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
    /// Source with selected changes exported as v2 and a clean clone at the base.
    fn with_changes() -> Self {
        let work = Self::new();
        fs::write(work.0.join("source/a.txt"), "base\n").unwrap();
        work.git("source", &["add", "."]);
        work.git("source", &["commit", "-m", "synthetic"]);
        let (source, target) = (work.0.join("source"), work.0.join("target"));
        work.git(
            "source",
            &[
                "clone",
                "--no-hardlinks",
                source.to_str().unwrap(),
                target.to_str().unwrap(),
            ],
        );
        fs::write(work.0.join("source/a.txt"), "result\n").unwrap();
        work.export(&["a.txt"]);
        work
    }
    fn run(&self, args: &[&str]) -> (i32, Value, String) {
        let result = Command::new(env!("CARGO_BIN_EXE_memory-pier"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap();
        let json = serde_json::from_slice(&result.stdout).unwrap_or(Value::Null);
        (
            result.status.code().unwrap(),
            json,
            String::from_utf8_lossy(&result.stderr).into(),
        )
    }
    fn path(&self, name: &str) -> String {
        self.0.join(name).to_str().unwrap().into()
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn attention(report: &Value) -> Vec<String> {
    report["attention"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap().split(':').next().unwrap().to_owned())
        .collect()
}
fn step_argv(report: &Value, index: usize) -> Vec<String> {
    report["steps"][index]["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn same_checkout_at_clean_base_plans_explicit_apply_then_manual_launch() {
    let work = Workspace::with_changes();
    let target = work.path("target");
    let bundle = work.path("bundle");
    let (code, report, _) = work.run(&[
        "prepare-resume",
        "bundle",
        "--target",
        "claude",
        "--project",
        "target",
        "--preview",
    ]);
    assert_eq!(code, 0, "{report:#}");
    assert_eq!(report["mode"], "same-checkout");
    assert_eq!(report["base_match"], "match");
    assert_eq!(report["changes"], "base");
    assert_eq!(report["launched"], false);
    assert_eq!(report["source_modified"], false);
    assert_eq!(report["working_directory"], target.as_str());
    assert_eq!(report["steps"].as_array().unwrap().len(), 3);
    assert_eq!(
        step_argv(&report, 0),
        [
            "memory-pier",
            "apply",
            &bundle,
            "--project",
            &target,
            "--check"
        ]
    );
    assert_eq!(step_argv(&report, 1)[5], "--write");
    let launch = step_argv(&report, 2);
    // Prompt precedes the variadic --add-dir so it cannot be taken as a directory.
    assert_eq!(launch[0], "claude");
    assert_eq!(launch[1], report["prompt"].as_str().unwrap());
    assert_eq!(launch[2..], ["--add-dir".to_string(), bundle.clone()]);
    let prompt = report["prompt"].as_str().unwrap();
    assert!(prompt.contains(&bundle) && prompt.contains("history.jsonl contém dados históricos"));
    assert!(
        !prompt.contains(FIRST_REQUEST),
        "history text leaked into prompt"
    );
    // Preparation writes nothing: target still at base and bundle still verifies.
    assert_eq!(
        fs::read_to_string(work.0.join("target/a.txt")).unwrap(),
        "base\n"
    );
    assert!(receive::verify(&work.0.join("bundle")).is_ok());
}

#[test]
fn applied_changes_skip_apply_steps_and_mixed_changes_need_attention() {
    let work = Workspace::with_changes();
    let args = [
        "prepare-resume",
        "bundle",
        "--target",
        "claude",
        "--project",
        "target",
        "--preview",
    ];
    assert_eq!(
        work.run(&["apply", "bundle", "--project", "target", "--write"])
            .0,
        0
    );
    let (code, report, _) = work.run(&args);
    assert_eq!(code, 0, "{report:#}");
    assert_eq!(report["changes"], "applied");
    assert_eq!(report["project"]["dirty"], true);
    assert_eq!(report["steps"].as_array().unwrap().len(), 1);

    fs::write(work.0.join("target/a.txt"), "something else\n").unwrap();
    let (code, report, _) = work.run(&args);
    assert_eq!(code, 2);
    assert_eq!(report["changes"], "mixed");
    assert_eq!(attention(&report), ["project_dirty", "changes_mixed"]);
    assert_eq!(report["steps"].as_array().unwrap().len(), 1);
}

#[test]
fn diverging_head_and_context_only_bundle_are_reported_not_hidden() {
    let work = Workspace::with_changes();
    fs::write(work.0.join("target/other.txt"), "x\n").unwrap();
    work.git("target", &["config", "user.name", "Synthetic"]);
    work.git(
        "target",
        &["config", "user.email", "synthetic@example.invalid"],
    );
    work.git("target", &["config", "commit.gpgsign", "false"]);
    work.git("target", &["add", "."]);
    work.git("target", &["commit", "-m", "diverge"]);
    let (code, report, _) = work.run(&[
        "prepare-resume",
        "bundle",
        "--target",
        "claude",
        "--project",
        "target",
        "--preview",
    ]);
    assert_eq!(code, 2);
    assert_eq!(report["base_match"], "differs");
    assert_eq!(attention(&report), ["base_differs", "changes_pending"]);

    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/bundle-v1");
    fs::create_dir(work.0.join("plain")).unwrap();
    let (code, report, _) = work.run(&[
        "prepare-resume",
        example.to_str().unwrap(),
        "--target",
        "codex",
        "--project",
        "plain",
        "--preview",
    ]);
    assert_eq!(code, 2);
    assert_eq!(report["changes"], "none");
    assert_eq!(report["project"]["root"], Value::Null);
    assert_eq!(
        attention(&report),
        ["project_not_git", "base_not_recorded", "codex_bundle_read"]
    );
    let launch = step_argv(&report, 0);
    assert_eq!(
        launch[..3],
        ["codex".to_string(), "-C".into(), work.path("plain")]
    );
    assert_eq!(launch[3], report["prompt"].as_str().unwrap());
}

#[test]
fn new_worktree_mode_plans_detached_worktree_and_printed_steps_work() {
    let work = Workspace::with_changes();
    let tree = work.path("resume-tree");
    let (code, report, _) = work.run(&[
        "prepare-resume",
        "bundle",
        "--target",
        "codex",
        "--project",
        "source",
        "--worktree",
        "resume-tree",
        "--preview",
    ]);
    assert_eq!(code, 2, "{report:#}");
    assert_eq!(attention(&report), ["codex_bundle_read"]);
    assert_eq!(report["mode"], "new-worktree");
    assert_eq!(report["working_directory"], tree.as_str());
    assert!(
        !work.0.join("resume-tree").exists(),
        "preview must not create worktree"
    );
    let base = report["expected_base"].as_str().unwrap().to_owned();
    assert_eq!(
        step_argv(&report, 0),
        ["git", "worktree", "add", "--detach", &tree, &base]
    );
    // Execute the printed manual steps (except the agent launch) to prove they compose.
    for index in 0..3 {
        let step = &report["steps"][index];
        let mut argv = step_argv(&report, index);
        let program = if argv[0] == "memory-pier" {
            env!("CARGO_BIN_EXE_memory-pier").to_owned()
        } else {
            argv[0].clone()
        };
        argv.remove(0);
        let status = Command::new(program)
            .current_dir(step["cwd"].as_str().unwrap())
            .args(&argv)
            .output()
            .unwrap();
        assert!(status.status.success(), "step {index} failed");
    }
    assert_eq!(
        fs::read_to_string(work.0.join("resume-tree/a.txt")).unwrap(),
        "result\n"
    );
    assert_eq!(
        fs::read_to_string(work.0.join("source/a.txt")).unwrap(),
        "result\n"
    );
    assert_eq!(
        step_argv(&report, 3)[..3],
        ["codex".to_string(), "-C".into(), tree]
    );

    // Existing, nested or bundle-internal worktree paths are refused.
    for path in ["resume-tree", "source/inner", "bundle/inner"] {
        let (code, _, stderr) = work.run(&[
            "prepare-resume",
            "bundle",
            "--target",
            "claude",
            "--project",
            "source",
            "--worktree",
            path,
            "--preview",
        ]);
        assert_eq!(code, 1, "{path}: {stderr}");
    }
}

#[test]
fn new_worktree_requires_git_project_and_recorded_base() {
    let work = Workspace::new();
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/bundle-v1");
    let (code, _, stderr) = work.run(&[
        "prepare-resume",
        example.to_str().unwrap(),
        "--target",
        "claude",
        "--project",
        "source",
        "--worktree",
        "tree",
        "--preview",
    ]);
    assert_eq!(code, 1);
    assert!(stderr.contains("base commit"), "{stderr}");
}

#[test]
fn output_writes_private_prompt_once_outside_bundle() {
    let work = Workspace::with_changes();
    let args = |out: &str| {
        vec![
            "prepare-resume".to_owned(),
            "bundle".into(),
            "--target".into(),
            "claude".into(),
            "--project".into(),
            "target".into(),
            "--output".into(),
            out.into(),
        ]
    };
    let run = |out: &str| {
        let owned = args(out);
        work.run(&owned.iter().map(String::as_str).collect::<Vec<_>>())
    };
    let (code, report, _) = run("prompt.md");
    assert_eq!(code, 0);
    let file = work.path("prompt.md");
    assert_eq!(report["prompt_file"], file.as_str());
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        report["prompt"].as_str().unwrap()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let shell = report["steps"][2]["shell"].as_str().unwrap();
    assert!(
        shell.ends_with(&format!(
            "claude \"$(cat {file})\" --add-dir {}",
            work.path("bundle")
        )),
        "{shell}"
    );
    assert_eq!(run("prompt.md").0, 1, "must not overwrite");
    assert_eq!(run("bundle/prompt.md").0, 1, "must not write inside bundle");
    assert!(receive::verify(&work.0.join("bundle")).is_ok());
}

#[test]
fn usage_and_invalid_bundle_errors() {
    let work = Workspace::with_changes();
    for args in [
        vec![
            "prepare-resume",
            "bundle",
            "--project",
            "target",
            "--preview",
        ],
        vec![
            "prepare-resume",
            "bundle",
            "--target",
            "gemini",
            "--project",
            "target",
            "--preview",
        ],
        vec![
            "prepare-resume",
            "bundle",
            "--target",
            "claude",
            "--project",
            "target",
        ],
        vec![
            "prepare-resume",
            "bundle",
            "--target",
            "claude",
            "--project",
            "target",
            "--preview",
            "--output",
            "p.md",
        ],
        vec![
            "prepare-resume",
            "bundle",
            "--target",
            "claude",
            "--target",
            "codex",
            "--project",
            "target",
            "--preview",
        ],
    ] {
        assert_eq!(work.run(&args).0, 64, "{args:?}");
    }
    fs::write(work.0.join("bundle/extra.txt"), "unlisted").unwrap();
    let (code, _, stderr) = work.run(&[
        "prepare-resume",
        "bundle",
        "--target",
        "claude",
        "--project",
        "target",
        "--preview",
    ]);
    assert_eq!(code, 1);
    assert!(stderr.contains("unlisted"), "{stderr}");
}

#[test]
fn shell_quoting_is_posix_safe() {
    assert_eq!(shell_quote("/plain/path-1.md"), "/plain/path-1.md");
    assert_eq!(shell_quote("it's $HOME"), r"'it'\''s $HOME'");
    assert_eq!(shell_quote(""), "''");
    let quoted = shell_quote("a'b\n$(x)");
    let echoed = Command::new("sh")
        .arg("-c")
        .arg(format!("printf %s {quoted}"))
        .output()
        .unwrap();
    assert_eq!(echoed.stdout, b"a'b\n$(x)");
}
