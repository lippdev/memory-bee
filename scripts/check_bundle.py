"""Validate actual synthetic exports against the versioned schema and SHA-256."""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parent.parent
SCHEMA = json.loads((ROOT / "schemas/bundle-v1.schema.json").read_text())
Draft202012Validator.check_schema(SCHEMA)
VALIDATOR = Draft202012Validator(SCHEMA, format_checker=FormatChecker())
BINARY = ROOT / "target/debug/memory-pier"


def check_bundle(path, excluded_lines):
    manifest = json.loads((path / "manifest.json").read_text())
    VALIDATOR.validate(manifest)
    datetime.datetime.fromisoformat(manifest["created_at"].replace("Z", "+00:00"))
    assert {item["path"] for item in manifest["files"]} == {"HANDOFF.md", "history.jsonl"}
    for item in manifest["files"]:
        assert hashlib.sha256((path / item["path"]).read_bytes()).hexdigest() == item["sha256"]
    events = [json.loads(line) for line in (path / "history.jsonl").read_text().splitlines()]
    assert [event["sequence"] for event in events] == list(range(1, len(events) + 1))
    assert events and all(event["source"]["line"] > 0 for event in events)
    assert not any(event["source"]["line"] in excluded_lines for event in events)
    assert manifest["redaction"] == "pending-review"
    assert manifest["code_state"] == "unknown"
    invalid = dict(manifest, format_version=99)
    assert not VALIDATOR.is_valid(invalid)


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
print("Generated bundles: schema v1, timestamps, provenance, exclusions and independent hashes OK")
