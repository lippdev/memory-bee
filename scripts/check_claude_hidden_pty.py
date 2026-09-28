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
import json, os, signal, subprocess, sys
signal.signal(signal.SIGINT, signal.SIG_IGN)  # Real Claude reads Ctrl+C as a key.
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
# Read allowed by Claude's own rules: no PermissionRequest.
hook('PreToolUse', tool_name='Read', tool_use_id='r1', tool_input={'file_path':'README.md'})
hook('PostToolUse', tool_name='Read', tool_use_id='r1', tool_input={'file_path':'README.md'})
hook('PreToolUse', tool_name='Bash', tool_use_id='b1', tool_input={'command':'echo synthetic'})
decision = json.loads(hook('PermissionRequest', tool_name='Bash', tool_input={'command':'echo synthetic'}))
behavior = decision['hookSpecificOutput']['decision']['behavior']
with open(os.environ['BEE_DECISIONS'], 'a') as f:
    f.write(behavior + '\n')
if behavior == 'allow':
    hook('PostToolUse', tool_name='Bash', tool_use_id='b1', tool_input={'command':'echo synthetic'})
hook('Stop')
'''

# Untrusted project: a native dialog waits without emitting any hook.
FAKE_TRUST = r'''#!/usr/bin/env python3
import json, os, subprocess, sys
args = sys.argv[1:]
session_id = args[args.index('--resume') + 1] if '--resume' in args else args[args.index('--session-id') + 1]
def hook(name, **fields):
    payload = json.dumps(dict(hook_event_name=name, session_id=session_id, **fields))
    subprocess.run([os.environ['BEE_BINARY'], '__claude-hook'], input=payload, text=True, capture_output=True, check=True)
print('Do you trust the files in this folder? 1. Yes 2. No', flush=True)
for line in sys.stdin:
    with open(os.environ['BEE_FAKE_STDIN'], 'a') as f:
        f.write(line.strip() + '\n')
    if line.strip() == '1':
        hook('SessionStart')
    elif line.strip() == '/exit':
        sys.exit(0)
'''

# Resumable session: reports its transcript through the official hook field.
FAKE_HISTORY = r'''#!/usr/bin/env python3
import json, os, shutil, subprocess, sys
args = sys.argv[1:]
resume = '--resume' in args
session_id = args[args.index('--resume') + 1] if resume else args[args.index('--session-id') + 1]
transcript = os.path.join(os.environ['BEE_TRANSCRIPTS'], session_id + '.jsonl')
shutil.copy(os.environ['BEE_FIXTURE'], transcript)
payload = json.dumps(dict(hook_event_name='SessionStart', session_id=session_id, transcript_path=transcript, source='resume' if resume else 'startup'))
subprocess.run([os.environ['BEE_BINARY'], '__claude-hook'], input=payload, text=True, capture_output=True, check=True)
for line in sys.stdin:
    with open(os.environ['BEE_FAKE_STDIN'], 'a') as f:
        f.write(line.strip() + '\n')
    if line.strip() == '/exit':
        sys.exit(0)
'''

# Editor checks: records exactly what Claude would receive on its input.
FAKE_EDITOR = r'''#!/usr/bin/env python3
import json, os, signal, subprocess, sys
signal.signal(signal.SIGINT, signal.SIG_IGN)  # Real Claude reads Ctrl+C as a key.
args = sys.argv[1:]
session_id = args[args.index('--session-id') + 1]
if os.environ.get('BEE_BRACKETED'):
    sys.stdout.write('\x1b[?2004h')
    sys.stdout.flush()
def hook(name, **fields):
    payload = json.dumps(dict(hook_event_name=name, session_id=session_id, **fields))
    subprocess.run([os.environ['BEE_BINARY'], '__claude-hook'], input=payload, text=True, capture_output=True, check=True)
hook('SessionStart')
hook('MessageDisplay', delta='\n'.join(f'linha-{i:02d}' for i in range(60)), final=True, index=0)
for line in sys.stdin:
    line = line.rstrip('\n')
    with open(os.environ['BEE_FAKE_STDIN'], 'a') as f:
        f.write(repr(line) + '\n')
    if line == '/exit':
        sys.exit(0)
    if not line.startswith('\x1b[200~') and not (os.environ.get('BEE_BRACKETED') and '\x1b[201~' not in line):
        hook('Stop')  # The turn ends once the whole message arrived.
