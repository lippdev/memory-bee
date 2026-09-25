#!/usr/bin/env python3
"""Exercise terminal handoff using a synthetic checkout and a fake agent only.

Run after cargo build --locked. Requires POSIX PTYs and system Git; no packages.
"""
import errno
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import struct
import subprocess
import tempfile
import termios
import time


def main():
    repo = Path(__file__).resolve().parent.parent
    binary = str(repo / "target/debug/memory-bee")
    with tempfile.TemporaryDirectory(prefix="memory-bee-pty-") as tmp:
        root = Path(tmp)
        project, fake = root / "project", root / "bin"
        project.mkdir()
        fake.mkdir()

        def git(*args):
            subprocess.run(
                ["git", "-C", str(project), *args], check=True,
                stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
            )

        git("init", "-b", "synthetic")
        for key, value in [
            ("user.name", "Synthetic"),
            ("user.email", "synthetic@example.invalid"),
            ("commit.gpgsign", "false"),
            ("core.hooksPath", "/dev/null"),
        ]:
            git("config", key, value)
        (project / "a.txt").write_text("base\n")
        git("add", ".")
        git("commit", "-m", "fixture")
        marker, prompt = root / "agent-result", root / "prompt"
        bundle = repo / "examples/bundle-v1"
        agent = fake / "claude"
        agent.write_text(
            '#!/bin/sh\n[ -t 0 ] && [ -t 1 ] || exit 22\n'
            'stty -a > "$FAKE_RESULT"\nprintf "fake agent complete\\n"\nexit 7\n'
        )
        agent.chmod(0o700)
        # Real agent binaries are never in this child's PATH.
        env = dict(os.environ, PATH=f"{fake}:/usr/bin:/bin",
                   TERM="xterm-256color", FAKE_RESULT=str(marker))
        preview = subprocess.run(
            [binary, "prepare-resume", str(bundle), "--target", "claude",
             "--project", str(project), "--preview"], env=env, capture_output=True,
        )
        assert preview.returncode in (0, 2), preview.stderr.decode()
        token = json.loads(preview.stdout)["confirmation"]
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
        proc = subprocess.Popen(
            [binary, "dashboard", "--project", str(project), "--claude-root",
             str(repo / "testdata/claude-projects"), "--bundle", str(bundle), "--no-color"],
            stdin=slave, stdout=slave, stderr=slave, env=env,
        )
        output = bytearray()

        def drain(seconds=0.3):
            end = time.monotonic() + seconds
            while time.monotonic() < end:
                if select.select([master], [], [], max(0, end - time.monotonic()))[0]:
                    try:
                        data = os.read(master, 65536)
                        if not data:
                            break
                        output.extend(data)
                        # Behave like a terminal answering cursor-position queries.
                        if b"\x1b[6n" in data:
                            os.write(master, b"\x1b[1;1R")
                    except OSError as error:
                        if error.errno != errno.EIO:
                            raise
                        break

        def send(data):
            os.write(master, data.encode())
            drain()

        def reported_exit():
            # Ratatui may position past blank cells instead of printing spaces.
            text = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", bytes(output))
            return "Códigodesaída:7".encode() in re.sub(rb"\s+", b"", text)

        try:
            drain()
            send("r")
            send("l")
            send(str(prompt))
            send("\r")
            assert not prompt.exists()
            send(token)
            send("\r")
            deadline = time.monotonic() + 10
            while not reported_exit() and time.monotonic() < deadline:
                drain()
            assert marker.exists(), "fake agent never started"
            assert prompt.exists(), "prompt was not preserved"
            settings = marker.read_text()
            assert "-icanon" not in settings and "-echo " not in settings, settings
            assert reported_exit(), "dashboard did not report the fake agent's exit"
            send("\r")
            send("q")
            proc.wait(timeout=5)
            assert proc.returncode in (0, 2), proc.returncode
            restored = termios.tcgetattr(slave)[3]
            assert restored & termios.ICANON and restored & termios.ECHO
            print("PASS: fake agent inherited cooked TTY; exit 7 shown; prompt preserved; "
                  "dashboard returned; terminal restored")
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait()
            os.close(master)
            os.close(slave)


if __name__ == "__main__":
    main()
