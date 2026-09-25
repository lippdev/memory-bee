#!/usr/bin/env python3
"""Exercise the demo in a POSIX PTY; no agents, credentials or model calls."""
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


def main():
    repo = Path(__file__).resolve().parent.parent
    binary = str(repo / 'target/debug/memory-bee')
    with tempfile.TemporaryDirectory(prefix='memory-bee-workspace-pty-') as tmp:
        root = Path(tmp)
        project = root / 'project'
        project.mkdir()
        state = root / 'state'
        bundle = root / 'bundle'
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 20, 60, 0, 0))
        before = termios.tcgetattr(slave)
        proc = subprocess.Popen(
            [binary, 'workspace', '--demo', '--project', str(project), '--state', str(state), '--no-color'],
            stdin=slave, stdout=slave, stderr=slave,
            env=dict(os.environ, TERM='xterm-256color', PATH='/usr/bin:/bin'),
        )

        def drain(seconds=0.15):
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                if select.select([master], [], [], max(0, deadline-time.monotonic()))[0]:
                    data = os.read(master, 65536)
                    if b'\x1b[6n' in data:
                        os.write(master, b'\x1b[1;1R')

        def send(text):
            os.write(master, text.encode())
            drain()

        def paste(text):
            send('\x1b[200~' + text + '\x1b[201~')

        def snapshot():
            return json.loads((state / 'workspace.json').read_text())

        def wait_for(predicate):
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                if predicate():
                    return
                drain()
            raise AssertionError('Timed out waiting for workspace state')

        try:
            drain()
            wait_for(lambda: (state / 'workspace.json').exists())
            send('\r')  # Home -> conversation.
            paste('synthetic task\nsecond line')
            assert not snapshot()['sessions'][0]['events'], 'Paste submitted a message'
            send('\r')
            wait_for(lambda: snapshot()['sessions'][0]['events'][-1]['kind'] == 'approval')
            send('\t')
            send('\r')  # Default deny.
            wait_for(lambda: not snapshot()['sessions'][0]['running'])
            events = snapshot()['sessions'][0]['events']
            assert any(e['kind'] == 'decision' and not e['allow'] for e in events)
            paste('/export')
            send('\r')
            paste(str(bundle))
            send('\r')
            send('\r')  # No exclusions; review snapshot.
            assert not bundle.exists()
            paste('EXPORTAR')
            send('\r')
            wait_for(lambda: (bundle / 'manifest.json').exists())
            result = subprocess.run([binary, 'verify', str(bundle)], capture_output=True)
            assert result.returncode in (0, 2), result.stderr.decode()
            import jsonschema
            manifest = json.loads((bundle / 'manifest.json').read_text())
            jsonschema.validate(manifest, json.loads((repo / 'schemas/bundle-v1.schema.json').read_text()))
            assert manifest['source']['agent'] == 'memory-bee-demo'
            send('\x1b')
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 12, 40, 0, 0))
            paste('interrupted task')
            send('\r')
            send('\x03')
            wait_for(lambda: not snapshot()['sessions'][0]['running'])
            assert snapshot()['sessions'][0]['events'][-1]['kind'] == 'interrupted'
            send('\x11')
            proc.wait(timeout=5)
            assert proc.returncode == 0
            assert termios.tcgetattr(slave) == before, 'Terminal was not restored'
            assert not (state / 'workspace.lock').exists()
            assert list(project.iterdir()) == [], 'Demo modified project'
            print('PASS: paste, approval denial, confirmed export/schema/verify, resize, interruption, private state and terminal restoration')
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait()
            os.close(master)
            os.close(slave)


if __name__ == '__main__':
    main()