'''


def editor(binary, project, state, env, size, bracketed):
    rows, cols = size
    master, slave, before, proc, output, until = spawn(binary, project, state, env, size=size)
    log = Path(env['BEE_FAKE_STDIN'])
    try:
        until(b'linha-59')
        os.write(master, b'\x1b[5~')  # PgUp reads older messages.
        until(b'anteriores')
        os.write(master, b'\x1b')  # Esc follows the end again.
        time.sleep(0.3)
        os.write(master, b'\x1b[200~linha um\nlinha dois\x1b[201~')
        time.sleep(0.5)
        assert not log.exists(), 'paste must not be sent before Enter'
        os.write(master, b'\x1b[H>')  # Home, then edit the second line.
        os.write(master, b'\x1b[F\x1b\r')  # End, then Alt+Enter adds a line.
        os.write(master, 'três'.encode())
        os.write(master, b'\r')
        until(b'Turno')  # Stop arrived: Ctrl+Q may type /exit at the idle prompt.
        os.write(master, b'\x11')
        code = drain_until_exit(master, proc, output, 10)
        assert code == 0, (code, log.exists() and log.read_text())
        assert termios.tcgetattr(slave) == before
        assert b'Ctrl+Q' in output, 'shortcuts stay visible at this width'
        received = [eval(line) for line in log.read_text().splitlines()]
        if bracketed:
            expected = ['\x1b[200~linha um', '>linha dois', 'três\x1b[201~', '/exit']
        else:
            expected = ['linha um >linha dois três', '/exit']
        assert received == expected, received
        log.unlink()
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)


def history_and_export(binary, project, state, env, resume):
    master, slave, before, proc, output, until = spawn(binary, project, state, env, resume)
    try:
        if resume:
            until(b'decrescente')  # Previous user message read back from the transcript.
        else:
            until(b'pronto')
            os.write(master, b'\x05')  # Ctrl+E opens the export preview.
            until(b'Destino')
            os.write(master, b'\x13')  # Ctrl+S writes and verifies.
            until(b'verificado')
        os.write(master, b'\x11')
        assert drain_until_exit(master, proc, output, 10) == 0
        assert termios.tcgetattr(slave) == before
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)


def drain_until_exit(master, proc, output, seconds):
    """Keep reading while waiting: a full PTY buffer (small on macOS) would
    block the Bee's final redraw and look like a hang."""
    deadline = time.monotonic() + seconds
    while proc.poll() is None:
        if time.monotonic() > deadline:
            raise subprocess.TimeoutExpired(proc.args, seconds)
        if select.select([master], [], [], 0.05)[0]:
            try:
                output.extend(os.read(master, 65536))
            except OSError:
                pass
    return proc.returncode


def spawn(binary, project, state, env, resume=False, size=(24, 80)):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', size[0], size[1], 0, 0))
    before = termios.tcgetattr(slave)
    argv = [str(binary), 'workspace', '--claude', '--project', str(project), '--state', str(state), '--no-color']
    if resume:
        argv.append('--resume')
    proc = subprocess.Popen(argv, stdin=slave, stdout=slave, stderr=slave, env=env)
    output = bytearray()

    def until(token, seconds=12):
        deadline = time.monotonic() + seconds
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

    return master, slave, before, proc, output, until


def untrusted(binary, project, state, env, quit_while_blocked):
    master, slave, before, proc, output, until = spawn(binary, project, state, env)
    try:
        until('confiança'.encode())
        assert b'trust' not in output, 'native dialog leaked into the Bee view'
        if quit_while_blocked:
            started = time.monotonic()
            os.write(master, b'\x11')
            drain_until_exit(master, proc, output, 4)
            assert time.monotonic() - started < 4
            assert proc.returncode == 4, proc.returncode
        else:
            os.write(master, b'\x0f')  # Ctrl+O opens the original screen.
            until(b'trust')
            os.write(master, b'1\r')
            until(b'encerra  ')  # Ready footer is shorter than the setup one.
            os.write(master, b'\x0f')
            until('concluída'.encode())
            os.write(master, b'\x11')  # Idle prompt: Ctrl+Q types /exit.
            drain_until_exit(master, proc, output, 10)
            assert proc.returncode == 0, proc.returncode
        assert termios.tcgetattr(slave) == before
        assert not (state / 'claude-native.lock').exists()
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)


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
        until(b'Permitir')
        os.write(master, decision.encode())
        until(b'sintetica')
        until(b'regras')  # Read ran without a request: Claude's rules, not the Bee.
        if decision == 'y':
            until(b'aprovada')
        try:
            drain_until_exit(master, proc, output, 20)
        except subprocess.TimeoutExpired as exc:
            raise AssertionError(f'Bee did not exit after {decision!r}; output={output[-800:]!r}') from exc
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
    with tempfile.TemporaryDirectory(prefix='bee-trust-') as temp:
        root = Path(temp)
        fake_dir = root / 'bin'
        fake_dir.mkdir()
        fake = fake_dir / 'claude'
        fake.write_text(FAKE_TRUST)
        fake.chmod(0o700)
        project = root / 'project'
        project.mkdir()
        stdin_log = root / 'stdin'
        env = dict(os.environ, PATH=f'{fake_dir}:/usr/bin:/bin', BEE_BINARY=str(binary), BEE_FAKE_STDIN=str(stdin_log))
        untrusted(binary, project, root / 'state-quit', env, True)
        assert not stdin_log.exists(), 'Ctrl+Q typed into the native dialog'
        untrusted(binary, project, root / 'state-setup', env, False)
        assert stdin_log.read_text().splitlines() == ['1', '/exit']
    with tempfile.TemporaryDirectory(prefix='bee-history-') as temp:
        root = Path(temp)
        fake_dir = root / 'bin'
        fake_dir.mkdir()
        fake = fake_dir / 'claude'
        fake.write_text(FAKE_HISTORY)
        fake.chmod(0o700)
        project = root / 'project'
        project.mkdir()
        transcripts = root / 'transcripts'
        transcripts.mkdir()
        state = root / 'state'
        stdin_log = root / 'stdin'
        fixture = Path(__file__).resolve().parent.parent / 'testdata/claude/basic.jsonl'
        env = dict(os.environ, PATH=f'{fake_dir}:/usr/bin:/bin', BEE_BINARY=str(binary), BEE_FAKE_STDIN=str(stdin_log), BEE_TRANSCRIPTS=str(transcripts), BEE_FIXTURE=str(fixture))
        history_and_export(binary, project, state, env, False)
        bundles = list((state / 'exports').iterdir())
        assert len(bundles) == 1, bundles
        verified = subprocess.run([str(binary), 'verify', str(bundles[0])], capture_output=True, text=True)
        assert verified.returncode == 0, verified.stdout + verified.stderr
        history_and_export(binary, project, state, env, True)
        # History is shown from the transcript, never typed back into Claude.
        assert stdin_log.read_text().splitlines() == ['/exit', '/exit']
    with tempfile.TemporaryDirectory(prefix='bee-editor-') as temp:
        root = Path(temp)
        fake_dir = root / 'bin'
        fake_dir.mkdir()
        fake = fake_dir / 'claude'
        fake.write_text(FAKE_EDITOR)
        fake.chmod(0o700)
        project = root / 'project'
        project.mkdir()
        env = dict(os.environ, PATH=f'{fake_dir}:/usr/bin:/bin', BEE_BINARY=str(binary), BEE_FAKE_STDIN=str(root / 'stdin'))
        editor(binary, project, root / 'state-a', dict(env, BEE_BRACKETED='1'), (24, 80), True)
        editor(binary, project, root / 'state-b', env, (12, 40), False)  # Documented minimum width.
    print('PASS: Bee-only UI, hidden original output, hooks, deny/allow, Ctrl+Q denial, resume, blocked native setup via Ctrl+O, safe Ctrl+Q, history rehydration, verified export, multiline editor, scrolling, 40-column controls and terminal restoration')


if __name__ == '__main__':
    main()
