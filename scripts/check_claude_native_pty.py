#!/usr/bin/env python3
"""Exercise Memory Bee's Claude terminal with a fake CLI; no model calls."""
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import tempfile
import termios
import time


def one_run(binary, project, state, env, resume=False):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 20, 80, 0, 0))
    before = termios.tcgetattr(slave)
    argv = [str(binary), 'workspace', '--claude', '--claude-terminal', '--project', str(project), '--state', str(state), '--no-color']
    if resume:
        argv.append('--resume')
    proc = subprocess.Popen(argv, stdin=slave, stdout=slave, stderr=slave, env=env)
    output = bytearray()

    def drain_until(token, seconds=5):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.05)[0]:
                chunk = os.read(master, 65536)
                output.extend(chunk)
                if b'\x1b[6n' in chunk:
                    os.write(master, b'\x1b[1;1R')
            if token in output:
                return
            if proc.poll() is not None:
                break
        raise AssertionError(f'did not see synthetic {token!r}; exit={proc.poll()}')

    try:
        drain_until(b'READY')
        assert (state / 'claude-native.json').exists()
        os.write(master, b'hello\r')
        drain_until(b'GOT:')
        proc.wait(timeout=5)
        assert proc.returncode == 0
        assert termios.tcgetattr(slave) == before, 'terminal state not restored'
        assert not (state / 'claude-native.lock').exists()
        return json.loads((state / 'claude-native.json').read_text())
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)


def main():
    repo = Path(__file__).resolve().parent.parent
    binary = repo / 'target/debug/memory-bee'
    with tempfile.TemporaryDirectory(prefix='memory-bee-claude-native-') as tmp:
        root = Path(tmp)
        fake_dir = root / 'bin'
        fake_dir.mkdir()
        fake = fake_dir / 'claude'
        fake.write_text('#!/bin/sh\nprintf "%s|%s|%s\\n" "$1" "$2" "$PWD" >> "$BEE_FAKE_ARGS"\nprintf "FAKE CLAUDE READY\\r\\n"\nIFS= read -r line\nprintf "GOT:%s\\r\\n" "$line"\n')
        fake.chmod(0o700)
        project = root / 'project'
        project.mkdir()
        state = root / 'state'
        args_file = root / 'claude-args'
        env = dict(os.environ, PATH=f'{fake_dir}:/usr/bin:/bin', TERM='xterm-256color', BEE_FAKE_ARGS=str(args_file))
        first = one_run(binary, project, state, env)
        assert first['version'] == 1
        assert len(first['sessions']) == 1
        session_id = first['sessions'][0]['id']
        assert first['sessions'][0]['exit_code'] == 0
        second = one_run(binary, project, state, env, resume=True)
        assert len(second['sessions']) == 1
        assert second['sessions'][0]['id'] == session_id
        assert args_file.read_text().splitlines() == [
            f'--session-id|{session_id}|{project.resolve()}',
            f'--resume|{session_id}|{project.resolve()}',
        ], 'Claude must receive the native session ID in the selected project'
        assert list(project.iterdir()) == [], 'fake CLI modified the project'
        denied_state = root / 'denied'
        result = subprocess.run([str(binary), 'workspace', '--claude', '--project', str(project), '--state', str(denied_state)], capture_output=True)
        assert result.returncode == 64 and not denied_state.exists(), 'non-TTY launch should refuse before writing'
    print('PASS: native Claude PTY, input, session ID, resume, private state and terminal restoration (fake CLI)')


if __name__ == '__main__':
    main()
