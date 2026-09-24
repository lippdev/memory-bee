"""Validate actual synthetic exports against the versioned schema and SHA-256."""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import copy

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parent.parent
VALIDATORS = {}
for version in (1, 2):
    schema = json.loads((ROOT / f"schemas/bundle-v{version}.schema.json").read_text())
    Draft202012Validator.check_schema(schema)
    VALIDATORS[version] = Draft202012Validator(schema, format_checker=FormatChecker())
BINARY = ROOT / "target/debug/memory-pier"


def check_bundle(path, excluded_lines, code_state="unknown"):
    manifest = json.loads((path / "manifest.json").read_text())
    validator = VALIDATORS[manifest["format_version"]]
    validator.validate(manifest)
    datetime.datetime.fromisoformat(manifest["created_at"].replace("Z", "+00:00"))
    actual = {str(p.relative_to(path)) for p in path.rglob("*") if p.is_file()} - {"manifest.json"}
    assert {item["path"] for item in manifest["files"]} == actual
    assert {"HANDOFF.md", "history.jsonl"} <= actual
    for item in manifest["files"]:
        assert hashlib.sha256((path / item["path"]).read_bytes()).hexdigest() == item["sha256"]
    events = [json.loads(line) for line in (path / "history.jsonl").read_text().splitlines()]
    assert [event["sequence"] for event in events] == list(range(1, len(events) + 1))
    assert events and all(event["source"]["line"] > 0 for event in events)
    assert not any(event["source"]["line"] in excluded_lines for event in events)
    assert manifest["redaction"] == "pending-review"
    assert manifest["code_state"] == code_state
    invalid = dict(manifest, format_version=99)
    assert not validator.is_valid(invalid)


with tempfile.TemporaryDirectory(prefix="memory-pier-schema-") as directory:
    cases = [("basic.jsonl", 0, []), ("basic.jsonl", 0, [1]), ("truncated.jsonl", 2, []), ("tools.jsonl", 2, [])]
    for index, (fixture, code, excluded_lines) in enumerate(cases):
        output = Path(directory) / str(index)
        extra = [argument for line in excluded_lines for argument in ("--exclude-line", str(line))]
        result = subprocess.run(
            [str(BINARY), "export", str(ROOT / "testdata/claude" / fixture), "--output", str(output), *extra],
            capture_output=True,
            text=True,
            check=False,
        )
        assert result.returncode == code, result.stderr
        assert json.loads(result.stdout)["written"]
        check_bundle(output, excluded_lines)
    repo = Path(directory) / "repo"
    repo.mkdir()
    def git(*args):
        return subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True, text=True).stdout.strip()
    git("init", "-b", "synthetic")
    git("config", "user.name", "Synthetic")
    git("config", "user.email", "synthetic@example.invalid")
    git("-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null", "commit", "--allow-empty", "-m", "synthetic")
    commit = git("rev-parse", "HEAD")
    output = Path(directory) / "git-bundle"
    subprocess.run([str(BINARY), "export", str(ROOT / "testdata/claude/basic.jsonl"), "--project", str(repo), "--output", str(output)], check=True, capture_output=True)
    check_bundle(output, [], "base-reference")
    manifest = json.loads((output / "manifest.json").read_text())
    assert manifest["project"]["base_commit"] == commit
    assert manifest["project"]["dirty"] is False
    (repo / "tracked.txt").write_text("base\n")
    git("add", "tracked.txt")
    git("-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null", "commit", "-m", "text base")
    commit = git("rev-parse", "HEAD")
    (repo / "tracked.txt").write_text("selected change\n")
    (repo / "new.txt").write_text("synthetic new text\n")
    output = Path(directory) / "code-bundle"
    subprocess.run([str(BINARY), "export", str(ROOT / "testdata/claude/basic.jsonl"), "--project", str(repo), "--include-path", "tracked.txt", "--include-path", "new.txt", "--output", str(output)], check=True, capture_output=True)
    check_bundle(output, [], "changes-included")
    manifest = json.loads((output / "manifest.json").read_text())
    assert manifest["format_version"] == 2
    assert manifest["project"]["base_commit"] == commit
    validator = VALIDATORS[2]
    for change in manifest["changes"]:
        assert change["path"] in manifest["selected_paths"]
        assert change["result_sha256"] == hashlib.sha256((repo / change["path"]).read_bytes()).hexdigest()
        if change["kind"] == "add":
            assert (output / change["payload"]).read_bytes() == (repo / change["path"]).read_bytes()
    invalid = copy.deepcopy(manifest)
    invalid["changes"][0]["path"] = "../escape"
    assert not validator.is_valid(invalid)
    invalid = copy.deepcopy(manifest)
    invalid["changes"][0]["base_mode"] = "100644"  # add must have null base
    assert not validator.is_valid(invalid)
    invalid = copy.deepcopy(manifest)
    invalid["project"]["base_commit"] = None
    assert not validator.is_valid(invalid)
    assert not VALIDATORS[1].is_valid(manifest)
    (repo / "binary").write_bytes(b"\0synthetic")
    output = Path(directory) / "omitted-bundle"
    result = subprocess.run([str(BINARY), "export", str(ROOT / "testdata/claude/basic.jsonl"), "--project", str(repo), "--include-path", "binary", "--output", str(output)], capture_output=True)
    assert result.returncode == 2
    check_bundle(output, [], "base-reference")
print("Generated bundles: v1/v2 schemas, timestamps, provenance, selection, exclusions and independent hashes OK")
