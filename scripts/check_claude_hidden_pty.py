#!/usr/bin/env python3
"""Exercise the Bee-only Claude UI with synthetic official-hook events."""
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


FAKE = r'''#!/usr/bin/env python3
import json, os, subprocess, sys
args = sys.argv[1:]
session_id = args[args.index('--resume') + 1] if '--resume' in args else args[args.index('--session-id') + 1]
settings = json.loads(args[args.index('--settings') + 1])
assert all(name in settings['hooks'] for name in ('SessionStart','MessageDisplay','PermissionRequest'))
with open(os.environ['BEE_FAKE_ARGS'], 'a') as f:
    f.write(f'{args[0]}|{session_id}|{os.getcwd()}\n')
with open(os.environ['BEE_SOCKETS'], 'a') as f:
    f.write(os.environ['MEMORY_BEE_CLAUDE_SOCKET'] + '\n')
def hook(name, **fields):
    payload = json.dumps(dict(hook_event_name=name, session_id=session_id, **fields))
    return subprocess.run([os.environ['BEE_BINARY'], '__claude-hook'], input=payload, text=True, capture_output=True, check=True).stdout
print('CLAUDE ORIGINAL SHOULD STAY HIDDEN', flush=True)
hook('SessionStart')
prompt = sys.stdin.readline().strip()
hook('UserPromptSubmit', prompt=prompt)
hook('MessageDisplay', delta='resposta sintetica', final=True, index=0)
decision = json.loads(hook('PermissionRequest', tool_name='Bash', tool_input={'command':'echo synthetic'}))
with open(os.environ['BEE_DECISIONS'], 'a') as f:
    f.write(decision['hookSpecificOutput']['decision']['behavior'] + '\n')
hook('Stop')
'''


def run(binary, project, state, env, resume, decision):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 24, 80, 0, 0))
    before = termios.tcgetattr(slave)
    argv = [str(binary), 'workspace', '--claude', '--project', str(project), '--state', str(state), '--no-color']
    if resume:
        argv.append('--resume')
    proc = subprocess.Popen(argv, stdin=slave, stdout=slave, stderr=slave, env=env)
    output = bytearray()

    def until(token):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.05)[0]:
                try:
                    output.extend(os.read(master, 65536))
                except OSError:
                    break
            if token in output:
                return
            if proc.poll() is not None:
                break
        raise AssertionError(f'missing Bee UI token {token!r}; process exit={proc.poll()}; Bee output={output[-400:]!r}')

    try:
        until(b'pronto.')
        os.write(master, b'hello\r')
        until(b'Permitir esta')
        os.write(master, decision.encode())
        until(b'sintetica')
        proc.wait(timeout=8)
        assert proc.returncode == 0, proc.returncode
        assert b'CLAUDE ORIGINAL SHOULD STAY HIDDEN' not in output
        assert termios.tcgetattr(slave) == before
        assert not (state / 'claude-native.lock').exists()
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)


def main():
    repo = Path(__file__).resolve().parent.parent
    binary = repo / 'target/debug/memory-bee'
    with tempfile.TemporaryDirectory(prefix='bee-hidden-') as temp:
        root = Path(temp)
        fake_dir = root / 'bin'
        fake_dir.mkdir()
        fake = fake_dir / 'claude'
        fake.write_text(FAKE)
        fake.chmod(0o700)
        project = root / 'project'
        project.mkdir()
        state = root / 'state'
        args_file = root / 'args'
        decisions = root / 'decisions'
        sockets = root / 'sockets'
        env = dict(os.environ, PATH=f'{fake_dir}:/usr/bin:/bin', BEE_BINARY=str(binary), BEE_FAKE_ARGS=str(args_file), BEE_DECISIONS=str(decisions), BEE_SOCKETS=str(sockets))
        run(binary, project, state, env, False, 'n')
        session = json.loads((state / 'claude-native.json').read_text())['sessions'][0]['id']
        run(binary, project, state, env, True, 'y')
        run(binary, project, state, env, True, '\x11')  # Ctrl+Q denies pending permission before exit.
        assert decisions.read_text().splitlines() == ['deny', 'allow', 'deny']
        assert args_file.read_text().splitlines() == [f'--session-id|{session}|{project.resolve()}', f'--resume|{session}|{project.resolve()}', f'--resume|{session}|{project.resolve()}']
        assert len(json.loads((state / 'claude-native.json').read_text())['sessions']) == 1
        assert all(not Path(path).exists() for path in sockets.read_text().splitlines())
        denied = subprocess.run([str(binary), '__claude-hook'], input=json.dumps({'hook_event_name':'PermissionRequest'}), text=True, capture_output=True, env=dict(env, MEMORY_BEE_CLAUDE_SOCKET=str(root / 'missing.sock')))
        assert denied.returncode == 0
        assert json.loads(denied.stdout)['hookSpecificOutput']['decision']['behavior'] == 'deny'
    print('PASS: Bee-only UI, hidden original output, hooks, deny/allow, Ctrl+Q denial, resume and terminal restoration')


if __name__ == '__main__':
    main()
